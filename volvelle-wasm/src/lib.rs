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
mod dom_action;
mod error;
mod fe;

use crate::error::Error;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn new_session(
    hrp: String,
    threshold: usize,
    size: usize,
    checksum: JsValue,
) -> Result<JsValue, JsValue> {
    let checksum = serde_wasm_bindgen::from_value(checksum)?;
    serde_wasm_bindgen::to_value(&Session {
        hrp,
        threshold,
        size,
        checksum,
        shares: vec![],
        active_share: None,
        action_queue: vec![],
    })
    .map_err(From::from)
}

#[wasm_bindgen]
pub fn new_share(session: JsValue) -> Result<JsValue, JsError> {
    let mut session: Session = serde_wasm_bindgen::from_value(session)?;
    session.new_share()?;
    serde_wasm_bindgen::to_value(&session).map_err(From::from)
}

#[wasm_bindgen]
pub fn handle_input_change(session: JsValue, id: &str, val: &str) -> Result<JsValue, JsError> {
    let mut session: Session = serde_wasm_bindgen::from_value(session)?;
    if let Some(idx) = session.active_share {
        session.action_queue = session.shares[idx].handle_input_change(id, val)?;
    }
    serde_wasm_bindgen::to_value(&session).map_err(From::from)
}

/// The entire checksumming session
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[wasm_bindgen]
pub struct Session {
    hrp: String,
    threshold: usize,
    size: usize,
    checksum: checksum_worksheet::Checksum,
    shares: Vec<checksum_worksheet::Worksheet>,
    active_share: Option<usize>,
    /// Only set when returning a session from a "handle event"-type call
    action_queue: Vec<dom_action::Action>,
}

impl Session {
    /// Adds a share to a session
    pub fn new_share(&mut self) -> Result<(), Error> {
        let new = checksum_worksheet::Worksheet::new(
            &self.hrp,
            checksum_worksheet::CreateMode::Create,
            self.size,
            self.checksum,
        )?;
        self.shares.push(new);
        Ok(())
    }
}
