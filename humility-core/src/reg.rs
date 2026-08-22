// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Architecture-neutral register identifiers.
//!
//! Humility historically used [`humility_arch_arm::ARMRegister`] everywhere
//! a register crossed an interface, which welded the probe abstraction to
//! one architecture. A [`RegId`] is the neutral currency: a 16-bit selector
//! whose value is whatever the debug transport uses to name the register --
//! for ARM the DCRSR selector encodings (which `ARMRegister`'s
//! discriminants already are), for RISC-V the abstract-command register
//! numbers (`0x1000 + n` for `x_n`, `0x7b1` for `dpc`). Both map straight
//! onto probe-rs `RegisterId`, so the conversion at the probe boundary is
//! the identity.
//!
//! Architecture *semantics* -- names, callee-saved-ness, which register is
//! the PC -- stay in the arch layers ([`crate::arch`]); a `RegId` by itself
//! is just a wire selector.

use humility_arch_arm::ARMRegister;
use num_traits::ToPrimitive;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegId(pub u16);

impl From<ARMRegister> for RegId {
    fn from(reg: ARMRegister) -> Self {
        //
        // ARMRegister's discriminants are the DCRSR selector encodings,
        // which is exactly what the probe layer wants; see the type's
        // own documentation.
        //
        RegId(reg.to_u16().unwrap())
    }
}

impl std::fmt::Display for RegId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "reg {:#x}", self.0)
    }
}
