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

    /// The stack pointer, as named on the debug transport.
    fn sp_reg(&self) -> RegId;

    /// The return-address register (`lr` on ARM, `ra` on RISC-V).
    fn ret_reg(&self) -> RegId;

    /// Maps a DWARF register number (as used by CFI rules) to the
    /// transport register id. The numbering is per-architecture and NOT
    /// the transport numbering -- conflating the two happens to work on
    /// ARM (both count r0..r15 from zero) and silently corrupts on
    /// everything else, which is why this hook exists.
    fn dwarf_reg(&self, n: u16) -> Option<RegId>;

    /// The architecture's name for a register, for display.
    fn reg_name(&self, reg: RegId) -> Option<&'static str>;

    /// Registers worth showing a human, in display order.
    fn display_regs(&self) -> &'static [RegId];

    /// When the port's trap machinery saves the *entire* register file
    /// into the Task's `SavedState` (the RISC-V port does), this returns
    /// the member name and transport id of every saved register, and
    /// register reconstruction is a single struct read. `None` means the
    /// state is split and the architecture needs its own reconstruction
    /// (ARM: hardware pushes half the file onto the task stack).
    fn save_members(&self) -> Option<&'static [(&'static str, RegId)]> {
        None
    }

    /// The `SavedState` member holding the frame-pointer-ish register
    /// used by heuristic stack guessing (`r7` on ARM Hubris, `s0` on
    /// RISC-V).
    fn saved_fp_member(&self) -> &'static str;

    /// The transport id of that same frame-pointer-ish register.
    fn fp_reg(&self) -> RegId;

    /// Whether a return-address value marks the exception/kernel
    /// boundary, ending a kernel stack walk (ARM's EXC_RETURN). No such
    /// sentinel exists on RISC-V; its walks end at the stack limit or
    /// when no frame info covers the pc.
    fn is_exception_return(&self, ra: u32) -> bool {
        let _ = ra;
        false
    }

    /// Bias subtracted from a return address before symbolizing the
    /// frame below the top, so it names the *call* rather than the
    /// instruction after it. The correction is equally valid on every
    /// architecture, but the ARM backend keeps the historic unbiased
    /// lookup: years of Humility output (and its recorded test
    /// expectations) pin it, and diverging from upstream there buys
    /// nothing but merge pain.
    fn ret_addr_symbolize_bias(&self) -> u32 {
        0
    }

    /// Whether the port's syscall stubs are true leaves with no CFI --
    /// they never touch sp or ra, so a task parked in one can be
    /// unwound by treating the first frame as a leaf (return address
    /// still in the return register, sp unmoved). The RISC-V stubs are
    /// this shape. ARM's stubs push a frame and carry CFI, so a
    /// first-frame CFI miss there is a genuine unknown and must stay
    /// an error, as it always has.
    fn has_cfi_less_leaf_stubs(&self) -> bool {
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

    fn sp_reg(&self) -> RegId {
        ARMRegister::SP.into()
    }

    fn ret_reg(&self) -> RegId {
        ARMRegister::LR.into()
    }

    /// ARM DWARF numbering (r0..r15 = 0..15) coincides with the DCRSR
    /// transport numbering, so this is (validated) identity.
    fn dwarf_reg(&self, n: u16) -> Option<RegId> {
        use num_traits::FromPrimitive;
        ARMRegister::from_u16(n).map(RegId::from)
    }

    //
    // These are ARMRegister's own Display forms: the names (and the
    // uppercase) are what Humility has always printed, and the trycmd
    // expectations pin them.
    //
    fn reg_name(&self, reg: RegId) -> Option<&'static str> {
        use num_traits::FromPrimitive;
        ARMRegister::from_u16(reg.0).map(|r| match r {
            ARMRegister::R0 => "R0",
            ARMRegister::R1 => "R1",
            ARMRegister::R2 => "R2",
            ARMRegister::R3 => "R3",
            ARMRegister::R4 => "R4",
            ARMRegister::R5 => "R5",
            ARMRegister::R6 => "R6",
            ARMRegister::R7 => "R7",
            ARMRegister::R8 => "R8",
            ARMRegister::R9 => "R9",
            ARMRegister::R10 => "R10",
            ARMRegister::R11 => "R11",
            ARMRegister::R12 => "R12",
            ARMRegister::SP => "SP",
            ARMRegister::LR => "LR",
            ARMRegister::PC => "PC",
            ARMRegister::PSR => "PSR",
            _ => "?",
        })
    }

    fn display_regs(&self) -> &'static [RegId] {
        //
        // r0..r12, sp, lr, pc, xPSR -- the DCRSR encodings, which
        // ARMRegister's discriminants are: 0..=15 then 16 (xPSR).
        //
        const REGS: [RegId; 17] = {
            let mut r = [RegId(0); 17];
            let mut i = 0;
            while i < 17 {
                r[i] = RegId(i as u16);
                i += 1;
            }
            r
        };
        &REGS
    }

    fn saved_fp_member(&self) -> &'static str {
        "r7"
    }

    fn fp_reg(&self) -> RegId {
        ARMRegister::R7.into()
    }

    fn is_exception_return(&self, ra: u32) -> bool {
        ra >> 28 == 0xf
    }
}

