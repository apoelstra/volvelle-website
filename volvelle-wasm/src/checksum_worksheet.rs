// Volvelle Website
// Written in 2022 by
//     Andrew Poelstra <apoelstra@wpsoftware.net>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Checksum Worksheet

use crate::error::Error;
use crate::fe::{self, Checksum, Fe};
use std::collections::VecDeque;
use wasm_bindgen::prelude::*;

/// Helper function to translate a cell ID into a row/cell index pair
fn cell_from_name(s: &str) -> Result<(usize, usize), Error> {
    use std::str::FromStr;
    if s.len() < 7 || &s.as_bytes()[0..4] != b"inp_" {
        return Err(Error::UnknownCell {
            id: s.into(),
            reason: "no inp_ prefix",
        });
    }
    let ret1;
    let ret2;
    let mut iter = s[4..].split('_');
    if let Some(n1) = iter.next() {
        if let Ok(n) = usize::from_str(n1) {
            ret1 = n;
        } else {
            return Err(Error::UnknownCell {
                id: s.into(),
                reason: "first number did not parse",
            });
        }
    } else {
        return Err(Error::UnknownCell {
            id: s.into(),
            reason: "no first number",
        });
    }
    if let Some(n2) = iter.next() {
        if let Ok(n) = usize::from_str(n2) {
            ret2 = n;
        } else {
            return Err(Error::UnknownCell {
                id: s.into(),
                reason: "second number did not parse",
            });
        }
    } else {
        return Err(Error::UnknownCell {
            id: s.into(),
            reason: "no second number",
        });
    }
    if iter.next().is_some() {
        return Err(Error::UnknownCell {
            id: s.into(),
            reason: "more than 2 numbers",
        });
    }
    Ok((ret1, ret2))
}

/// How to render a given cell
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CellType {
    /// Share data, which is editable in all modes and bolded
    ShareData,
    /// Residue line, implied by cells above it in all modes
    Residue,
    /// Sum, implied by cells above it in all modes
    Sum,
    /// Global `SECRETSHARE32` residue, fixed in "create" mode but computed in "verify" mode
    GlobalResidue,
}

impl CellType {
    fn text(&self, is_checksum: bool) -> &'static str {
        match *self {
            CellType::ShareData => {
                if is_checksum {
                    "share_data_checksum"
                } else {
                    "share_data"
                }
            }
            CellType::Residue => "residue",
            CellType::Sum => {
                if is_checksum {
                    "sum_checksum"
                } else {
                    "sum"
                }
            }
            CellType::GlobalResidue => "global_residue",
        }
    }
}

/// A single cell of the checksum worksheet
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    ty: CellType,
    /// Whether this cell is far enough to the right of the sheet to be a
    /// "checksum" cell, i.e. one that is backcomputed.
    is_checksum: bool,
    /// The ID of the corresponding input box on the page.
    dom_id: String,
    /// The value in this cell, unless it is blank
    val: Option<Fe>,
}

/// A row in the worksheet
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Row {
    /// The actual cell data
    cells: Vec<Cell>,
}

/// Whether the worksheet should be in "create share" or "verify share" mode
#[wasm_bindgen]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CreateMode {
    Create = 0,
    Verify = 1,
}

impl Checksum {
    /// Length of the generator polynomial
    fn len(&self) -> usize {
        match *self {
            Checksum::Codex32 => 13,
            Checksum::Bech32 => 6,
        }
    }
}

/// Action to perform on the DOM
#[wasm_bindgen]
pub struct Action {
    /// What to do
    ty: &'static str,
    /// and what cell to do it to
    id: String,
    /// Data for the action (e.g. to set a cell to a particular value)
    pub value: Option<char>,
}
#[wasm_bindgen]
impl Action {
    // Need to manually implement accessors for now with wasm_bindgen
    #[wasm_bindgen(getter)]
    pub fn ty(&self) -> String {
        self.ty.into()
    }
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

/// Cell to construct in the DOM
#[wasm_bindgen]
pub struct DomCell {
    ty: &'static str,
    dom_id: String,
    pub val: Option<char>,
    pub x: usize,
    pub y: usize,
}

#[wasm_bindgen]
impl DomCell {
    // Need to manually implement accessors for now with wasm_bindgen
    #[wasm_bindgen(getter)]
    pub fn ty(&self) -> String {
        self.ty.into()
    }
    #[wasm_bindgen(getter)]
    pub fn dom_id(&self) -> String {
        self.dom_id.clone()
    }
}

/// The entire checksum worksheet
#[wasm_bindgen]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Worksheet {
    hrp: String,
    create_mode: CreateMode,
    size: usize,
    rows: Vec<Row>,
    checksum: Checksum,
}

