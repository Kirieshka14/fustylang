use std::collections::HashSet;

use fusty_bytecode::chunk::{Chunk, Constant};
use fusty_bytecode::opcode::Opcode;

use crate::{JitError, JitValueKind};

/// A decoded bytecode instruction inside a single `nya` function body.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedInstr {
    pub ip: usize,
    pub op: DecodedOp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodedOp {
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    Nil,
    Pop,
    PopUnder(u16),
    GetLocal(u16),
    SetLocal(u16),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Not,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Jump(usize),
    JumpIfFalse(usize),
    Loop(usize),
    SelfCall { argc: u8 },
    Return,
}

/// Verified compilation plan for a single `nya` function.
#[derive(Debug, Clone)]
pub struct AnalyzedFunction {
    pub fn_idx: usize,
    pub name: String,
    pub arity: u8,
    pub kind: JitValueKind,
    pub is_recursive: bool,
    pub instrs: Vec<DecodedInstr>,
    pub jump_targets: HashSet<usize>,
}

/// Inspects bytecode of `chunk.functions[fn_idx]` and verifies whether it can be
/// compiled natively for `kind` (`Int64` or `Float64`).
pub fn analyze_function(
    chunk: &Chunk,
    fn_idx: usize,
    kind: JitValueKind,
) -> Result<AnalyzedFunction, JitError> {
    let meta = chunk
        .functions
        .get(fn_idx)
        .ok_or_else(|| JitError::new(format!("Invalid function index {}", fn_idx)))?;

    if meta.is_meow {
        return Err(JitError::new(
            "meow functions stay in VM to preserve meow_depth tracking",
        ));
    }
    if meta.arity > 16 {
        return Err(JitError::new("JIT supports up to 16 parameters"));
    }
    if meta.entry_ip >= meta.end_ip || meta.end_ip > chunk.code.len() {
        return Err(JitError::new("Invalid function bytecode bounds"));
    }

    let code = &chunk.code;
    let mut ip = meta.entry_ip;
    let mut instrs = Vec::new();
    let mut jump_targets = HashSet::new();
    let mut is_recursive = false;

    while ip < meta.end_ip {
        let op_ip = ip;
        let byte = code[ip];
        ip += 1;

        let op = Opcode::from_u8(byte)
            .ok_or_else(|| JitError::new(format!("Unknown opcode byte 0x{:02X}", byte)))?;

        let decoded = match op {
            Opcode::Const => {
                let idx = read_u16(code, &mut ip, meta.end_ip)? as usize;
                let c = chunk
                    .constants
                    .get(idx)
                    .ok_or_else(|| JitError::new("Constant index out of bounds"))?;
                match (kind, c) {
                    (JitValueKind::Int64, Constant::Int(v)) => DecodedOp::ConstInt(*v),
                    (JitValueKind::Float64, Constant::Float(v)) => DecodedOp::ConstFloat(*v),
                    (JitValueKind::Float64, Constant::Int(v)) => DecodedOp::ConstFloat(*v as f64),
                    (_, Constant::Bool(b)) => DecodedOp::ConstBool(*b),
                    _ => {
                        return Err(JitError::new(format!(
                            "Constant {:?} incompatible with {:?} JIT specialization",
                            c, kind
                        )))
                    }
                }
            }
            Opcode::Nil => DecodedOp::Nil,
            Opcode::True => DecodedOp::ConstBool(true),
            Opcode::False => DecodedOp::ConstBool(false),
            Opcode::Pop => DecodedOp::Pop,
            Opcode::PopUnder => {
                let count = read_u16(code, &mut ip, meta.end_ip)?;
                DecodedOp::PopUnder(count)
            }
            Opcode::GetLocal => {
                let slot = read_u16(code, &mut ip, meta.end_ip)?;
                DecodedOp::GetLocal(slot)
            }
            Opcode::SetLocal => {
                let slot = read_u16(code, &mut ip, meta.end_ip)?;
                DecodedOp::SetLocal(slot)
            }
            Opcode::Add => DecodedOp::Add,
            Opcode::Sub => DecodedOp::Sub,
            Opcode::Mul => DecodedOp::Mul,
            Opcode::Div => DecodedOp::Div,
            Opcode::Mod => {
                if kind == JitValueKind::Float64 {
                    return Err(JitError::new(
                        "Float modulo falls back to VM for exact IEEE fmod",
                    ));
                }
                DecodedOp::Mod
            }
            Opcode::Neg => DecodedOp::Neg,
            Opcode::Not => DecodedOp::Not,
            Opcode::Eq => DecodedOp::Eq,
            Opcode::NotEq => DecodedOp::NotEq,
            Opcode::Lt => DecodedOp::Lt,
            Opcode::LtEq => DecodedOp::LtEq,
            Opcode::Gt => DecodedOp::Gt,
            Opcode::GtEq => DecodedOp::GtEq,
            Opcode::Jump => {
                let offset = read_u16(code, &mut ip, meta.end_ip)? as usize;
                let target = ip + offset;
                if target < meta.entry_ip || target >= meta.end_ip {
                    return Err(JitError::new("Jump target outside function bounds"));
                }
                jump_targets.insert(target);
                DecodedOp::Jump(target)
            }
            Opcode::JumpIfFalse => {
                let offset = read_u16(code, &mut ip, meta.end_ip)? as usize;
                let target = ip + offset;
                if target < meta.entry_ip || target >= meta.end_ip {
                    return Err(JitError::new("JumpIfFalse target outside function bounds"));
                }
                jump_targets.insert(target);
                DecodedOp::JumpIfFalse(target)
            }
            Opcode::Loop => {
                let offset = read_u16(code, &mut ip, meta.end_ip)? as usize;
                if offset > ip {
                    return Err(JitError::new("Loop offset underflow"));
                }
                let target = ip - offset;
                if target < meta.entry_ip || target >= meta.end_ip {
                    return Err(JitError::new("Loop target outside function bounds"));
                }
                jump_targets.insert(target);
                DecodedOp::Loop(target)
            }
            Opcode::Call => {
                let callee = read_u16(code, &mut ip, meta.end_ip)? as usize;
                let argc = read_u8(code, &mut ip, meta.end_ip)?;
                if callee != fn_idx || argc != meta.arity {
                    return Err(JitError::new(
                        "Cross-function calls stay in VM; self-recursion is JIT-compiled",
                    ));
                }
                is_recursive = true;
                DecodedOp::SelfCall { argc }
            }
            Opcode::Return => DecodedOp::Return,
            // Coroutines, vectors, globals, natives, meow, and baka! stay in VM.
            other => {
                return Err(JitError::new(format!(
                    "Opcode {:?} requires VM execution (clean fallback)",
                    other
                )))
            }
        };

        instrs.push(DecodedInstr {
            ip: op_ip,
            op: decoded,
        });
    }

    Ok(AnalyzedFunction {
        fn_idx,
        name: meta.name.clone(),
        arity: meta.arity,
        kind,
        is_recursive,
        instrs,
        jump_targets,
    })
}