/// The RV32 backend, for Hubris on RISC-V; the ESP32-C6 is its first
/// target.
///
/// Instruction analysis (capstone passes) is not implemented yet and its
/// capability reports false; everything else -- reflection, register
/// reconstruction from the port's whole-file `SavedState`, and DWARF CFI
/// stack unwinding -- is.
pub struct Riscv32;

impl Arch for Riscv32 {
    fn name(&self) -> &'static str {
        "riscv32"
    }

    fn elf_machine(&self) -> u16 {
        goblin::elf::header::EM_RISCV
    }

    // Function symbols carry no tag bits on RISC-V; the default identity
    // `strip_fn_addr` stands.

    fn pc_reg(&self) -> RegId {
        //
        // The debug module exposes the halted PC as the dpc CSR, 0x7b1,
        // which is also how probe-rs names it.
        //
        RegId(0x7b1)
    }

    /// The RISC-V port's syscall args live in a0..a6 (doc/syscalls.adoc),
    /// and SavedState's members carry the ABI names.
    fn saved_arg_member(&self, n: usize) -> Option<&'static str> {
        Some(match n {
            0 => "a0",
            1 => "a1",
            2 => "a2",
            3 => "a3",
            4 => "a4",
            5 => "a5",
            6 => "a6",
            _ => return None,
        })
    }

    fn has_unwind(&self) -> bool {
        true
    }

    fn sp_reg(&self) -> RegId {
        RegId(0x1002) // x2
    }

    fn ret_reg(&self) -> RegId {
        RegId(0x1001) // x1, ra
    }

    /// RISC-V DWARF numbering is x0..x31 = 0..31; the transport numbers
    /// the same registers 0x1000..0x101f.
    fn dwarf_reg(&self, n: u16) -> Option<RegId> {
        if n < 32 { Some(RegId(0x1000 + n)) } else { None }
    }

    fn reg_name(&self, reg: RegId) -> Option<&'static str> {
        if reg == self.pc_reg() {
            return Some("pc");
        }
        let n = reg.0.checked_sub(0x1000)?;
        RISCV_SAVE_MEMBERS
            .iter()
            .find(|(_, r)| r.0 == 0x1000 + n)
            .map(|(name, _)| *name)
    }

    fn display_regs(&self) -> &'static [RegId] {
        //
        // pc then x1..x31, in transport numbering.
        //
        const REGS: [RegId; 32] = {
            let mut r = [RegId(0x7b1); 32];
            let mut i = 1;
            while i < 32 {
                r[i] = RegId(0x1000 + i as u16);
                i += 1;
            }
            r
        };
        &REGS
    }

    fn save_members(&self) -> Option<&'static [(&'static str, RegId)]> {
        Some(RISCV_SAVE_MEMBERS)
    }

    fn has_cfi_less_leaf_stubs(&self) -> bool {
        true
    }

    fn ret_addr_symbolize_bias(&self) -> u32 {
        1
    }

    fn saved_fp_member(&self) -> &'static str {
        "s0"
    }

    fn fp_reg(&self) -> RegId {
        RegId(0x1008) // x8, s0
    }
}

/// The RISC-V port's `SavedState` members and their transport ids: the
/// whole integer file in x-order (the trap entry spills it contiguously),
/// then the pc (from `mepc`). Names match the struct definition in
/// `sys/kern/src/arch/riscv32.rs`.
const RISCV_SAVE_MEMBERS: &[(&str, RegId)] = &[
    ("ra", RegId(0x1001)),
    ("sp", RegId(0x1002)),
    ("gp", RegId(0x1003)),
    ("tp", RegId(0x1004)),
    ("t0", RegId(0x1005)),
    ("t1", RegId(0x1006)),
    ("t2", RegId(0x1007)),
    ("s0", RegId(0x1008)),
    ("s1", RegId(0x1009)),
    ("a0", RegId(0x100a)),
    ("a1", RegId(0x100b)),
    ("a2", RegId(0x100c)),
    ("a3", RegId(0x100d)),
    ("a4", RegId(0x100e)),
    ("a5", RegId(0x100f)),
    ("a6", RegId(0x1010)),
    ("a7", RegId(0x1011)),
    ("s2", RegId(0x1012)),
    ("s3", RegId(0x1013)),
    ("s4", RegId(0x1014)),
    ("s5", RegId(0x1015)),
    ("s6", RegId(0x1016)),
    ("s7", RegId(0x1017)),
    ("s8", RegId(0x1018)),
    ("s9", RegId(0x1019)),
    ("s10", RegId(0x101a)),
    ("s11", RegId(0x101b)),
    ("t3", RegId(0x101c)),
    ("t4", RegId(0x101d)),
    ("t5", RegId(0x101e)),
    ("t6", RegId(0x101f)),
    ("pc", RegId(0x7b1)),
];

/// Returns the backend for an ELF `e_machine`, if the architecture is
/// known.
pub fn from_elf_machine(machine: u16) -> Option<&'static dyn Arch> {
    match machine {
        goblin::elf::header::EM_ARM => Some(&ArmM),
        goblin::elf::header::EM_RISCV => Some(&Riscv32),
        _ => None,
    }
}