impl Worksheet {
    /// Constructs a new blank worksheet
    pub fn new(
        hrp: &str,
        create_mode: CreateMode,
        size: usize,
        checksum: Checksum,
    ) -> Result<Worksheet, Error> {
        let mut ret = Worksheet {
            hrp: hrp.to_string().to_ascii_uppercase(),
            create_mode,
            size,
            rows: vec![],
            checksum,
        };

        if size < ret.hrp.len() + checksum.len() {
            return Err(Error::TooShort {
                minimum: ret.hrp.len() + checksum.len(),
                actual: size,
            });
        }
        let data_len = size - ret.hrp.len() - checksum.len() - 1;
        if data_len % 2 == 1 {
            return Err(Error::OddLength { data_len });
        }

        // Number of non-global-residue pairs of rows
        let n_rows = data_len / 2 + 1;

        // Treat first two rows specially
        ret.add_first_row();
        ret.add_second_row();
        // Then do the remaining rows except the global SECRETSHARE32 residue
        for i in 0..n_rows - 1 {
            ret.add_2nth_rows(2 * i);
        }
        // Finally stick the global residue row on
        ret.add_final_row();

        Ok(ret)
    }

    /// Helper to populate the first row (checksum_len many data chars)
    fn add_first_row(&mut self) {
        assert_eq!(self.rows.len(), 0);
        self.rows.push(Row { cells: vec![] });
        for n in 0..self.checksum.len() {
            self.rows[0].cells.push(Cell {
                ty: CellType::ShareData,
                is_checksum: self.hrp.len() + 1 + n >= self.size - self.checksum.len(),
                dom_id: format!("inp_{}_{}", 0, n),
                val: None,
            });
        }
    }

    /// Helper to populate the second row (HRP residue) (does not change offset)
    fn add_second_row(&mut self) {
        assert_eq!(self.rows.len(), 1);
        self.rows.push(Row { cells: vec![] });
        let hrp_poly = match self.checksum {
            Checksum::Bech32 => fe::Poly::bech32_hrp_residue(&self.hrp),
            Checksum::Codex32 => fe::Poly::codex32_hrp_residue(&self.hrp),
        };
        for (n, fe) in hrp_poly.iter().enumerate() {
            self.rows[1].cells.push(Cell {
                ty: CellType::Residue,
                is_checksum: self.hrp.len() + 1 + n >= self.size - self.checksum.len(),
                dom_id: format!("inp_{}_{}", 1, n),
                val: Some(fe),
            });
        }
    }

    /// Helper to populate the nth and (n+1)th "ordinary" rows (sum then residue)
    fn add_2nth_rows(&mut self, offset: usize) {
        let ridx = self.rows.len();

        self.rows.push(Row { cells: vec![] });
        let row = self.rows.last_mut().unwrap();
        for n in 0..self.checksum.len() {
            row.cells.push(Cell {
                ty: CellType::Sum,
                is_checksum: self.hrp.len() + 1 + offset + n >= self.size - self.checksum.len(),
                dom_id: format!("inp_{}_{}", ridx, n),
                val: None,
            });
        }
        for n in 0..2 {
            row.cells.push(Cell {
                ty: CellType::ShareData,
                is_checksum: self.hrp.len() + 1 + offset + self.checksum.len() + n
                    >= self.size - self.checksum.len(),
                dom_id: format!("inp_{}_{}", ridx, self.checksum.len() + n),
                val: None,
            });
        }

        self.rows.push(Row { cells: vec![] });
        let row = self.rows.last_mut().unwrap();
        for n in 0..self.checksum.len() {
            row.cells.push(Cell {
                ty: CellType::Residue,
                is_checksum: self.hrp.len() + 1 + offset + self.checksum.len() + n
                    >= self.size - self.checksum.len(),
                dom_id: format!("inp_{}_{}", ridx + 1, n),
                val: None,
            });
        }
    }