fn read_u8(code: &[u8], ip: &mut usize, end: usize) -> Result<u8, JitError> {
    if *ip >= end {
        return Err(JitError::new("Unexpected end of function bytecode"));
    }
    let b = code[*ip];
    *ip += 1;
    Ok(b)
}

fn read_u16(code: &[u8], ip: &mut usize, end: usize) -> Result<u16, JitError> {
    let hi = read_u8(code, ip, end)? as u16;
    let lo = read_u8(code, ip, end)? as u16;
    Ok((hi << 8) | lo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fusty_bytecode::chunk::{Chunk, FunctionMeta};

    #[test]
    fn test_analyze_pure_i64_function() {
        let mut chunk = Chunk::new();
        let entry = chunk.code.len();
        chunk.write_op(Opcode::GetLocal, 1);
        chunk.write_u16(0, 1);
        chunk.write_const(Constant::Int(10), 1);
        chunk.write_op(Opcode::Add, 1);
        chunk.write_op(Opcode::Return, 1);
        let end = chunk.code.len();
        chunk.functions.push(FunctionMeta {
            name: "add10".into(),
            entry_ip: entry,
            end_ip: end,
            arity: 1,
            is_meow: false,
        });

        let analyzed = analyze_function(&chunk, 0, JitValueKind::Int64).expect("eligible");
        assert_eq!(analyzed.name, "add10");
        assert!(!analyzed.is_recursive);
        assert_eq!(analyzed.instrs.len(), 4);
    }

    #[test]
    fn test_analyze_rejects_baka_and_wait() {
        let mut chunk = Chunk::new();
        let entry = chunk.code.len();
        chunk.write_const(Constant::String("nope".into()), 1);
        chunk.write_op(Opcode::Baka, 1);
        let end = chunk.code.len();
        chunk.functions.push(FunctionMeta {
            name: "panic_fn".into(),
            entry_ip: entry,
            end_ip: end,
            arity: 0,
            is_meow: false,
        });

        assert!(analyze_function(&chunk, 0, JitValueKind::Int64).is_err());
    }
}
