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

use crate::error::Error;
use crate::fe::Checksum;
use wasm_bindgen::prelude::*;

/// The entire checksumming session
#[wasm_bindgen]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Session {
    hrp: String,
    pub threshold: usize,
    pub size: usize,
    pub checksum: Checksum,
    shares: Vec<checksum_worksheet::Worksheet>,
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
        }
    }

    #[wasm_bindgen(getter)]
    pub fn hrp(&self) -> String {
        self.hrp.clone()
    }
    #[wasm_bindgen(setter)]
    pub fn set_hrp(&mut self, s: String) {
        self.hrp = s;
    }

    pub fn n_shares(&self) -> usize {
        self.shares.len()
    }

    /// Adds a share to a session
    pub fn new_share(&mut self) -> Result<usize, JsError> {
        let idx = self.shares.len();
        let new = checksum_worksheet::Worksheet::new(&self.hrp, self.size, self.checksum, idx)?;
        self.shares.push(new);
        Ok(idx)
    }

    /// Gets the list of cells to build a checksum worksheet from
    pub fn get_checksum_worksheet_cells(&self, idx: usize) -> Result<js_sys::Array, JsError> {
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

    /// Gets the list of cells to build a checksum worksheet from
    pub fn get_checksum_worksheet_header_str(&self, idx: usize) -> Result<String, JsError> {
        let share = self
            .shares
            .get(idx)
            .ok_or_else(|| JsError::new("get_checksum_worksheet_cells: bad active share idx"))?;
        Ok(share.header_str())
    }

    /// Extracts the share index from an input ID
    pub fn get_idx_of(&self, id: &str) -> Result<usize, JsError> {
        Ok(cell_from_name(id)?[0])
    }

    /// Responds to a user update of a cell by updating the state of the sheet
    ///
    /// Returns a list of updated cells for the JS to update the DOM with
    pub fn handle_input_change(&mut self, id: &str, val: &str) -> Result<js_sys::Array, JsError> {
        let id = cell_from_name(id)?;

        let share = self
            .shares
            .get_mut(id[0])
            .ok_or_else(|| JsError::new("handle_input_change: bad active share idx"))?;
        // FIXME this conversion is inefficient and ought to be unnecessary but
        // if we directly create a js_sys::Array in get_dom_cells then our unit
        // tests break
        share
            .handle_input_change(id[1], id[2], val)
            .map(|vec| vec.into_iter().map(JsValue::from).collect())
            .map_err(From::from)
    }

    /// Outputs a serialization of the session which can be read from local storage
    pub fn local_storage_str(&self) -> String {
        let mut ret = format!(
            "{}_{}_{}_{}_{}",
            self.size,
            match self.checksum {
                Checksum::Bech32 => 0,
                Checksum::Codex32 => 1,
            },
            self.threshold,
            self.hrp.len(),
            self.hrp,
        );
        for s in &self.shares {
            ret.push('_');
            s.cells_into_str(&mut ret);
        }
        ret
    }

    /// Outputs a serialization of the session which can be read from local storage
    pub fn update_from_local_storage_str(&mut self, s: &str) -> Result<(), JsError> {
        let mut iter = s.splitn(5, '_');
        self.size = iter
            .next()
            .ok_or_else(|| JsError::new("missing size"))?
            .parse()?;
        let checksum = iter
            .next()
            .ok_or_else(|| JsError::new("missing checksum"))?
            .parse::<usize>()?;
        self.checksum = match checksum {
            0 => Checksum::Bech32,
            1 => Checksum::Codex32,
            _ => return Err(JsError::new("bad checksum value")),
        };
        self.threshold = iter
            .next()
            .ok_or_else(|| JsError::new("missing threshold"))?
            .parse()?;
        let hrp_len = iter
            .next()
            .ok_or_else(|| JsError::new("missing hrp len"))?
            .parse::<usize>()?;
        let rem = iter.next().ok_or_else(|| JsError::new("missing hrp"))?;
        if rem.len() < hrp_len {
            return Err(JsError::new("bad hrp len"));
        }
        self.hrp = rem[..hrp_len].into();

        if rem.len() > hrp_len {
            for share_data in rem[hrp_len + 1..].split("_") {
                let idx = self.new_share()?;
                self.shares[idx].cells_from_str(share_data)?;
            }
        }
        Ok(())
    }
}

/// Helper function to translate a cell ID into a shareidx/row/cell index tuple
fn cell_from_name(s: &str) -> Result<[usize; 3], Error> {
    use std::str::FromStr;
    let mut iter = s.split('_');
    if iter.next() != Some("inp") {
        return Err(Error::UnknownCell {
            id: s.into(),
            reason: "no inp_ prefix",
        });
    }
    let mut ret = [0; 3];
    for i in 0..3 {
        let ns = iter.next().ok_or_else(|| Error::UnknownCell {
            id: s.into(),
            reason: "missing number",
        })?;
        ret[i] = usize::from_str(ns).map_err(|_| Error::UnknownCell {
            id: s.into(),
            reason: "bad number",
        })?;
    }
    if iter.next().is_none() {
        Ok(ret)
    } else {
        Err(Error::UnknownCell {
            id: s.into(),
            reason: "extra numbers",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_from_name() {
        assert_eq!(cell_from_name("inp_0_0_0"), Ok([0, 0, 0]));
        assert_eq!(cell_from_name("inp_0_10_0"), Ok([0, 10, 0]));
        assert_eq!(cell_from_name("inp_100_10_10"), Ok([100, 10, 10]));
        assert!(cell_from_name("inp_10_10_10_10").is_err());
        assert!(cell_from_name("inp_10_10").is_err());
        assert!(cell_from_name("inP_10_10_10").is_err());
        assert!(cell_from_name("inp_10").is_err());
        assert!(cell_from_name("inp___").is_err());
    }
}