    fn add_final_row(&mut self) {
        let ridx = self.rows.len();

        let checksum_str = match self.checksum {
            Checksum::Codex32 => "SECRETSHARE32",
            Checksum::Bech32 => "QQQQQP",
        };
        self.rows.push(Row { cells: vec![] });
        let row = self.rows.last_mut().unwrap();
        for (n, ch) in checksum_str.chars().enumerate() {
            row.cells.push(Cell {
                ty: CellType::GlobalResidue,
                is_checksum: true,
                dom_id: format!("inp_{}_{}", ridx, n),
                val: Some(Fe::try_from(ch).unwrap()),
            });
        }
    }

    /// Constructs a giant array of cells with information to populate the DOM with
    pub fn get_dom_cells(&self) -> Result<Vec<DomCell>, JsError> {
        let mut ret = vec![]; // FIXME compute length
        if self.rows.is_empty() {
            return Ok(ret);
        }

        let mut offset = self.hrp.len();
        for (ny, row) in self.rows.iter().enumerate() {
            if ny == 0 {
                for (n, ch) in self.hrp.chars().enumerate() {
                    ret.push(DomCell {
                        ty: "fixed_hrp",
                        dom_id: format!("cell_hrp_{}", n),
                        val: Some(ch),
                        x: n,
                        y: ny,
                    });
                }
                ret.push(DomCell {
                    ty: "fixed_hrp",
                    dom_id: format!("cell_hrp_{}", self.hrp.len()),
                    val: Some('1'),
                    x: self.hrp.len(),
                    y: ny,
                });
            } else {
                if ny > 2 && ny % 2 == 1 {
                    offset += 2;
                }
                ret.push(DomCell {
                    ty: "symbol",
                    dom_id: format!("cell_symb_{}", ny),
                    val: Some(if ny % 2 == 0 { '=' } else { '+' }),
                    x: offset,
                    y: ny,
                });
            }
            for (nx, cell) in row.cells.iter().enumerate() {
                ret.push(DomCell {
                    ty: cell.ty.text(cell.is_checksum),
                    dom_id: cell.dom_id.clone(),
                    val: cell.val.map(|fe| fe.into()),
                    x: offset + 1 + nx,
                    y: ny,
                });
            }
        }
        Ok(ret)
    }

    /// Helper function to offset a ridx/cidx pair
    fn cell_below(&self, ridx: usize, cidx: usize) -> Option<(usize, usize)> {
        // No more rows
        if ridx >= self.rows.len() - 1 {
            return None;
        }
        // No more rows in this column
        let offset_adj = if ridx > 0 && ridx % 2 == 0 { 2 } else { 0 };
        if offset_adj > cidx {
            return None;
        }
        Some((ridx + 1, cidx - offset_adj))
    }

    /// Helper function to offset a ridx/cidx pair
    fn cell_above(&self, ridx: usize, cidx: usize) -> Option<(usize, usize)> {
        // No more rows
        if ridx == 0 {
            return None;
        }
        // No more rows in this column
        let offset_adj = 2 * (ridx % 2);
        if cidx + offset_adj >= self.rows[ridx - 1].cells.len() {
            return None;
        }
        Some((ridx - 1, cidx + offset_adj))
    }

