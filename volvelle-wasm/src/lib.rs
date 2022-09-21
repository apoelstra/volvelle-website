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

//! Volvelle Website (rust-wasm support code)

mod checksum_worksheet;
mod error;
mod fe;

use crate::fe::Checksum;
use wasm_bindgen::prelude::*;

/// The entire checksumming session
#[wasm_bindgen]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Session {
    hrp: String,
    threshold: usize,
    size: usize,
    checksum: Checksum,
    shares: Vec<checksum_worksheet::Worksheet>,
    active_share: Option<usize>,
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new(hrp: String, threshold: usize, size: usize, checksum: Checksum) -> Session {
        Session {
            hrp,
            threshold,
            size,
            checksum,
            shares: vec![],
            active_share: None,
        }
    }

    /// Adds a share to a session
    pub fn new_share(&mut self) -> Result<usize, JsError> {
        let new = checksum_worksheet::Worksheet::new(
            &self.hrp,
            checksum_worksheet::CreateMode::Create,
            self.size,
            self.checksum,
        )?;
        self.shares.push(new);
        Ok(self.shares.len() - 1)
    }

    /// Activates a specific share
    pub fn set_active_share(&mut self, idx: usize) {
        self.active_share = Some(idx);
    }

    /// De-activates any share
    pub fn clear_active_share(&mut self) {
        self.active_share = None;
    }

    /// Gets the list of cells to build a checksum worksheet from
    pub fn get_checksum_worksheet_cells(&mut self) -> Result<js_sys::Array, JsError> {
        let idx = self
            .active_share
            .ok_or_else(|| JsError::new("get_checksum_worksheet_cells: no active share"))?;
        let share = self
            .shares
            .get(idx)
            .ok_or_else(|| JsError::new("get_checksum_worksheet_cells: bad active share idx"))?;
        // FIXME this conversion is inefficient and ought to be unnecessary but
        // if we directly create a js_sys::Array in get_dom_cells then our unit
        // tests break
        share
            .get_dom_cells()
            .map(|vec| vec.into_iter().map(JsValue::from).collect())
    }

    /// Responds to a user update of a cell by updating the state of the sheet
    ///
    /// Returns a list of updated cells for the JS to update the DOM with
    pub fn handle_input_change(&mut self, id: &str, val: &str) -> Result<js_sys::Array, JsError> {
        let idx = self
            .active_share
            .ok_or_else(|| JsError::new("handle_input_change: no active share"))?;
        let share = self
            .shares
            .get_mut(idx)
            .ok_or_else(|| JsError::new("handle_input_change: bad active share idx"))?;
        // FIXME this conversion is inefficient and ought to be unnecessary but
        // if we directly create a js_sys::Array in get_dom_cells then our unit
        // tests break
        share
            .handle_input_change(id, val)
            .map(|vec| vec.into_iter().map(JsValue::from).collect())
            .map_err(From::from)
    }
}
