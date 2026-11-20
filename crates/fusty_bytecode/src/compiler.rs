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

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, line: usize) {
        self.scope_depth -= 1;
        while let Some(local) = self.locals.last() {
            if local.depth > self.scope_depth {
                self.chunk.write_op(Opcode::Pop, line);
                self.locals.pop();
            } else {
                break;
            }
        }
    }

    fn end_scope_keeping_top(&mut self, line: usize) -> Result<(), CompileError> {
        self.scope_depth -= 1;
        let mut count = 0usize;
        while let Some(local) = self.locals.last() {
            if local.depth > self.scope_depth {
                self.locals.pop();
                count += 1;
            } else {
                break;
            }
        }
        if count > 0 {
            if count > u16::MAX as usize {
                return Err(CompileError {
                    message: "Too many locals in block".into(),
                });
            }
            self.chunk.write_op(Opcode::PopUnder, line);
            self.chunk.write_u16(count as u16, line);
        }
        Ok(())
    }

    fn resolve_var(&self, name: &str) -> Option<VarBinding> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(VarBinding::Local {
                    slot: i as u16,
                    is_mut: local.is_mut,
                });
            }
        }
        for global in self.globals.iter().rev() {
            if global.name == name {
                return Some(VarBinding::Global {
                    slot: global.slot,
                    is_mut: global.is_mut,
                });
            }
        }
        None
    }

    fn emit_const(&mut self, constant: Constant, line: usize) -> Result<(), CompileError> {
        let idx = self
            .chunk
            .try_add_constant(constant)
            .map_err(|message| CompileError { message })?;
        self.chunk.write_op(Opcode::Const, line);
        self.chunk.write_u16(idx, line);
        Ok(())
    }

    fn compile_block_value(&mut self, block: &Block) -> Result<(), CompileError> {
        let line = block.span.line;
        self.begin_scope();
        for s in &block.stmts {
            self.compile_stmt(s)?;
        }
        if let Some(expr) = &block.expr {
            self.compile_expr(expr)?;
        } else {
            self.chunk.write_op(Opcode::Nil, line);
        }
        self.end_scope_keeping_top(line)?;
        Ok(())
    }

    fn compile_fn(&mut self, fn_decl: &FnDecl) -> Result<(), CompileError> {
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_depth = self.scope_depth;
        let saved_loops = std::mem::take(&mut self.loop_stack);
        let saved_meow = self.meow_depth;

        self.scope_depth = 1;
        self.meow_depth = 0;

        for param in &fn_decl.params {
            self.locals.push(Local {
                name: param.name.clone(),
                depth: self.scope_depth,
                is_mut: param.is_mut,
            });
        }

        if fn_decl.is_meow {
            self.chunk.write_op(Opcode::MeowEnter, fn_decl.span.line);
            self.meow_depth += 1;
        }

        for stmt in &fn_decl.body.stmts {
            self.compile_stmt(stmt)?;
        }

        if let Some(expr) = &fn_decl.body.expr {
            self.compile_expr(expr)?;
        } else {
            self.chunk.write_op(Opcode::Nil, fn_decl.span.line);
        }

        if fn_decl.is_meow {
            self.meow_depth -= 1;
            self.chunk.write_op(Opcode::MeowExit, fn_decl.span.line);
        }

        self.chunk.write_op(Opcode::Return, fn_decl.span.line);

        self.locals = saved_locals;
        self.scope_depth = saved_depth;
        self.loop_stack = saved_loops;
        self.meow_depth = saved_meow;
        Ok(())
    }


}