    /// Handle a user-initiated change in one of the cells
    pub fn handle_input_change(&mut self, id: &str, val: &str) -> Result<Vec<Action>, Error> {
        let cell = cell_from_name(id)?;
        if cell.0 >= self.rows.len() {
            return Err(Error::InvalidRow {
                row: cell.0,
                n_rows: self.rows.len(),
            });
        }
        if cell.1 >= self.rows[cell.0].cells.len() {
            return Err(Error::InvalidCell {
                cell: cell.1,
                row: cell.0,
                n_cells: self.rows[0].cells.len(),
            });
        }

        match val.len() {
            0 => {
                self.rows[cell.0].cells[cell.1].val = None;
                Ok(vec![])
            }
            1 => {
                if !val.is_ascii() {
                    return Ok(vec![Action {
                        ty: "flash_error",
                        id: id.into(),
                        value: None,
                    }]);
                }
                let ch = val.chars().next().unwrap();
                let ch_u = ch.to_ascii_uppercase();
                let fe = match Fe::try_from(ch_u) {
                    Ok(fe) => fe,
                    Err(_) => {
                        return Ok(vec![Action {
                            ty: "flash_error",
                            id: id.into(),
                            value: None,
                        }]);
                    }
                };

                self.rows[cell.0].cells[cell.1].val = Some(fe);
                let mut ret = vec![];
                if ch != ch_u {
                    ret.push(Action {
                        ty: "flash_set",
                        id: id.into(),
                        value: Some(ch_u),
                    });
                };

                // Actually update the sheet
                let mut queue = VecDeque::with_capacity(2 * self.checksum.len());
                queue.push_back((cell.0, cell.1));

                macro_rules! unwrap_or_continue {
                    ($e:expr) => {
                        if let Some(e) = $e {
                            e
                        } else {
                            continue;
                        }
                    };
                }

                while let Some((ridx, cidx)) = queue.pop_front() {
                    let cell = &self.rows[ridx].cells[cidx];
                    // First, if the cell we're updating is blank, just skip it. Otherwise extract its value.
                    let fe = match cell.val {
                        Some(fe) => fe,
                        None => continue,
                    };

                    match (cell.ty, cell.is_checksum) {
                        (CellType::Sum, _) if cidx == 0 || cidx == 1 => {
                            // For lower diagonal cells, try to compute a residue:
                            assert!(self.rows[ridx].cells.len() > 1);
                            assert!(cidx == 0 || cidx == 1); // check that one of the two lines below grabs our target
                            let fe1 = self.rows[ridx].cells[0].val;
                            let fe2 = self.rows[ridx].cells[1].val;
                            if let (Some(fe1), Some(fe2)) = (fe1, fe2) {
                                // compute residue...
                                let mut poly: fe::Poly = fe1.into();
                                poly.mul_by_x_then_add(fe2);
                                poly.mul_by_x(self.checksum.len());
                                assert!(self.rows[ridx + 1].cells.len() >= self.checksum.len());
                                let residue = match self.checksum {
                                    Checksum::Codex32 => poly.codex32_polymod(),
                                    Checksum::Bech32 => poly.bech32_polymod(),
                                };
                                // ...then put it into the next line's cells
                                for (n, fe) in residue.iter().enumerate() {
                                    if self.rows[ridx + 1].cells[n].val == Some(fe) {
                                        continue; // don't update if the cell is already set
                                    }
                                    self.rows[ridx + 1].cells[n].val = Some(fe);
                                    ret.push(Action {
                                        ty: "set",
                                        id: self.rows[ridx + 1].cells[n].dom_id.clone(),
                                        value: Some(fe.into()),
                                    });
                                    queue.push_back((ridx + 1, n));
                                }
                            }
                        }
                        (CellType::Sum, false) | (CellType::ShareData, false) => {
                            // For sum cells, we try to add to the cell below
                            let below = unwrap_or_continue!(self.cell_below(ridx, cidx));
                            let fe2 = unwrap_or_continue!(self.rows[below.0].cells[below.1].val);
                            let below2 = unwrap_or_continue!(self.cell_below(below.0, below.1));
                            // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                            let fe3 = fe + fe2;
                            if self.rows[below2.0].cells[below2.1].val == Some(fe3) {
                                continue; // don't update if the cell is already set
                            }
                            self.rows[below2.0].cells[below2.1].val = Some(fe3);
                            ret.push(Action {
                                ty: "set",
                                id: self.rows[below2.0].cells[below2.1].dom_id.clone(),
                                value: Some(fe3.into()),
                            });
                            queue.push_front((below2.0, below2.1));
                        }
                        (CellType::Sum, true) | (CellType::ShareData, true) => {
                            // For sum cells, we try to add to the cell above
                            let above = unwrap_or_continue!(self.cell_above(ridx, cidx));
                            let fe2 = unwrap_or_continue!(self.rows[above.0].cells[above.1].val);
                            let above2 = unwrap_or_continue!(self.cell_above(above.0, above.1));
                            // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                            let fe3 = fe + fe2;
                            if self.rows[above2.0].cells[above2.1].val == Some(fe3) {
                                continue; // don't update if the cell is already set
                            }
                            self.rows[above2.0].cells[above2.1].val = Some(fe3);
                            ret.push(Action {
                                ty: "set",
                                id: self.rows[above2.0].cells[above2.1].dom_id.clone(),
                                value: Some(fe3.into()),
                            });
                            queue.push_front((above2.0, above2.1));
                        }
                        (CellType::Residue, false) => {
                            // Residue cells are very similar to sum cells
                            let above = unwrap_or_continue!(self.cell_above(ridx, cidx));
                            let fe2 = unwrap_or_continue!(self.rows[above.0].cells[above.1].val);
                            let below = unwrap_or_continue!(self.cell_below(ridx, cidx));
                            // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                            let fe3 = fe + fe2;
                            if self.rows[below.0].cells[below.1].val == Some(fe3) {
                                continue; // don't update if the cell is already set
                            }
                            self.rows[below.0].cells[below.1].val = Some(fe3);
                            ret.push(Action {
                                ty: "set",
                                id: self.rows[below.0].cells[below.1].dom_id.clone(),
                                value: Some(fe3.into()),
                            });
                            queue.push_front((below.0, below.1));
                        }
                        (CellType::Residue, true) => {
                            // Residue cells are very similar to sum cells
                            let below = unwrap_or_continue!(self.cell_below(ridx, cidx));
                            let fe2 = unwrap_or_continue!(self.rows[below.0].cells[below.1].val);
                            let above = unwrap_or_continue!(self.cell_above(ridx, cidx));
                            // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                            let fe3 = fe + fe2;
                            if self.rows[above.0].cells[above.1].val == Some(fe3) {
                                continue; // don't update if the cell is already set
                            }
                            self.rows[above.0].cells[above.1].val = Some(fe3);
                            ret.push(Action {
                                ty: "set",
                                id: self.rows[above.0].cells[above.1].dom_id.clone(),
                                value: Some(fe3.into()),
                            });
                            queue.push_back((above.0, above.1));
                        }
                        (CellType::GlobalResidue, _) => unreachable!(),
                    }
                }
                Ok(ret)
            }
            _ => Ok(vec![Action {
                ty: "flash_error",
                id: id.into(),
                value: None,
            }]),
        }
    }

