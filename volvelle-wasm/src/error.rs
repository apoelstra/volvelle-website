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

//! Error

use std::{error, fmt};

/// The main error type for the whole crate
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Error {
    BadBech32Char {
        ch: char,
    },
    BadShareDataLen {
        len: usize,
    },
    OddLength {
        data_len: usize,
    },
    TooShort {
        minimum: usize,
        actual: usize,
    },
    UnknownCell {
        id: String,
        reason: &'static str,
    },
    InvalidRow {
        row: usize,
        n_rows: usize,
    },
    InvalidCell {
        row: usize,
        cell: usize,
        n_cells: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::BadBech32Char { ch } => {
                write!(f, "Bad bech32 character {}", ch)
            }
            Error::BadShareDataLen { len } => {
                write!(f, "Bad share data length {}", len)
            }
            Error::OddLength { data_len } => {
                write!(
                    f,
                    "Data length is {}, which is odd (unsupported for now)",
                    data_len
                )
            }
            Error::TooShort { minimum, actual } => {
                write!(
                    f,
                    "Share size is {} but HRP+checksum need size {}",
                    actual, minimum
                )
            }
            Error::UnknownCell { ref id, reason } => {
                write!(f, "Unknown cell id {} ({})", id, reason)
            }
            Error::InvalidRow { row, n_rows } => {
                write!(f, "Invalid row {} (have {} rows)", row, n_rows)
            }
            Error::InvalidCell { row, cell, n_cells } => {
                write!(
                    f,
                    "Invalid cell {} (row {} has {} cells)",
                    cell, row, n_cells
                )
            }
        }
    }
}

impl error::Error for Error {}
