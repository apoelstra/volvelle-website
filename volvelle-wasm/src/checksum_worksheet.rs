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
use crate::fe;
use serde::{Deserialize, Serialize};
use std::ops;

/// How to render a given cell
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum CellType {
    /// The HRP, which is fixed in all modes
    #[serde(rename = "fixed_hrp")]
    FixedHrp,
    /// The HRP's residue, which is fixed in all modes
    #[serde(rename = "fixed_hrp")]
    FixedResidue,
    /// Share data, which is editable in all modes and bolded
    #[serde(rename = "share_data_non_checksum")]
    ShareDataNonChecksum,
    /// Share checksum, which is computed in "create" mode but entered in "verify" mode
    #[serde(rename = "share_data_checksum")]
    ShareDataChecksum,
    /// Residue line, implied by cells above it in all modes
    #[serde(rename = "residue")]
    Residue,
    /// Sum, implied by cells above it in all modes
    #[serde(rename = "sum")]
    Sum,
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
        hrp: String,
        create_mode: CreateMode,
        size: usize,
        checksum: Checksum,
    ) -> Result<Worksheet, Error> {
        let mut ret = Worksheet {
            hrp,
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
        let n_rows = data_len / 2;

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
        self.hrp.len();
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
                ty: CellType::ShareDataNonChecksum,
                dom_id: format!("inp_{}_{}", 0, n + offset),
                val: None,
            });
        }
        offset
    }

    /// Helper to populate the second row (HRP residue) (does not change offset)
    fn add_second_row(&mut self, offset: usize) {
        assert_eq!(self.rows.len(), 1);
        self.rows.push(Row {
            start_idx: offset,
            cells: vec![],
        });
        let hrp_poly = match self.checksum {
            Checksum::Bech32 => fe::FePoly::bech32_hrp_residue(&self.hrp),
            Checksum::Codex32 => fe::FePoly::codex32_hrp_residue(&self.hrp),
        };
        for (n, fe) in hrp_poly.iter().enumerate() {
            self.rows[1].cells.push(Cell {
                ty: CellType::FixedResidue,
                dom_id: format!("inp_{}_{}", 1, n + offset),
                val: Some(fe.into()),
            });
        }
    }

    /// Helper to populate the nth and (n+1)th "ordinary" rows (sum then residue)
    fn add_2nth_rows(&mut self, mut offset: usize) -> usize {
        let ridx = self.rows.len();

        self.rows.push(Row {
            start_idx: offset,
            cells: vec![],
        });
        let row = self.rows.last_mut().unwrap();
        for n in 0..self.checksum.len() {
            row.cells.push(Cell {
                ty: CellType::Sum,
                dom_id: format!("inp_{}_{}", ridx, n + offset),
                val: None,
            });
        }
        for n in 0..2 {
            row.cells.push(Cell {
                ty: CellType::ShareDataNonChecksum,
                dom_id: format!("inp_{}_{}", ridx, n + offset + self.checksum.len()),
                val: None,
            });
        }
        offset += 2;

        self.rows.push(Row {
            start_idx: offset,
            cells: vec![],
        });
        let row = self.rows.last_mut().unwrap();
        for n in 0..self.checksum.len() {
            row.cells.push(Cell {
                ty: CellType::Sum,
                dom_id: format!("inp_{}_{}", ridx, n + offset),
                val: None,
            });
        }
        offset
    }

    fn add_final_row(&mut self, offset: usize) {
        let ridx = self.rows.len();

        self.rows.push(Row {
            start_idx: offset,
            cells: vec![],
        });

        let row = self.rows.last_mut().unwrap();
        for n in 0..self.checksum.len() {
            row.cells.push(Cell {
                ty: CellType::GlobalResidue,
                dom_id: format!("inp_{}_{}", ridx, n + offset),
                val: None,
            });
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
