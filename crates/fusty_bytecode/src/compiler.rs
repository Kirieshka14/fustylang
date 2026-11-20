use std::collections::HashMap;

use crate::chunk::{Chunk, Constant, FunctionMeta};
use crate::opcode::Opcode;
use fusty_parser::ast::*;

pub struct Local {
    pub name: String,
    pub depth: usize,
    pub is_mut: bool,
}

pub struct Global {
    pub name: String,
    pub slot: u16,
    pub is_mut: bool,
}

struct LoopContext {
    loop_start: usize,
    local_count: usize,
    meow_depth: usize,
    break_jumps: Vec<usize>,
}

enum VarBinding {
    Local { slot: u16, is_mut: bool },
    Global { slot: u16, is_mut: bool },
}

pub struct Compiler {
    pub chunk: Chunk,
    pub locals: Vec<Local>,
    pub globals: Vec<Global>,
    pub scope_depth: usize,
    pub fn_indices: HashMap<String, u16>,
    fn_arities: HashMap<String, u8>,
    loop_stack: Vec<LoopContext>,
    meow_depth: usize,
}

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            locals: Vec::new(),
            globals: Vec::new(),
            scope_depth: 0,
            fn_indices: HashMap::new(),
            fn_arities: HashMap::new(),
            loop_stack: Vec::new(),
            meow_depth: 0,
        }
    }


}
