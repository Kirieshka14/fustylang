use std::collections::HashMap;

use fusty_bytecode::chunk::Chunk;

use crate::analyze::{analyze_function, AnalyzedFunction, DecodedOp};
use crate::asm::{Assembler, Cond, Label, Reg, Xmm};
use crate::memory::ExecutableBuffer;
use crate::{JitError, JitRuntimeErr, JitValueKind};

/// Native x86-64 calling convention for JIT-compiled `i64` functions.
///
/// - `args`: pointer to `arity` contiguous `i64` values
/// - `out_err`: pointer to a `u8` receiving [`JitRuntimeErr`] on failure
/// - `depth`: current recursion depth (capped at 256)
pub type JitIntFn = extern "C" fn(args: *const i64, out_err: *mut u8, depth: usize) -> i64;

/// Native x86-64 calling convention for JIT-compiled `f64` functions.
pub type JitFloatFn = extern "C" fn(args: *const f64, out_err: *mut u8, depth: usize) -> f64;

/// A compiled native x86-64 function stored in W^X executable memory.
pub struct CompiledFunction {
    pub fn_idx: usize,
    pub name: String,
    pub arity: u8,
    pub kind: JitValueKind,
    buffer: ExecutableBuffer,
}

impl CompiledFunction {
    #[inline]
    pub fn code_size(&self) -> usize {
        self.buffer.len()
    }

    /// Invokes an `Int64`-specialized native function.
    pub fn call_i64(&self, args: &[i64]) -> Result<i64, JitRuntimeErr> {
        debug_assert_eq!(self.kind, JitValueKind::Int64);
        debug_assert_eq!(args.len(), self.arity as usize);
        let mut err: u8 = 0;
        let func: JitIntFn = unsafe { std::mem::transmute(self.buffer.as_ptr()) };
        let res = func(args.as_ptr(), &mut err as *mut u8, 0);
        let err_code = JitRuntimeErr::from_u8(err);
        if err_code != JitRuntimeErr::None {
            Err(err_code)
        } else {
            Ok(res)
        }
    }

    /// Invokes a `Float64`-specialized native function.
    pub fn call_f64(&self, args: &[f64]) -> Result<f64, JitRuntimeErr> {
        debug_assert_eq!(self.kind, JitValueKind::Float64);
        debug_assert_eq!(args.len(), self.arity as usize);
        let mut err: u8 = 0;
        let func: JitFloatFn = unsafe { std::mem::transmute(self.buffer.as_ptr()) };
        let res = func(args.as_ptr(), &mut err as *mut u8, 0);
        let err_code = JitRuntimeErr::from_u8(err);
        if err_code != JitRuntimeErr::None {
            Err(err_code)
        } else {
            Ok(res)
        }
    }
}

/// Calling convention ABI targeted by the x86-64 JIT backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAbi {
    /// System V AMD64 ABI (Linux & macOS x86-64): args in RDI, RSI, RDX.
    SysV,
    /// Microsoft x64 ABI (Windows x86-64): args in RCX, RDX, R8 + 32-byte shadow space.
    Win64,
}

impl TargetAbi {
    #[inline]
    pub const fn host() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Win64
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::SysV
        }
    }

    #[inline]
    const fn arg0(self) -> Reg {
        match self {
            Self::SysV => Reg::Rdi,
            Self::Win64 => Reg::Rcx,
        }
    }

    #[inline]
    const fn arg1(self) -> Reg {
        match self {
            Self::SysV => Reg::Rsi,
            Self::Win64 => Reg::Rdx,
        }
    }

    #[inline]
    const fn arg2(self) -> Reg {
        match self {
            Self::SysV => Reg::Rdx,
            Self::Win64 => Reg::R8,
        }
    }

    #[inline]
    const fn shadow_space_bytes(self) -> i32 {
        match self {
            Self::SysV => 0,
            Self::Win64 => 32,
        }
    }
}

