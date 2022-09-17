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

//! Field Arithmetic
//!
//! Functionality to compute the codex32 error-correcting code, do field arithmetic, etc
//!

use std::ops;

/// Needed for indexing as we need a static-lifetime zero object
const ZERO: Fe = Fe(0);
/// The bech32 alphabet, in binary order
const BECH32_ALPHABET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
/// The codex32 generator polynomial
const CODEX32_POLYMOD: &[Fe] = &[
    Fe(25), Fe(27), Fe(17), Fe(8), Fe(0), Fe(25), Fe(25), Fe(25), Fe(31), Fe(27), Fe(24), Fe(16), Fe(16)
];

/// A single field element in the bech32 field
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Fe(u8);

impl Fe {
    /// Construct the additive identity of the field
    pub fn zero() -> Self { Fe(0) }
    /// Construct the multiplicative identity of the field
    pub fn one() -> Self { Fe(1) }

    /// Construct a field element from its binary expression
    pub fn from_bin(n: u8) -> Self { Fe(n) }
}

impl From<Fe> for char {
    fn from(fe: Fe) -> Self { BECH32_ALPHABET[fe.0 as usize].into() }
}

impl ops::Add<Fe> for Fe {
    type Output = Fe;
    fn add(self, other: Fe) -> Fe {
        Fe(self.0 ^ other.0)
    }
}
impl ops::Add<&Fe> for Fe {
    type Output = Fe;
    fn add(self, other: &Fe) -> Fe {
        Fe(self.0 ^ other.0)
    }
}

impl ops::Mul<Fe> for Fe {
    type Output = Fe;
    fn mul(self, other: Fe) -> Fe {
        self * &other
    }
}
impl ops::Mul<&Fe> for Fe {
    type Output = Fe;
    fn mul(mut self, other: &Fe) -> Fe {
        let mut ret = 0;
        let mut fe2 = other.0;
        while self.0 > 0 {
            if self.0 & 1  == 1 {
                ret ^= fe2;
            }

            self.0 >>= 1;
            fe2 <<= 1;

            if fe2 & 32 == 32 {
                fe2 ^= 32 + 8 + 1;
            }
        }
        Fe(ret)
    }
}


/// A polynomial in the bech32 field
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FePoly(Vec<Fe>);

impl From<Fe> for FePoly {
    fn from(fe: Fe) -> Self { FePoly(vec![fe]) }
}

impl ops::Index<usize> for FePoly {
    type Output = Fe;
    fn index(&self, idx: usize) -> &Fe { self.0.get(idx).unwrap_or(&ZERO) }
}

impl FePoly {
    /// Creates a 0 polynomial
    pub fn new() -> Self { Self::default() }

    /// Reduce a polynomial modulo the codex32 generator polynomial
    pub fn codex32_polymod(&self) -> Self {
        let mut ret = vec![Fe(0); 13];

        for ch in &self.0 {
            // Multiply residue by x
            let c13 = ret[0];
            for i in 0..12 {
                ret[i] = ret[i + 1];
            }
            // Add next character
            ret[12] = *ch;
            // Replace A*x^13 by A*polymod
            for i in 0..13 {
                ret[i] = ret[i] + c13 * CODEX32_POLYMOD[i];
            }
        }

        FePoly(ret)
    }
}

