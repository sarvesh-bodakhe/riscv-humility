// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The architecture abstraction for Hubris targets.
//!
//! Humility grew up 100% ARM/Cortex-M; the Hubris RISC-V port makes the
//! architecture a property of the *archive*, not of Humility. This trait
//! carries exactly the decisions that turned out to be ARM-specific when
//! the tree was audited for the port, and no more; it grows as commands
//! are brought up on the second architecture.
//!
//! The archive selects its backend from the ELF `e_machine` of the objects
//! it loads (see `HubrisArchive::load_object`), so nothing downstream ever
//! has to guess.

use crate::reg::RegId;
use humility_arch_arm::ARMRegister;

pub trait Arch: Send + Sync {
    /// Short name, for messages ("arm-m", "riscv32").
    fn name(&self) -> &'static str;

    /// The ELF `e_machine` this backend corresponds to.
    fn elf_machine(&self) -> u16;

    /// Strips ISA tag bits from a function symbol's address. On ARM this
    /// clears the Thumb bit; most architectures have nothing to strip.
    fn strip_fn_addr(&self, addr: u32) -> u32 {
        addr
    }

    /// The program counter, as named on the debug transport.
    fn pc_reg(&self) -> RegId;

    /// The name of the `SavedState` member holding syscall argument `n`.
    ///
    /// Hubris keeps syscall arguments in registers whose spill into
    /// `SavedState` *is* the argument marshalling, so commands that want a
    /// blocked task's syscall arguments (notification masks, panic
    /// messages) read these members by name. Which registers those are is
    /// each port's ABI choice: `r4`.. on ARM, `a0`.. on RISC-V.
    fn saved_arg_member(&self, n: usize) -> Option<&'static str>;

    /// Whether the instruction-analysis passes (disassembly, syscall-push
    /// tracking, branch-target extraction) understand this architecture.
    /// When false, the archive skips them and the features they feed
    /// (`-s` symbolization, syscall-aware stack walks) degrade gracefully.
    fn has_instr_analysis(&self) -> bool {
        false
    }

    /// Whether register reconstruction and stack unwinding
    /// (`HubrisArchive::{registers,stack}`, `humility tasks -r/-s`) are
    /// implemented for this architecture.
    fn has_unwind(&self) -> bool {
        false
    }
}

/// The ARM Cortex-M backend: everything Humility historically assumed.
pub struct ArmM;

impl Arch for ArmM {
    fn name(&self) -> &'static str {
        "arm-m"
    }

    fn elf_machine(&self) -> u16 {
        goblin::elf::header::EM_ARM
    }

    /// Function symbols on ARM carry the Thumb bit in their address.
    fn strip_fn_addr(&self, addr: u32) -> u32 {
        addr & !1
    }

    fn pc_reg(&self) -> RegId {
        ARMRegister::PC.into()
    }

    /// The ARM port's syscall args live in r4..r10 (doc/syscalls.adoc).
    fn saved_arg_member(&self, n: usize) -> Option<&'static str> {
        Some(match n {
            0 => "r4",
            1 => "r5",
            2 => "r6",
            3 => "r7",
            4 => "r8",
            5 => "r9",
            6 => "r10",
            _ => return None,
        })
    }

    fn has_instr_analysis(&self) -> bool {
        true
    }

    fn has_unwind(&self) -> bool {
        true
    }
}

/// Returns the backend for an ELF `e_machine`, if the architecture is
/// known.
pub fn from_elf_machine(machine: u16) -> Option<&'static dyn Arch> {
    match machine {
        goblin::elf::header::EM_ARM => Some(&ArmM),
        _ => None,
    }
}
