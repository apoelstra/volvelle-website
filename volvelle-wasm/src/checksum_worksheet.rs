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

use crate::dom_action;
use crate::error::Error;
use crate::fe;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ops;

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
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum CellType {
    /// A + or = symbol
    #[serde(rename = "symbol")]
    Symbol,
    /// The HRP, which is fixed in all modes
    #[serde(rename = "fixed_hrp")]
    FixedHrp,
    /// The HRP's residue, which is fixed in all modes
    #[serde(rename = "fixed_residue")]
    FixedResidue,
    /// Share data, which is editable in all modes and bolded
    #[serde(rename = "share_data")]
    ShareData,
    /// Share checksum, which is computed in "create" mode but entered in "verify" mode
    #[serde(rename = "share_data_checksum")]
    ShareDataChecksum,
    /// Residue line, implied by cells above it in all modes
    #[serde(rename = "residue")]
    Residue,
    /// Sum, implied by cells above it in all modes
    #[serde(rename = "sum")]
    Sum,
    /// Sum, implied by cells below it in "create" mode, by cells above it in "verify" mode
    #[serde(rename = "sum_checksum")]
    SumChecksum,
    /// Global `SECRETSHARE32` residue, fixed in "create" mode but computed in "verify" mode
    #[serde(rename = "global_residue")]
    GlobalResidue,
}

/// A single cell of the checksum worksheet
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Cell {
    ty: CellType,
    dom_id: String,
    val: Option<char>,
}

/// A row in the worksheet
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Row {
    /// Offset of the cell when rendered on the sheet
    start_idx: usize,
    /// The actual cell data
    cells: Vec<Cell>,
}

/// Whether the worksheet should be in "create share" or "verify share" mode
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum CreateMode {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "verify")]
    Verify,
}