/// 128 virtual stack slots (1024 bytes) + 8 bytes padding for 16-byte stack alignment.
const VSTACK_FRAME_BYTES: i32 = 1032;
const MAX_JIT_RECURSION_DEPTH: i64 = 256;

/// Emits native x86-64 machine code for `chunk.functions[fn_idx]` targeting `abi`.
pub fn emit_machine_code_for_abi(
    chunk: &Chunk,
    fn_idx: usize,
    kind: JitValueKind,
    abi: TargetAbi,
) -> Result<(AnalyzedFunction, Vec<u8>), JitError> {
    let analyzed = analyze_function(chunk, fn_idx, kind)?;
    let machine_code = match kind {
        JitValueKind::Int64 => emit_i64_function(&analyzed, abi)?,
        JitValueKind::Float64 => emit_f64_function(&analyzed, abi)?,
    };
    Ok((analyzed, machine_code))
}

/// Compiles `chunk.functions[fn_idx]` into native x86-64 machine code for the host OS.
pub fn compile_function(
    chunk: &Chunk,
    fn_idx: usize,
    kind: JitValueKind,
) -> Result<CompiledFunction, JitError> {
    if !crate::is_supported_platform() {
        return Err(JitError::new(
            "Native x86-64 JIT is unsupported on this target",
        ));
    }

    let (analyzed, machine_code) =
        emit_machine_code_for_abi(chunk, fn_idx, kind, TargetAbi::host())?;

    let buffer = ExecutableBuffer::from_code(&machine_code)?;
    Ok(CompiledFunction {
        fn_idx,
        name: analyzed.name,
        arity: analyzed.arity,
        kind,
        buffer,
    })
}

#[inline]
fn emit_push(asm: &mut Assembler, reg: Reg) {
    asm.mov_mr_disp(Reg::R13, 0, reg);
    asm.lea_disp(Reg::R13, Reg::R13, 8);
}

#[inline]
fn emit_pop(asm: &mut Assembler, reg: Reg) {
    asm.lea_disp(Reg::R13, Reg::R13, -8);
    asm.mov_rm_disp(reg, Reg::R13, 0);
}

#[inline]
fn emit_peek(asm: &mut Assembler, reg: Reg) {
    asm.mov_rm_disp(reg, Reg::R13, -8);
}

