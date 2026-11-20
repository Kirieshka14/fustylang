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

    pub fn compile_program(mut self, program: &Program) -> Result<Chunk, CompileError> {
        let mut fn_decls: Vec<&FnDecl> = Vec::new();

        // Pass 1: Register all function signatures so forward/recursive calls resolve.
        for item in &program.items {
            if let Item::Fn(fn_decl) = item {
                if self.fn_indices.contains_key(&fn_decl.name) {
                    return Err(CompileError {
                        message: format!("Duplicate function definition '{}'", fn_decl.name),
                    });
                }
                let idx = self.chunk.functions.len();
                if idx > u16::MAX as usize {
                    return Err(CompileError {
                        message: "Too many functions in chunk".into(),
                    });
                }
                if fn_decl.params.len() > u8::MAX as usize {
                    return Err(CompileError {
                        message: format!(
                            "Function '{}' has too many parameters (max 255)",
                            fn_decl.name
                        ),
                    });
                }
                let arity = fn_decl.params.len() as u8;
                self.fn_indices.insert(fn_decl.name.clone(), idx as u16);
                self.fn_arities.insert(fn_decl.name.clone(), arity);
                self.chunk.functions.push(FunctionMeta {
                    name: fn_decl.name.clone(),
                    entry_ip: 0,
                    arity,
                    is_meow: fn_decl.is_meow,
                });
                fn_decls.push(fn_decl);
            }
        }

        // Pass 2: Compile top-level statements (top-level `let` populates globals) and halt before function bodies.
        for item in &program.items {
            match item {
                Item::Stmt(stmt) => self.compile_stmt(stmt)?,
                Item::Fn(_) | Item::Struct(_) | Item::Enum(_) | Item::Impl(_) => {}
            }
        }
        self.chunk.write_op(Opcode::Halt, 0);

        // Pass 3: Compile each function body in isolation (with read/write access to globals).
        for (idx, fn_decl) in fn_decls.iter().enumerate() {
            self.chunk.functions[idx].entry_ip = self.chunk.code.len();
            self.compile_fn(fn_decl)?;
        }

        self.chunk.global_count = self.globals.len() as u16;
        Ok(self.chunk)
    }


}
