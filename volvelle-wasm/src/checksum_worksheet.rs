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
use std::{iter, ops};

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
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Cell {
    ty: CellType,
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

        let mut offset = 0;
        // Treat first two rows specially
        ret.rows.push(Row {
            start_idx: 0,
            cells: vec![],
        });
        ret.rows[0].cells.extend(ret.hrp.chars().map(|ch| Cell {
            ty: CellType::FixedHrp,
            val: Some(ch.to_ascii_uppercase()),
        }));
        ret.rows[0].cells.push(Cell {
            ty: CellType::FixedHrp,
            val: Some('1'),
        });
        ret.rows[0].cells.extend(
            iter::repeat(Cell {
                ty: CellType::ShareDataNonChecksum,
                val: None,
            })
            .take(checksum.len()),
        );

        offset += ret.hrp.len() + 1;
        ret.rows.push(Row {
            start_idx: offset,
            cells: vec![],
        });
        let hrp_poly = match checksum {
            Checksum::Bech32 => fe::FePoly::bech32_hrp_residue(&ret.hrp),
            Checksum::Codex32 => fe::FePoly::codex32_hrp_residue(&ret.hrp),
        };
        ret.rows[1].cells.extend(hrp_poly.iter().map(|fe| Cell {
            ty: CellType::FixedResidue,
            val: Some(fe.into()),
        }));

        // For the remaining rows except the global SECRETSHARE32 residue
        for _ in 1..n_rows {
            ret.rows.push(Row {
                start_idx: offset,
                cells: vec![],
            });
            ret.rows.last_mut().unwrap().cells.extend(
                iter::repeat(Cell {
                    ty: CellType::Sum,
                    val: None,
                })
                .take(checksum.len()),
            );
            ret.rows.last_mut().unwrap().cells.extend(
                iter::repeat(Cell {
                    ty: CellType::ShareDataNonChecksum,
                    val: None,
                })
                .take(2),
            );
            offset += 2;

            ret.rows.push(Row {
                start_idx: offset,
                cells: vec![],
            });
            ret.rows.last_mut().unwrap().cells.extend(
                iter::repeat(Cell {
                    ty: CellType::Residue,
                    val: None,
                })
                .take(checksum.len()),
            );
        }

        // Finally stick the global residue row on
        ret.rows.push(Row {
            start_idx: offset,
            cells: vec![],
        });
        ret.rows.last_mut().unwrap().cells.extend(
            iter::repeat(Cell {
                ty: CellType::GlobalResidue,
                val: None,
            })
            .take(checksum.len()),
        );

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
