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

//! DOM Actions

use serde::{Deserialize, Serialize};

/// Type of action
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum ActionType {
    /// Flash a cell red indicating that the input was invalid
    #[serde(rename = "flash_error")]
    FlashError,
    /// Flash a cell green and set it to a new (corrected) value
    #[serde(rename = "flash_set")]
    FlashSet,
    /// Set a cell to a value (without any flashing or anything)
    #[serde(rename = "set")]
    Set,
}

/// Action to perform on the DOM
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Action {
    /// What to do
    pub ty: ActionType,
    /// and what cell to do it to
    pub id: String,
    /// Data for the action (e.g. to set a cell to a particular value)
    pub value: Option<char>,
}
