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

use serde::{Deserialize, Serialize};
use std::{error, fmt};

/// The main error type for the whole crate
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum Error {
    OddLength { data_len: usize },
    TooShort { minimum: usize, actual: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
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
        }
    }
}

impl error::Error for Error {}