/// The checksum to use for this worksheet
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum Checksum {
    #[serde(rename = "codex32")]
    Codex32,
    #[serde(rename = "bech32")]
    Bech32,
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

/// The entire checksum worksheet
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Worksheet {
    hrp: String,
    create_mode: CreateMode,
    size: usize,
    rows: Vec<Row>,
    checksum: Checksum,
}

impl ops::Index<(usize, usize)> for Worksheet {
    type Output = Cell;
    fn index(&self, idx: (usize, usize)) -> &Cell {
        if idx.0 >= self.rows.len() {
            panic!("Row {} out of bounds (max {})", idx.0, self.rows.len());
        }

        let offs = self.rows[idx.0].start_idx;
        if offs > idx.1 || idx.1 - offs >= self.rows[idx.0].cells.len() {
            panic!(
                "Cell {} out of bounds (row {} runs from {}-{})",
                idx.1,
                idx.0,
                offs,
                offs + self.rows[idx.0].cells.len()
            );
        }
        &self.rows[idx.0].cells[idx.1 - offs]
    }
}
impl ops::IndexMut<(usize, usize)> for Worksheet {
    fn index_mut(&mut self, idx: (usize, usize)) -> &mut Cell {
        if idx.0 >= self.rows.len() {
            panic!("Row {} out of bounds (max {})", idx.0, self.rows.len());
        }

        let offs = self.rows[idx.0].start_idx;
        if offs > idx.1 || idx.1 - offs >= self.rows[idx.0].cells.len() {
            panic!(
                "Cell {} out of bounds (row {} runs from {}-{})",
                idx.1,
                idx.0,
                offs,
                offs + self.rows[idx.0].cells.len()
            );
        }
        &mut self.rows[idx.0].cells[idx.1 - offs]
    }
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
        let mut offset = ret.add_first_row();
        ret.add_second_row(offset);

        // For the remaining rows except the global SECRETSHARE32 residue
        for _ in 1..n_rows {
            offset = ret.add_2nth_rows(offset);
        }

        // Finally stick the global residue row on
        ret.add_final_row(offset);

        Ok(ret)
    }

    /// Helper to populate the first row (HRP + checksum_len many data chars)
    fn add_first_row(&mut self) -> usize {
        assert_eq!(self.rows.len(), 0);
        self.rows.push(Row {
            start_idx: 0,
            cells: vec![],
        });
        for (n, ch) in self.hrp.chars().enumerate() {
            self.rows[0].cells.push(Cell {
                ty: CellType::FixedHrp,
                dom_id: format!("inp_{}_{}", 0, n),
                val: Some(ch),
            });
        }
        self.rows[0].cells.push(Cell {
            ty: CellType::FixedHrp,
            dom_id: format!("inp_{}_{}", 0, self.hrp.len()),
            val: Some('1'),
        });
        let offset = self.hrp.len() + 1;
        for n in 0..self.checksum.len() {
            self.rows[0].cells.push(Cell {
                ty: if offset + n >= self.size - self.checksum.len() {
                    CellType::ShareDataChecksum
                } else {
                    CellType::ShareData
                },
                dom_id: format!("inp_{}_{}", 0, self.hrp.len() + 1 + n),
                val: None,
            });
        }
        offset
    }

    /// Helper to populate the second row (HRP residue) (does not change offset)
    fn add_second_row(&mut self, offset: usize) {
        assert_eq!(self.rows.len(), 1);
        self.rows.push(Row {
            start_idx: offset - 1,
            cells: vec![Cell {
                ty: CellType::Symbol,
                dom_id: format!("inp_{}_symb", 1),
                val: Some('+'),
            }],
        });
        let hrp_poly = match self.checksum {
            Checksum::Bech32 => fe::FePoly::bech32_hrp_residue(&self.hrp),
            Checksum::Codex32 => fe::FePoly::codex32_hrp_residue(&self.hrp),
        };
        for (n, fe) in hrp_poly.iter().enumerate() {
            self.rows[1].cells.push(Cell {
                ty: CellType::FixedResidue,
                dom_id: format!("inp_{}_{}", 1, n),
                val: Some(fe.into()),
            });
        }
    }

    /// Helper to populate the nth and (n+1)th "ordinary" rows (sum then residue)
    fn add_2nth_rows(&mut self, mut offset: usize) -> usize {
        let mut ridx = self.rows.len();

        self.rows.push(Row {
            start_idx: offset - 1,
            cells: vec![Cell {
                ty: CellType::Symbol,
                dom_id: format!("inp_{}_symb", ridx),
                val: Some('='),
            }],
        });
        let row = self.rows.last_mut().unwrap();
        for n in 0..self.checksum.len() {
            row.cells.push(Cell {
                ty: if offset + n >= self.size - self.checksum.len() {
                    CellType::SumChecksum
                } else {
                    CellType::Sum
                },
                dom_id: format!("inp_{}_{}", ridx, 1 + n),
                val: None,
            });
        }
        for n in 0..2 {
            row.cells.push(Cell {
                ty: if offset + self.checksum.len() + n >= self.size - self.checksum.len() {
                    CellType::ShareDataChecksum
                } else {
                    CellType::ShareData
                },
                dom_id: format!("inp_{}_{}", ridx, 1 + n + self.checksum.len()),
                val: None,
            });
        }
        offset += 2;
        ridx += 1;

        self.rows.push(Row {
            start_idx: offset - 1,
            cells: vec![Cell {
                ty: CellType::Symbol,
                dom_id: format!("inp_{}_symb", ridx + 1),
                val: Some('+'),
            }],
        });
        let row = self.rows.last_mut().unwrap();
        for n in 0..self.checksum.len() {
            row.cells.push(Cell {
                ty: CellType::Residue,
                dom_id: format!("inp_{}_{}", ridx, 1 + n),
                val: None,
            });
        }
        offset
    }

    fn add_final_row(&mut self, offset: usize) {
        let ridx = self.rows.len();

        self.rows.push(Row {
            start_idx: offset - 1,
            cells: vec![Cell {
                ty: CellType::Symbol,
                dom_id: format!("inp_{}_symb", ridx),
                val: Some('='),
            }],
        });

        let checksum_str = match self.checksum {
            Checksum::Codex32 => "SECRETSHARE32",
            Checksum::Bech32 => "QQQQQP",
        };
        let row = self.rows.last_mut().unwrap();
        for (n, ch) in checksum_str.chars().enumerate() {
            row.cells.push(Cell {
                ty: CellType::GlobalResidue,
                dom_id: format!("inp_{}_{}", ridx, n),
                val: Some(ch),
            });
        }
    }

    /// Handle a user-initiated change in one of the cells
    pub fn handle_input_change(
        &mut self,
        id: &str,
        val: &str,
    ) -> Result<Vec<dom_action::Action>, Error> {
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
                    return Ok(vec![dom_action::Action {
                        ty: dom_action::ActionType::FlashError,
                        id: id.into(),
                        value: None,
                    }]);
                }
                let ch = val.chars().next().unwrap();
                let ch_u = ch.to_ascii_uppercase();
                if fe::Fe::try_from(ch_u).is_err() {
                    return Ok(vec![dom_action::Action {
                        ty: dom_action::ActionType::FlashError,
                        id: id.into(),
                        value: None,
                    }]);
                }

                self.rows[cell.0].cells[cell.1].val = Some(ch_u);
                let mut ret = if ch == ch_u {
                    vec![]
                } else {
                    vec![dom_action::Action {
                        ty: dom_action::ActionType::FlashSet,
                        id: id.into(),
                        value: Some(ch_u),
                    }]
                };
                ret.extend(self.update_sheet(cell.0, cell.1)?);
                Ok(ret)
            }
            _ => Ok(vec![dom_action::Action {
                ty: dom_action::ActionType::FlashError,
                id: id.into(),
                value: None,
            }]),
        }
    }

    /// Helper function to offset a ridx/cidx pair
    fn cell_below(&self, ridx: usize, cidx: usize) -> Option<(usize, usize)> {
        // No more rows
        if ridx >= self.rows.len() - 1 {
            return None;
        }
        // No more rows in this column
        let offset_adj = self.rows[ridx + 1].start_idx - self.rows[ridx].start_idx;
        if offset_adj > cidx {
            return None;
        }
        // A cell exists but it's just the +/= one
        if self.rows[ridx + 1].cells[cidx - offset_adj].ty == CellType::Symbol {
            return None;
        }
        return Some((ridx + 1, cidx - offset_adj));
    }

    /// Helper function to offset a ridx/cidx pair
    fn cell_above(&self, ridx: usize, cidx: usize) -> Option<(usize, usize)> {
        // No more rows
        if ridx == 0 {
            return None;
        }
        // No more rows in this column
        let offset_adj = self.rows[ridx].start_idx - self.rows[ridx - 1].start_idx;
        if cidx + offset_adj >= self.rows[ridx - 1].cells.len() {
            return None;
        }
        return Some((ridx - 1, cidx + offset_adj));
    }

    /// Helper which _actually_ reacts to a change
    fn update_sheet(&mut self, ridx: usize, cidx: usize) -> Result<Vec<dom_action::Action>, Error> {
        //println!("*** ENTER update_sheet({}, {})", ridx, cidx);
        #[derive(Debug)]
        enum CellPos {
            /// A "sum" cell which is either input data or the sum of two cells
            /// If it is updated, we should try to add it to the cell below it,
            /// and put this in the cell below that.
            Sum,
            /// A pink "sum" square; add the value to the cell above, and store
            /// the result in the cell above that
            SumChecksum,
            /// A "lower diagonal" cell for which we should look up a residue (if
            /// we can) and then update the next row based on that
            LowerDiagonal,
            /// A "residue" cell; similar to a sum cell but we add to the cell
            /// *above* it to update the one *below* it
            Residue,
            /// A "residue" cell in the checksum end of the worksheet; if this is
            /// updated we should add the value to the value *below* it, and update
            /// the cell *above* it.
            ResidueChecksum,
        }

        /// Infer from coordinates what kind of cell we're looking at
        fn cell_pos(ridx: usize, cidx: usize, worksheet: &Worksheet) -> CellPos {
            if ridx % 2 == 1 {
                // Residue cell, which are on every other line
                if worksheet.rows[ridx].start_idx + cidx
                    >= worksheet.size - worksheet.checksum.len()
                {
                    CellPos::ResidueChecksum
                } else {
                    CellPos::Residue
                }
            } else if cidx == 1 || cidx == 2 {
                // The first two squares of a "sum" line are lower-diagonal
                CellPos::LowerDiagonal
            } else {
                // The rest we classify as "sum" for now
                if worksheet.rows[ridx].start_idx + cidx
                    >= worksheet.size - worksheet.checksum.len()
                {
                    CellPos::SumChecksum
                } else {
                    CellPos::Sum
                }
            }
        }

        let mut queue = VecDeque::with_capacity(2 * self.checksum.len());
        queue.push_back((ridx, cidx));

        macro_rules! unwrap_or_continue {
            ($e:expr) => {
                if let Some(e) = $e {
                    e
                } else {
                    continue;
                }
            };
        }

        let mut ret = vec![];
        while let Some((ridx, cidx)) = queue.pop_front() {
            // First, if the cell we're updating is blank, just skip it. Otherwise extract its value.
            let fe = match self.rows[ridx].cells[cidx]
                .val
                .map(|ch| fe::Fe::try_from(ch).unwrap())
            {
                Some(fe) => fe,
                None => continue,
            };
            //println!("Looping on {},{} which is {:?} type {:?}", ridx, cidx, self.rows[ridx].cells[cidx].val, cell_pos(ridx, cidx));

            match cell_pos(ridx, cidx, self) {
                CellPos::LowerDiagonal => {
                    // For lower diagonal cells, try to compute a residue:
                    assert!(self.rows[ridx].cells.len() > 1);
                    assert!(cidx == 1 || cidx == 2); // check that one of the two lines below grabs our target
                    let fe1 = self.rows[ridx].cells[1]
                        .val
                        .map(|ch| fe::Fe::try_from(ch).unwrap());
                    let fe2 = self.rows[ridx].cells[2]
                        .val
                        .map(|ch| fe::Fe::try_from(ch).unwrap());
                    if let (Some(fe1), Some(fe2)) = (fe1, fe2) {
                        // compute residue...
                        let mut poly: fe::FePoly = fe1.into();
                        poly.mul_by_x_then_add(fe2);
                        poly.mul_by_x(self.checksum.len());
                        assert!(self.rows[ridx + 1].cells.len() >= self.checksum.len() + 1);
                        let residue = match self.checksum {
                            Checksum::Codex32 => poly.codex32_polymod(),
                            Checksum::Bech32 => poly.bech32_polymod(),
                        };
                        // ...then put it into the next line's cells
                        for (n, fe) in residue.iter().enumerate() {
                            if self.rows[ridx + 1].cells[1 + n].val == Some(fe.into()) {
                                continue; // don't update if the cell is already set
                            }
                            self.rows[ridx + 1].cells[1 + n].val = Some(fe.into());
                            ret.push(dom_action::Action {
                                ty: dom_action::ActionType::Set,
                                id: self.rows[ridx + 1].cells[1 + n].dom_id.clone(),
                                value: Some(fe.into()),
                            });
                            queue.push_back((ridx + 1, 1 + n));
                        }
                    }
                }
                CellPos::Sum => {
                    // For sum cells, we try to add to the cell below
                    let below = unwrap_or_continue!(self.cell_below(ridx, cidx));
                    let fe2 = unwrap_or_continue!(self.rows[below.0].cells[below.1]
                        .val
                        .map(|ch| fe::Fe::try_from(ch).unwrap()));
                    let below2 = unwrap_or_continue!(self.cell_below(below.0, below.1));
                    // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                    let fe3 = fe + fe2;
                    if self.rows[below2.0].cells[below2.1].val == Some(fe3.into()) {
                        continue; // don't update if the cell is already set
                    }
                    self.rows[below2.0].cells[below2.1].val = Some(fe3.into());
                    ret.push(dom_action::Action {
                        ty: dom_action::ActionType::Set,
                        id: self.rows[below2.0].cells[below2.1].dom_id.clone(),
                        value: Some(fe3.into()),
                    });
                    queue.push_front((below2.0, below2.1));
                }
                CellPos::SumChecksum => {
                    // For sum cells, we try to add to the cell above
                    let above = unwrap_or_continue!(self.cell_above(ridx, cidx));
                    let fe2 = unwrap_or_continue!(self.rows[above.0].cells[above.1]
                        .val
                        .map(|ch| fe::Fe::try_from(ch).unwrap()));
                    let above2 = unwrap_or_continue!(self.cell_above(above.0, above.1));
                    // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                    let fe3 = fe + fe2;
                    if self.rows[above2.0].cells[above2.1].val == Some(fe3.into()) {
                        continue; // don't update if the cell is already set
                    }
                    self.rows[above2.0].cells[above2.1].val = Some(fe3.into());
                    ret.push(dom_action::Action {
                        ty: dom_action::ActionType::Set,
                        id: self.rows[above2.0].cells[above2.1].dom_id.clone(),
                        value: Some(fe3.into()),
                    });
                    queue.push_front((above2.0, above2.1));
                }
                CellPos::Residue => {
                    // Residue cells are very similar to sum cells
                    let above = unwrap_or_continue!(self.cell_above(ridx, cidx));
                    let fe2 = unwrap_or_continue!(self.rows[above.0].cells[above.1]
                        .val
                        .map(|ch| fe::Fe::try_from(ch).unwrap()));
                    let below = unwrap_or_continue!(self.cell_below(ridx, cidx));
                    // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                    let fe3 = fe + fe2;
                    //println!("Update residue cell {},{} (val {}); add to {} to get {}", ridx, cidx, fe, fe2, fe3);
                    if self.rows[below.0].cells[below.1].val == Some(fe3.into()) {
                        continue; // don't update if the cell is already set
                    }
                    self.rows[below.0].cells[below.1].val = Some(fe3.into());
                    ret.push(dom_action::Action {
                        ty: dom_action::ActionType::Set,
                        id: self.rows[below.0].cells[below.1].dom_id.clone(),
                        value: Some(fe3.into()),
                    });
                    queue.push_front((below.0, below.1));
                }
                CellPos::ResidueChecksum => {
                    // Residue cells are very similar to sum cells
                    let below = unwrap_or_continue!(self.cell_below(ridx, cidx));
                    let fe2 = unwrap_or_continue!(self.rows[below.0].cells[below.1]
                        .val
                        .map(|ch| fe::Fe::try_from(ch).unwrap()));
                    let above = unwrap_or_continue!(self.cell_above(ridx, cidx));
                    // Update the sum, return to the JS an instruction to update the cell, and add it to the queue
                    let fe3 = fe + fe2;
                    if self.rows[above.0].cells[above.1].val == Some(fe3.into()) {
                        continue; // don't update if the cell is already set
                    }
                    println!(
                        "Update residue cell {},{} set to {} + {} = {}",
                        above.0, above.1, fe, fe2, fe3
                    );
                    self.rows[above.0].cells[above.1].val = Some(fe3.into());
                    ret.push(dom_action::Action {
                        ty: dom_action::ActionType::Set,
                        id: self.rows[above.0].cells[above.1].dom_id.clone(),
                        value: Some(fe3.into()),
                    });
                    queue.push_back((above.0, above.1));
                }
            }
        }
        Ok(ret)
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
        worksheet.handle_input_change("inp_0_3", "c").unwrap();
        worksheet.handle_input_change("inp_0_4", "c").unwrap();
        worksheet.handle_input_change("inp_0_5", "c").unwrap();
        worksheet.handle_input_change("inp_0_6", "c").unwrap();
        worksheet.handle_input_change("inp_0_7", "c").unwrap();
        worksheet.handle_input_change("inp_0_8", "c").unwrap();
        worksheet.handle_input_change("inp_0_9", "c").unwrap();
        worksheet.handle_input_change("inp_0_10", "c").unwrap();
        worksheet.handle_input_change("inp_0_11", "c").unwrap();
        worksheet.handle_input_change("inp_0_12", "c").unwrap();
        worksheet.handle_input_change("inp_0_13", "c").unwrap();
        worksheet.handle_input_change("inp_0_14", "c").unwrap();
        worksheet.handle_input_change("inp_0_15", "c").unwrap();

        worksheet.handle_input_change("inp_2_14", "c").unwrap();
        worksheet.handle_input_change("inp_2_15", "c").unwrap();
        worksheet.handle_input_change("inp_4_14", "c").unwrap();
        worksheet.handle_input_change("inp_4_15", "c").unwrap();
        worksheet.handle_input_change("inp_6_14", "c").unwrap();
        worksheet.handle_input_change("inp_6_15", "c").unwrap();
        worksheet.handle_input_change("inp_8_14", "c").unwrap();
        worksheet.handle_input_change("inp_8_15", "c").unwrap();
        worksheet.handle_input_change("inp_10_14", "c").unwrap();
        worksheet.handle_input_change("inp_10_15", "c").unwrap();
        worksheet.handle_input_change("inp_12_14", "c").unwrap();
        worksheet.handle_input_change("inp_12_15", "c").unwrap();
        worksheet.handle_input_change("inp_14_14", "c").unwrap();
        worksheet.handle_input_change("inp_14_15", "c").unwrap();
        worksheet.handle_input_change("inp_16_14", "c").unwrap();
        worksheet.handle_input_change("inp_16_15", "c").unwrap();
        worksheet.handle_input_change("inp_18_14", "c").unwrap();
        worksheet.handle_input_change("inp_18_15", "c").unwrap();
        worksheet.handle_input_change("inp_20_14", "c").unwrap();

        assert_eq!(worksheet.cell_below(2, 15), Some((3, 13)));
        assert_eq!(worksheet.cell_below(3, 13), Some((4, 13)));
    }
}