fn emit_i64_function(func: &AnalyzedFunction, abi: TargetAbi) -> Result<Vec<u8>, JitError> {
    let mut asm = Assembler::new();

    let entry_label = asm.create_label();
    let epilogue_label = asm.create_label();
    let div_zero_label = asm.create_label();
    let mod_zero_label = asm.create_label();
    let stack_overflow_label = asm.create_label();

    // Map bytecode target IPs to assembler labels.
    let mut ip_labels: HashMap<usize, Label> = HashMap::new();
    for &target_ip in &func.jump_targets {
        ip_labels.insert(target_ip, asm.create_label());
    }

    // --- Function Entry & Prologue ---
    asm.bind_label(entry_label);
    asm.push_reg(Reg::Rbp);
    asm.mov_rr(Reg::Rbp, Reg::Rsp);
    asm.push_reg(Reg::R12);
    asm.push_reg(Reg::R13);
    asm.push_reg(Reg::R14);
    asm.push_reg(Reg::R15);
    asm.push_reg(Reg::Rbx);
    asm.sub_rsp_i32(VSTACK_FRAME_BYTES);

    // R12 = base of virtual stack frame (slot 0 at [r12 + 0])
    // R13 = top pointer of virtual stack frame
    // R14 = out_err (*mut u8)
    // R15 = recursion depth
    asm.mov_rr(Reg::R12, Reg::Rsp);
    asm.mov_rr(Reg::R13, Reg::Rsp);
    asm.mov_rr(Reg::R14, abi.arg1());
    asm.mov_rr(Reg::R15, abi.arg2());

    // Guard recursion depth against stack overflow
    asm.mov_ri64(Reg::Rax, MAX_JIT_RECURSION_DEPTH);
    asm.cmp_rr(Reg::R15, Reg::Rax);
    asm.jcc_label(Cond::Ge, stack_overflow_label);

    // Copy `arity` arguments from [arg0 + i*8] into virtual stack slots [r12 + i*8]
    for i in 0..func.arity as i32 {
        asm.mov_rm_disp(Reg::Rax, abi.arg0(), i * 8);
        asm.mov_mr_disp(Reg::R12, i * 8, Reg::Rax);
    }
    if func.arity > 0 {
        asm.lea_disp(Reg::R13, Reg::R12, (func.arity as i32) * 8);
    }

    // --- Bytecode Body Translation ---
    for instr in &func.instrs {
        if let Some(&lbl) = ip_labels.get(&instr.ip) {
            asm.bind_label(lbl);
        }

        match instr.op {
            DecodedOp::ConstInt(v) => {
                asm.mov_ri64(Reg::Rax, v);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::ConstBool(b) => {
                asm.mov_ri64(Reg::Rax, if b { 1 } else { 0 });
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Nil => {
                asm.mov_ri64(Reg::Rax, 0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::ConstFloat(_) => {
                return Err(JitError::new("Unexpected float constant in i64 JIT"));
            }
            DecodedOp::Pop => {
                asm.lea_disp(Reg::R13, Reg::R13, -8);
            }
            DecodedOp::PopUnder(count) => {
                emit_pop(&mut asm, Reg::Rax);
                asm.lea_disp(Reg::R13, Reg::R13, -(count as i32) * 8);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::GetLocal(slot) => {
                asm.mov_rm_disp(Reg::Rax, Reg::R12, (slot as i32) * 8);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::SetLocal(slot) => {
                emit_peek(&mut asm, Reg::Rax);
                asm.mov_mr_disp(Reg::R12, (slot as i32) * 8, Reg::Rax);
            }
            DecodedOp::Add => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.add_rr(Reg::Rax, Reg::Rcx);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Sub => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.sub_rr(Reg::Rax, Reg::Rcx);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Mul => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.imul_rr(Reg::Rax, Reg::Rcx);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Div => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.test_rr(Reg::Rcx, Reg::Rcx);
                asm.jcc_label(Cond::E, div_zero_label);
                asm.cqo();
                asm.idiv_r(Reg::Rcx);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Mod => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.test_rr(Reg::Rcx, Reg::Rcx);
                asm.jcc_label(Cond::E, mod_zero_label);
                asm.cqo();
                asm.idiv_r(Reg::Rcx);
                emit_push(&mut asm, Reg::Rdx);
            }
            DecodedOp::Neg => {
                emit_pop(&mut asm, Reg::Rax);
                asm.neg_r(Reg::Rax);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Not => {
                emit_pop(&mut asm, Reg::Rax);
                asm.test_rr(Reg::Rax, Reg::Rax);
                asm.setcc_bool(Cond::E, Reg::Rax);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Eq => emit_i64_cmp(&mut asm, Cond::E),
            DecodedOp::NotEq => emit_i64_cmp(&mut asm, Cond::Ne),
            DecodedOp::Lt => emit_i64_cmp(&mut asm, Cond::L),
            DecodedOp::LtEq => emit_i64_cmp(&mut asm, Cond::Le),
            DecodedOp::Gt => emit_i64_cmp(&mut asm, Cond::G),
            DecodedOp::GtEq => emit_i64_cmp(&mut asm, Cond::Ge),
            DecodedOp::Jump(target) | DecodedOp::Loop(target) => {
                let lbl = ip_labels[&target];
                asm.jmp_label(lbl);
            }
            DecodedOp::JumpIfFalse(target) => {
                let lbl = ip_labels[&target];
                emit_peek(&mut asm, Reg::Rax);
                asm.test_rr(Reg::Rax, Reg::Rax);
                asm.jcc_label(Cond::E, lbl);
            }
            DecodedOp::SelfCall { argc } => {
                let bytes = (argc as i32) * 8;
                let shadow = abi.shadow_space_bytes();
                asm.lea_disp(Reg::R13, Reg::R13, -bytes);
                asm.mov_rr(abi.arg0(), Reg::R13);
                asm.mov_rr(abi.arg1(), Reg::R14);
                asm.lea_disp(abi.arg2(), Reg::R15, 1);
                if shadow > 0 {
                    asm.sub_rsp_i32(shadow);
                }
                asm.call_label(entry_label);
                if shadow > 0 {
                    asm.add_rsp_i32(shadow);
                }
                asm.cmp_byte_mr_zero(Reg::R14);
                asm.jcc_label(Cond::Ne, epilogue_label);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Return => {
                emit_pop(&mut asm, Reg::Rax);
                asm.jmp_label(epilogue_label);
            }
        }
    }

    // Default return 0 if control reaches end without explicit Return
    asm.mov_ri64(Reg::Rax, 0);
    asm.jmp_label(epilogue_label);

    // --- Runtime Error Handlers ---
    asm.bind_label(div_zero_label);
    asm.mov_byte_mr(Reg::R14, JitRuntimeErr::DivisionByZero as u8);
    asm.mov_ri64(Reg::Rax, 0);
    asm.jmp_label(epilogue_label);

    asm.bind_label(mod_zero_label);
    asm.mov_byte_mr(Reg::R14, JitRuntimeErr::ModuloByZero as u8);
    asm.mov_ri64(Reg::Rax, 0);
    asm.jmp_label(epilogue_label);

    asm.bind_label(stack_overflow_label);
    asm.mov_byte_mr(Reg::R14, JitRuntimeErr::StackOverflow as u8);
    asm.mov_ri64(Reg::Rax, 0);

    // --- Epilogue ---
    asm.bind_label(epilogue_label);
    asm.add_rsp_i32(VSTACK_FRAME_BYTES);
    asm.pop_reg(Reg::Rbx);
    asm.pop_reg(Reg::R15);
    asm.pop_reg(Reg::R14);
    asm.pop_reg(Reg::R13);
    asm.pop_reg(Reg::R12);
    asm.pop_reg(Reg::Rbp);
    asm.ret();

    asm.finalize()
}

fn emit_i64_cmp(asm: &mut Assembler, cond: Cond) {
    emit_pop(asm, Reg::Rcx);
    emit_pop(asm, Reg::Rax);
    asm.cmp_rr(Reg::Rax, Reg::Rcx);
    asm.setcc_bool(cond, Reg::Rax);
    emit_push(asm, Reg::Rax);
}

fn emit_f64_cmp(asm: &mut Assembler, cond: Cond) {
    emit_pop(asm, Reg::Rcx); // rhs bits
    emit_pop(asm, Reg::Rax); // lhs bits
    asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
    asm.movq_xmm_r(Xmm::Xmm1, Reg::Rcx);
    asm.ucomisd(Xmm::Xmm0, Xmm::Xmm1);
    asm.setcc_bool(cond, Reg::Rax);
    asm.cvtsi2sd(Xmm::Xmm0, Reg::Rax);
    asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
    emit_push(asm, Reg::Rax);
}

fn emit_f64_function(func: &AnalyzedFunction, abi: TargetAbi) -> Result<Vec<u8>, JitError> {
    let mut asm = Assembler::new();

    let entry_label = asm.create_label();
    let epilogue_label = asm.create_label();
    let div_zero_label = asm.create_label();
    let stack_overflow_label = asm.create_label();

    let mut ip_labels: HashMap<usize, Label> = HashMap::new();
    for &target_ip in &func.jump_targets {
        ip_labels.insert(target_ip, asm.create_label());
    }

    // --- Prologue ---
    asm.bind_label(entry_label);
    asm.push_reg(Reg::Rbp);
    asm.mov_rr(Reg::Rbp, Reg::Rsp);
    asm.push_reg(Reg::R12);
    asm.push_reg(Reg::R13);
    asm.push_reg(Reg::R14);
    asm.push_reg(Reg::R15);
    asm.push_reg(Reg::Rbx);
    asm.sub_rsp_i32(VSTACK_FRAME_BYTES);

    asm.mov_rr(Reg::R12, Reg::Rsp);
    asm.mov_rr(Reg::R13, Reg::Rsp);
    asm.mov_rr(Reg::R14, abi.arg1());
    asm.mov_rr(Reg::R15, abi.arg2());

    asm.mov_ri64(Reg::Rax, MAX_JIT_RECURSION_DEPTH);
    asm.cmp_rr(Reg::R15, Reg::Rax);
    asm.jcc_label(Cond::Ge, stack_overflow_label);

    for i in 0..func.arity as i32 {
        asm.mov_rm_disp(Reg::Rax, abi.arg0(), i * 8);
        asm.mov_mr_disp(Reg::R12, i * 8, Reg::Rax);
    }
    if func.arity > 0 {
        asm.lea_disp(Reg::R13, Reg::R12, (func.arity as i32) * 8);
    }

    // --- Body ---
    for instr in &func.instrs {
        if let Some(&lbl) = ip_labels.get(&instr.ip) {
            asm.bind_label(lbl);
        }

        match instr.op {
            DecodedOp::ConstFloat(v) => {
                asm.mov_ri64(Reg::Rax, v.to_bits() as i64);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::ConstInt(v) => {
                asm.mov_ri64(Reg::Rax, (v as f64).to_bits() as i64);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::ConstBool(b) => {
                let bits = if b {
                    1.0f64.to_bits()
                } else {
                    0.0f64.to_bits()
                };
                asm.mov_ri64(Reg::Rax, bits as i64);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Nil => {
                asm.mov_ri64(Reg::Rax, 0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Pop => {
                asm.lea_disp(Reg::R13, Reg::R13, -8);
            }
            DecodedOp::PopUnder(count) => {
                emit_pop(&mut asm, Reg::Rax);
                asm.lea_disp(Reg::R13, Reg::R13, -(count as i32) * 8);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::GetLocal(slot) => {
                asm.mov_rm_disp(Reg::Rax, Reg::R12, (slot as i32) * 8);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::SetLocal(slot) => {
                emit_peek(&mut asm, Reg::Rax);
                asm.mov_mr_disp(Reg::R12, (slot as i32) * 8, Reg::Rax);
            }
            DecodedOp::Add => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm1, Reg::Rcx);
                asm.addsd(Xmm::Xmm0, Xmm::Xmm1);
                asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Sub => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm1, Reg::Rcx);
                asm.subsd(Xmm::Xmm0, Xmm::Xmm1);
                asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Mul => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm1, Reg::Rcx);
                asm.mulsd(Xmm::Xmm0, Xmm::Xmm1);
                asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Div => {
                emit_pop(&mut asm, Reg::Rcx);
                emit_pop(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm1, Reg::Rcx);
                asm.xorpd(Xmm::Xmm2, Xmm::Xmm2);
                asm.ucomisd(Xmm::Xmm1, Xmm::Xmm2);
                asm.jcc_label(Cond::E, div_zero_label);
                asm.divsd(Xmm::Xmm0, Xmm::Xmm1);
                asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Mod => {
                return Err(JitError::new("Float modulo is handled by VM fallback"));
            }
            DecodedOp::Neg => {
                emit_pop(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.mov_ri64(Reg::Rcx, i64::MIN); // sign bit 0x8000_0000_0000_0000
                asm.movq_xmm_r(Xmm::Xmm1, Reg::Rcx);
                asm.xorpd(Xmm::Xmm0, Xmm::Xmm1);
                asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Not => {
                emit_pop(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.xorpd(Xmm::Xmm1, Xmm::Xmm1);
                asm.ucomisd(Xmm::Xmm0, Xmm::Xmm1);
                asm.setcc_bool(Cond::E, Reg::Rax);
                asm.cvtsi2sd(Xmm::Xmm0, Reg::Rax);
                asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Eq => emit_f64_cmp(&mut asm, Cond::E),
            DecodedOp::NotEq => emit_f64_cmp(&mut asm, Cond::Ne),
            DecodedOp::Lt => emit_f64_cmp(&mut asm, Cond::B),
            DecodedOp::LtEq => emit_f64_cmp(&mut asm, Cond::Be),
            DecodedOp::Gt => emit_f64_cmp(&mut asm, Cond::A),
            DecodedOp::GtEq => emit_f64_cmp(&mut asm, Cond::Ae),
            DecodedOp::Jump(target) | DecodedOp::Loop(target) => {
                let lbl = ip_labels[&target];
                asm.jmp_label(lbl);
            }
            DecodedOp::JumpIfFalse(target) => {
                let lbl = ip_labels[&target];
                emit_peek(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.xorpd(Xmm::Xmm1, Xmm::Xmm1);
                asm.ucomisd(Xmm::Xmm0, Xmm::Xmm1);
                asm.jcc_label(Cond::E, lbl);
            }
            DecodedOp::SelfCall { argc } => {
                let bytes = (argc as i32) * 8;
                let shadow = abi.shadow_space_bytes();
                asm.lea_disp(Reg::R13, Reg::R13, -bytes);
                asm.mov_rr(abi.arg0(), Reg::R13);
                asm.mov_rr(abi.arg1(), Reg::R14);
                asm.lea_disp(abi.arg2(), Reg::R15, 1);
                if shadow > 0 {
                    asm.sub_rsp_i32(shadow);
                }
                asm.call_label(entry_label);
                if shadow > 0 {
                    asm.add_rsp_i32(shadow);
                }
                asm.cmp_byte_mr_zero(Reg::R14);
                asm.jcc_label(Cond::Ne, epilogue_label);
                asm.movq_r_xmm(Reg::Rax, Xmm::Xmm0);
                emit_push(&mut asm, Reg::Rax);
            }
            DecodedOp::Return => {
                emit_pop(&mut asm, Reg::Rax);
                asm.movq_xmm_r(Xmm::Xmm0, Reg::Rax);
                asm.jmp_label(epilogue_label);
            }
        }
    }

    asm.xorpd(Xmm::Xmm0, Xmm::Xmm0);
    asm.jmp_label(epilogue_label);

    asm.bind_label(div_zero_label);
    asm.mov_byte_mr(Reg::R14, JitRuntimeErr::DivisionByZero as u8);
    asm.xorpd(Xmm::Xmm0, Xmm::Xmm0);
    asm.jmp_label(epilogue_label);

    asm.bind_label(stack_overflow_label);
    asm.mov_byte_mr(Reg::R14, JitRuntimeErr::StackOverflow as u8);
    asm.xorpd(Xmm::Xmm0, Xmm::Xmm0);

    // --- Epilogue ---
    asm.bind_label(epilogue_label);
    asm.add_rsp_i32(VSTACK_FRAME_BYTES);
    asm.pop_reg(Reg::Rbx);
    asm.pop_reg(Reg::R15);
    asm.pop_reg(Reg::R14);
    asm.pop_reg(Reg::R13);
    asm.pop_reg(Reg::R12);
    asm.pop_reg(Reg::Rbp);
    asm.ret();

    asm.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fusty_bytecode::compiler::Compiler;
    use fusty_lexer::Lexer;
    use fusty_parser::Parser;

    fn compile_chunk(src: &str) -> Chunk {
        let tokens = Lexer::new(src).tokenize().expect("lex");
        let program = Parser::new(tokens).parse_program().expect("parse");
        Compiler::new().compile_program(&program).expect("compile")
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn test_jit_compile_i64_loop_and_recursive_fib() {
        let src = r#"
            nya sum_to(n: i64) -> i64 {
                let mut acc = 0;
                let mut i = 1;
                while i <= n {
                    acc += i;
                    i += 1;
                }
                return acc;
            }

            nya fib(n: i64) -> i64 {
                if n <= 1 {
                    return n;
                }
                return fib(n - 1) + fib(n - 2);
            }
        "#;
        let chunk = compile_chunk(src);

        let sum_fn = compile_function(&chunk, 0, JitValueKind::Int64).expect("jit sum_to");
        assert_eq!(sum_fn.call_i64(&[100]).unwrap(), 5050);
        assert_eq!(sum_fn.call_i64(&[1000]).unwrap(), 500_500);

        let fib_fn = compile_function(&chunk, 1, JitValueKind::Int64).expect("jit fib");
        assert_eq!(fib_fn.call_i64(&[0]).unwrap(), 0);
        assert_eq!(fib_fn.call_i64(&[1]).unwrap(), 1);
        assert_eq!(fib_fn.call_i64(&[10]).unwrap(), 55);
        assert_eq!(fib_fn.call_i64(&[20]).unwrap(), 6765);
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn test_jit_i64_div_zero_and_stack_overflow_safe() {
        let src = r#"
            nya safe_div(a: i64, b: i64) -> i64 {
                return a / b;
            }
            nya boom(n: i64) -> i64 {
                return boom(n + 1);
            }
        "#;
        let chunk = compile_chunk(src);

        let div_fn = compile_function(&chunk, 0, JitValueKind::Int64).expect("jit safe_div");
        assert_eq!(div_fn.call_i64(&[42, 6]).unwrap(), 7);
        assert_eq!(
            div_fn.call_i64(&[42, 0]).unwrap_err(),
            JitRuntimeErr::DivisionByZero
        );

        let boom_fn = compile_function(&chunk, 1, JitValueKind::Int64).expect("jit boom");
        assert_eq!(
            boom_fn.call_i64(&[0]).unwrap_err(),
            JitRuntimeErr::StackOverflow
        );
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn test_jit_compile_f64_sse2_physics_and_div_zero() {
        let src = r#"
            nya integrate(v0: f64, acc: f64, steps: f64) -> f64 {
                let mut v = v0;
                let mut i = 0.0;
                while i < steps {
                    v = v + acc * 0.5;
                    i = i + 1.0;
                }
                return -(-v);
            }

            nya ratio(a: f64, b: f64) -> f64 {
                return a / b;
            }
        "#;
        let chunk = compile_chunk(src);

        let f = compile_function(&chunk, 0, JitValueKind::Float64).expect("jit integrate");
        let out = f.call_f64(&[10.0, 4.0, 5.0]).unwrap();
        assert!((out - 20.0).abs() < 1e-12);

        let r = compile_function(&chunk, 1, JitValueKind::Float64).expect("jit ratio");
        assert!((r.call_f64(&[9.0, 2.0]).unwrap() - 4.5).abs() < 1e-12);
        assert_eq!(
            r.call_f64(&[9.0, 0.0]).unwrap_err(),
            JitRuntimeErr::DivisionByZero
        );
    }

    #[test]
    fn test_sysv_and_win64_abi_codegen() {
        let src = r#"
            nya fib(n: i64) -> i64 {
                if n <= 1 { return n; }
                return fib(n - 1) + fib(n - 2);
            }
        "#;
        let chunk = compile_chunk(src);
        let (_, sysv_code) =
            emit_machine_code_for_abi(&chunk, 0, JitValueKind::Int64, TargetAbi::SysV)
                .expect("SysV codegen");
        let (_, win64_code) =
            emit_machine_code_for_abi(&chunk, 0, JitValueKind::Int64, TargetAbi::Win64)
                .expect("Win64 codegen");
        assert!(!sysv_code.is_empty());
        // Win64 reserves 32-byte shadow space around recursive self-calls, so it emits more bytes.
        assert!(win64_code.len() > sysv_code.len());
    }
}