    /// Returns the first six characters of the share, with `_`s for missign characters
    /// characters of the share (the header)
    pub fn header_str(&self) -> String {
        if self.rows.is_empty() {
            "______".into()
        } else {
            let mut ret = String::with_capacity(6);
            let iter = self.rows[0]
                .cells
                .iter()
                .skip(self.hrp.len())
                .map(|cell| cell.val)
                .take(6);
            for fe in iter {
                ret.push(fe.map(From::from).unwrap_or('_'));
            }
            while ret.len() < 6 {
                ret.push('_');
            }
            ret
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_input_id() {
        assert_eq!(cell_from_name("inp_0_0"), Ok((0, 0)));
        assert_eq!(cell_from_name("inp_10_0"), Ok((10, 0)));
        assert_eq!(cell_from_name("inp_10_10"), Ok((10, 10)));
        assert!(cell_from_name("inp_10_10_10").is_err());
        assert!(cell_from_name("inP_10_10").is_err());
        assert!(cell_from_name("inp_10").is_err());
        assert!(cell_from_name("inp__").is_err());
    }

    #[test]
    fn user_test() {
        let mut worksheet =
            Worksheet::new("ms", CreateMode::Create, 48, Checksum::Codex32).unwrap();
        assert!(worksheet.handle_input_change("inp_0_0", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_1", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_2", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_3", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_4", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_5", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_6", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_7", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_8", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_9", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_10", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_11", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_0_12", "c").is_ok());

        assert!(worksheet.handle_input_change("inp_2_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_2_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_4_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_4_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_6_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_6_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_8_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_8_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_10_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_10_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_12_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_12_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_14_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_14_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_16_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_16_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_18_13", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_18_14", "c").is_ok());
        assert!(worksheet.handle_input_change("inp_20_13", "c").is_ok());

        assert_eq!(worksheet.cell_below(2, 15), Some((3, 13)));
        assert_eq!(worksheet.cell_below(3, 13), Some((4, 13)));
    }
}
