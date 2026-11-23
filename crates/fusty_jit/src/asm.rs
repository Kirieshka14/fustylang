use crate::JitError;

/// General-purpose 64-bit x86-64 registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reg {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

impl Reg {
    #[inline]
    pub const fn low3(self) -> u8 {
        (self as u8) & 0x7
    }

    #[inline]
    pub const fn is_ext(self) -> bool {
        (self as u8) >= 8
    }
}

/// Condition codes for x86-64 `jcc` (0x0F 0x80+cc) and `setcc` (0x0F 0x90+cc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Cond {
    E = 0x4,  // ==
    Ne = 0x5, // !=
    B = 0x2,  // unsigned <  (also float < after ucomisd)
    Ae = 0x3, // unsigned >= (also float >= after ucomisd)
    Be = 0x6, // unsigned <= (also float <= after ucomisd)
    A = 0x7,  // unsigned >  (also float > after ucomisd)
    L = 0xC,  // signed <
    Ge = 0xD, // signed >=
    Le = 0xE, // signed <=
    G = 0xF,  // signed >
}

/// Opaque jump/call target label resolved during `finalize()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(usize);

struct Fixup {
    /// Offset of the 4-byte `rel32` immediate inside `code`.
    patch_offset: usize,
    target: Label,
}

/// Pure-Rust x86-64 machine code assembler with rel32 label resolution.
pub struct Assembler {
    code: Vec<u8>,
    labels: Vec<Option<usize>>,
    fixups: Vec<Fixup>,
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            code: Vec::with_capacity(256),
            labels: Vec::new(),
            fixups: Vec::new(),
        }
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.code.len()
    }

    pub fn create_label(&mut self) -> Label {
        let id = self.labels.len();
        self.labels.push(None);
        Label(id)
    }

    pub fn bind_label(&mut self, label: Label) {
        self.labels[label.0] = Some(self.code.len());
    }

    #[inline]
    pub(crate) fn emit_u8(&mut self, b: u8) {
        self.code.push(b);
    }

    #[inline]
    pub(crate) fn emit_i32(&mut self, v: i32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    #[inline]
    pub(crate) fn emit_u64(&mut self, v: u64) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// Emit REX prefix: `0100 W R X B`.
    #[inline]
    pub(crate) fn emit_rex(&mut self, w: bool, r: bool, x: bool, b: bool) {
        let rex = 0x40
            | if w { 0x08 } else { 0 }
            | if r { 0x04 } else { 0 }
            | if x { 0x02 } else { 0 }
            | if b { 0x01 } else { 0 };
        self.emit_u8(rex);
    }

    #[inline]
    pub(crate) fn modrm(md: u8, reg: u8, rm: u8) -> u8 {
        ((md & 0x3) << 6) | ((reg & 0x7) << 3) | (rm & 0x7)
    }

    /// Emit `[base + disp32]` ModRM (+ SIB if base is RSP/R12) + disp32.
    pub(crate) fn emit_mem_disp32(&mut self, reg_field: u8, base: Reg, disp: i32) {
        self.emit_u8(Self::modrm(0b10, reg_field, base.low3()));
        if base.low3() == 0b100 {
            // RSP / R12 require a SIB byte (scale=0, index=RSP/none=4, base=4)
            self.emit_u8(0x24);
        }
        self.emit_i32(disp);
    }

    // --- Stack & Control ---

    pub fn push_reg(&mut self, r: Reg) {
        if r.is_ext() {
            self.emit_rex(false, false, false, true);
        }
        self.emit_u8(0x50 + r.low3());
    }

    pub fn pop_reg(&mut self, r: Reg) {
        if r.is_ext() {
            self.emit_rex(false, false, false, true);
        }
        self.emit_u8(0x58 + r.low3());
    }

    pub fn ret(&mut self) {
        self.emit_u8(0xC3);
    }

    pub fn nop(&mut self) {
        self.emit_u8(0x90);
    }

    // --- Data Movement ---

    /// `mov dst, src` (64-bit)
    pub fn mov_rr(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.is_ext(), false, dst.is_ext());
        self.emit_u8(0x89);
        self.emit_u8(Self::modrm(0b11, src.low3(), dst.low3()));
    }

    /// `movabs dst, imm64`
    pub fn mov_ri64(&mut self, dst: Reg, imm: i64) {
        self.emit_rex(true, false, false, dst.is_ext());
        self.emit_u8(0xB8 + dst.low3());
        self.emit_u64(imm as u64);
    }

    /// `mov dst, qword ptr [base + disp32]`
    pub fn mov_rm_disp(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.emit_rex(true, dst.is_ext(), false, base.is_ext());
        self.emit_u8(0x8B);
        self.emit_mem_disp32(dst.low3(), base, disp);
    }

    /// `mov qword ptr [base + disp32], src`
    pub fn mov_mr_disp(&mut self, base: Reg, disp: i32, src: Reg) {
        self.emit_rex(true, src.is_ext(), false, base.is_ext());
        self.emit_u8(0x89);
        self.emit_mem_disp32(src.low3(), base, disp);
    }

    /// `mov byte ptr [base], imm8` (used for writing `out_err` status codes)
    pub fn mov_byte_mr(&mut self, base: Reg, imm: u8) {
        if base.is_ext() {
            self.emit_rex(false, false, false, true);
        }
        self.emit_u8(0xC6);
        self.emit_mem_disp32(0, base, 0);
        self.emit_u8(imm);
    }

    /// `lea dst, [base + disp32]`
    pub fn lea_disp(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.emit_rex(true, dst.is_ext(), false, base.is_ext());
        self.emit_u8(0x8D);
        self.emit_mem_disp32(dst.low3(), base, disp);
    }

    // --- Integer Arithmetic ---

    /// `add dst, src`
    pub fn add_rr(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.is_ext(), false, dst.is_ext());
        self.emit_u8(0x01);
        self.emit_u8(Self::modrm(0b11, src.low3(), dst.low3()));
    }

    /// `sub dst, src`
    pub fn sub_rr(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.is_ext(), false, dst.is_ext());
        self.emit_u8(0x29);
        self.emit_u8(Self::modrm(0b11, src.low3(), dst.low3()));
    }

    /// `imul dst, src` (signed 64-bit)
    pub fn imul_rr(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, dst.is_ext(), false, src.is_ext());
        self.emit_u8(0x0F);
        self.emit_u8(0xAF);
        self.emit_u8(Self::modrm(0b11, dst.low3(), src.low3()));
    }

    /// `neg r`
    pub fn neg_r(&mut self, r: Reg) {
        self.emit_rex(true, false, false, r.is_ext());
        self.emit_u8(0xF7);
        self.emit_u8(Self::modrm(0b11, 3, r.low3()));
    }

    /// `cqo` (sign-extend RAX into RDX:RAX before `idiv`)
    pub fn cqo(&mut self) {
        self.emit_u8(0x48);
        self.emit_u8(0x99);
    }

    /// `idiv r` (signed divide RDX:RAX by `r`; quotient in RAX, remainder in RDX)
    pub fn idiv_r(&mut self, r: Reg) {
        self.emit_rex(true, false, false, r.is_ext());
        self.emit_u8(0xF7);
        self.emit_u8(Self::modrm(0b11, 7, r.low3()));
    }

    /// `sub rsp, imm32`
    pub fn sub_rsp_i32(&mut self, imm: i32) {
        self.emit_u8(0x48);
        self.emit_u8(0x81);
        self.emit_u8(Self::modrm(0b11, 5, Reg::Rsp.low3()));
        self.emit_i32(imm);
    }

    /// `add rsp, imm32`
    pub fn add_rsp_i32(&mut self, imm: i32) {
        self.emit_u8(0x48);
        self.emit_u8(0x81);
        self.emit_u8(Self::modrm(0b11, 0, Reg::Rsp.low3()));
        self.emit_i32(imm);
    }

    // --- Comparisons & Booleans ---

    /// `cmp lhs, rhs`
    pub fn cmp_rr(&mut self, lhs: Reg, rhs: Reg) {
        self.emit_rex(true, rhs.is_ext(), false, lhs.is_ext());
        self.emit_u8(0x39);
        self.emit_u8(Self::modrm(0b11, rhs.low3(), lhs.low3()));
    }

    /// `test a, b`
    pub fn test_rr(&mut self, a: Reg, b: Reg) {
        self.emit_rex(true, b.is_ext(), false, a.is_ext());
        self.emit_u8(0x85);
        self.emit_u8(Self::modrm(0b11, b.low3(), a.low3()));
    }

    /// `cmp r, imm8` (sign-extended)
    pub fn cmp_ri8(&mut self, r: Reg, imm: i8) {
        self.emit_rex(true, false, false, r.is_ext());
        self.emit_u8(0x83);
        self.emit_u8(Self::modrm(0b11, 7, r.low3()));
        self.emit_u8(imm as u8);
    }

    /// `setcc al; movzx dst, al` -> sets `dst` to 1 if `cond` holds, else 0.
    pub fn setcc_bool(&mut self, cond: Cond, dst: Reg) {
        // setcc al (0F 90+cc C0)
        self.emit_u8(0x0F);
        self.emit_u8(0x90 + (cond as u8));
        self.emit_u8(0xC0);
        // movzx dst, al (REX.W 0F B6 /r)
        self.emit_rex(true, dst.is_ext(), false, false);
        self.emit_u8(0x0F);
        self.emit_u8(0xB6);
        self.emit_u8(Self::modrm(0b11, dst.low3(), Reg::Rax.low3()));
    }

    // --- Jumps & Calls ---

    /// `jmp rel32` to `target`
    pub fn jmp_label(&mut self, target: Label) {
        self.emit_u8(0xE9);
        let patch_offset = self.code.len();
        self.emit_i32(0);
        self.fixups.push(Fixup {
            patch_offset,
            target,
        });
    }

    /// `jcc rel32` to `target`
    pub fn jcc_label(&mut self, cond: Cond, target: Label) {
        self.emit_u8(0x0F);
        self.emit_u8(0x80 + (cond as u8));
        let patch_offset = self.code.len();
        self.emit_i32(0);
        self.fixups.push(Fixup {
            patch_offset,
            target,
        });
    }

    /// `call rel32` to `target`
    pub fn call_label(&mut self, target: Label) {
        self.emit_u8(0xE8);
        let patch_offset = self.code.len();
        self.emit_i32(0);
        self.fixups.push(Fixup {
            patch_offset,
            target,
        });
    }

    /// Patch all label fixups and return the finalized machine code bytes.
    pub fn finalize(mut self) -> Result<Vec<u8>, JitError> {
        for fixup in &self.fixups {
            let target_off = self.labels[fixup.target.0].ok_or_else(|| {
                JitError::new("B-baka! Unbound JIT label during machine code finalization! >.<")
            })?;
            let next_rip = fixup.patch_offset + 4;
            let rel = (target_off as isize) - (next_rip as isize);
            let rel32 = i32::try_from(rel).map_err(|_| {
                JitError::new("Hmph! JIT jump offset overflowed 32-bit displacement! >///<")
            })?;
            self.code[fixup.patch_offset..fixup.patch_offset + 4]
                .copy_from_slice(&rel32.to_le_bytes());
        }
        Ok(self.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ExecutableBuffer;

    #[test]
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    fn test_asm_native_add_and_loop_factorial() {
        // Build: extern "C" fn(a: i64, b: i64) -> i64 { a + b * 2 }
        let mut a = Assembler::new();
        // SysV ABI: rdi = a, rsi = b
        a.mov_rr(Reg::Rax, Reg::Rsi);
        a.add_rr(Reg::Rax, Reg::Rsi); // 2 * b
        a.add_rr(Reg::Rax, Reg::Rdi); // a + 2 * b
        a.ret();
        let bytes = a.finalize().unwrap();
        let buf = ExecutableBuffer::from_code(&bytes).unwrap();
        let f: extern "C" fn(i64, i64) -> i64 = unsafe { std::mem::transmute(buf.as_ptr()) };
        assert_eq!(f(10, 7), 24);

        // Build iterative factorial(n: i64) -> i64 using labels & jcc
        let mut b = Assembler::new();
        let loop_lbl = b.create_label();
        let done_lbl = b.create_label();

        b.mov_ri64(Reg::Rax, 1); // acc = 1
        b.bind_label(loop_lbl);
        b.cmp_ri8(Reg::Rdi, 1);
        b.jcc_label(Cond::Le, done_lbl);
        b.imul_rr(Reg::Rax, Reg::Rdi);
        b.mov_ri64(Reg::Rcx, 1);
        b.sub_rr(Reg::Rdi, Reg::Rcx);
        b.jmp_label(loop_lbl);
        b.bind_label(done_lbl);
        b.ret();

        let fact_bytes = b.finalize().unwrap();
        let fact_buf = ExecutableBuffer::from_code(&fact_bytes).unwrap();
        let fact: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(fact_buf.as_ptr()) };
        assert_eq!(fact(1), 1);
        assert_eq!(fact(5), 120);
        assert_eq!(fact(10), 3_628_800);
    }
}
