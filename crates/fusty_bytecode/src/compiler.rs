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

    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::Let {
                name,
                is_mut,
                init,
                span,
                ..
            } => {
                let line = span.line;
                if let Some(init_expr) = init {
                    self.compile_expr(init_expr)?;
                } else {
                    self.chunk.write_op(Opcode::Nil, line);
                }

                if self.scope_depth == 0 {
                    // Top-level binding stored in globals table so functions can access it.
                    if self.globals.len() >= u16::MAX as usize {
                        return Err(CompileError {
                            message: "Too many global variables (max 65535)".into(),
                        });
                    }
                    let slot = self.globals.len() as u16;
                    self.globals.push(Global {
                        name: name.clone(),
                        slot,
                        is_mut: *is_mut,
                    });
                    self.chunk.write_op(Opcode::SetGlobal, line);
                    self.chunk.write_u16(slot, line);
                    self.chunk.write_op(Opcode::Pop, line);
                } else {
                    if self.locals.len() >= u16::MAX as usize {
                        return Err(CompileError {
                            message: "Too many local variables (max 65535)".into(),
                        });
                    }
                    self.locals.push(Local {
                        name: name.clone(),
                        depth: self.scope_depth,
                        is_mut: *is_mut,
                    });
                }
            }

            Stmt::Assign { target, value, span } => {
                let line = span.line;
                match target {
                    Expr::Ident(name, _) => {
                        let binding = self.resolve_var(name).ok_or_else(|| CompileError {
                            message: format!("Undefined variable '{}' in assignment", name),
                        })?;
                        match binding {
                            VarBinding::Local { slot, is_mut } => {
                                if !is_mut {
                                    return Err(CompileError {
                                        message: format!(
                                            "Cannot assign twice to immutable variable '{}' (use `let mut`)",
                                            name
                                        ),
                                    });
                                }
                                self.compile_expr(value)?;
                                self.chunk.write_op(Opcode::SetLocal, line);
                                self.chunk.write_u16(slot, line);
                                self.chunk.write_op(Opcode::Pop, line);
                            }
                            VarBinding::Global { slot, is_mut } => {
                                if !is_mut {
                                    return Err(CompileError {
                                        message: format!(
                                            "Cannot assign twice to immutable variable '{}' (use `let mut`)",
                                            name
                                        ),
                                    });
                                }
                                self.compile_expr(value)?;
                                self.chunk.write_op(Opcode::SetGlobal, line);
                                self.chunk.write_u16(slot, line);
                                self.chunk.write_op(Opcode::Pop, line);
                            }
                        }
                    }
                    Expr::FieldAccess {
                        target: inner,
                        field,
                        ..
                    } => {
                        let Expr::Ident(name, _) = inner.as_ref() else {
                            return Err(CompileError {
                                message: "Field assignment target must be a variable (e.g. `v.x = ...`)"
                                    .into(),
                            });
                        };
                        let set_op = match field.as_str() {
                            "x" => Opcode::VecSetX,
                            "y" => Opcode::VecSetY,
                            "z" => Opcode::VecSetZ,
                            other => {
                                return Err(CompileError {
                                    message: format!("Cannot assign to unknown field '{}'", other),
                                })
                            }
                        };
                        let binding = self.resolve_var(name).ok_or_else(|| CompileError {
                            message: format!("Undefined variable '{}' in field assignment", name),
                        })?;
                        let (get_op, store_op, slot, is_mut) = match binding {
                            VarBinding::Local { slot, is_mut } => {
                                (Opcode::GetLocal, Opcode::SetLocal, slot, is_mut)
                            }
                            VarBinding::Global { slot, is_mut } => {
                                (Opcode::GetGlobal, Opcode::SetGlobal, slot, is_mut)
                            }
                        };
                        if !is_mut {
                            return Err(CompileError {
                                message: format!(
                                    "Cannot mutate field '{}' of immutable variable '{}' (use `let mut`)",
                                    field, name
                                ),
                            });
                        }
                        self.chunk.write_op(get_op, line);
                        self.chunk.write_u16(slot, line);
                        self.compile_expr(value)?;
                        self.chunk.write_op(set_op, line);
                        self.chunk.write_op(store_op, line);
                        self.chunk.write_u16(slot, line);
                        self.chunk.write_op(Opcode::Pop, line);
                    }
                    _ => {
                        return Err(CompileError {
                            message: "Assignment target must be a variable or vector field".into(),
                        });
                    }
                }
            }

            Stmt::CompoundAssign {
                target,
                op,
                value,
                span,
            } => {
                let line = span.line;
                let math_op = match op {
                    BinOp::Add => Opcode::Add,
                    BinOp::Sub => Opcode::Sub,
                    BinOp::Mul => Opcode::Mul,
                    BinOp::Div => Opcode::Div,
                    _ => {
                        return Err(CompileError {
                            message: "Unsupported compound assignment operator".into(),
                        })
                    }
                };
                match target {
                    Expr::Ident(name, _) => {
                        let binding = self.resolve_var(name).ok_or_else(|| CompileError {
                            message: format!("Undefined variable '{}'", name),
                        })?;
                        let (get_op, store_op, slot, is_mut) = match binding {
                            VarBinding::Local { slot, is_mut } => {
                                (Opcode::GetLocal, Opcode::SetLocal, slot, is_mut)
                            }
                            VarBinding::Global { slot, is_mut } => {
                                (Opcode::GetGlobal, Opcode::SetGlobal, slot, is_mut)
                            }
                        };
                        if !is_mut {
                            return Err(CompileError {
                                message: format!(
                                    "Cannot mutate immutable variable '{}' (use `let mut`)",
                                    name
                                ),
                            });
                        }
                        self.chunk.write_op(get_op, line);
                        self.chunk.write_u16(slot, line);
                        self.compile_expr(value)?;
                        self.chunk.write_op(math_op, line);
                        self.chunk.write_op(store_op, line);
                        self.chunk.write_u16(slot, line);
                        self.chunk.write_op(Opcode::Pop, line);
                    }
                    Expr::FieldAccess {
                        target: inner,
                        field,
                        ..
                    } => {
                        let Expr::Ident(name, _) = inner.as_ref() else {
                            return Err(CompileError {
                                message: "Compound field assignment target must be a variable".into(),
                            });
                        };
                        let (get_field_op, set_field_op) = match field.as_str() {
                            "x" => (Opcode::VecGetX, Opcode::VecSetX),
                            "y" => (Opcode::VecGetY, Opcode::VecSetY),
                            "z" => (Opcode::VecGetZ, Opcode::VecSetZ),
                            other => {
                                return Err(CompileError {
                                    message: format!("Unknown field '{}'", other),
                                })
                            }
                        };
                        let binding = self.resolve_var(name).ok_or_else(|| CompileError {
                            message: format!("Undefined variable '{}'", name),
                        })?;
                        let (get_op, store_op, slot, is_mut) = match binding {
                            VarBinding::Local { slot, is_mut } => {
                                (Opcode::GetLocal, Opcode::SetLocal, slot, is_mut)
                            }
                            VarBinding::Global { slot, is_mut } => {
                                (Opcode::GetGlobal, Opcode::SetGlobal, slot, is_mut)
                            }
                        };
                        if !is_mut {
                            return Err(CompileError {
                                message: format!(
                                    "Cannot mutate field '{}' of immutable variable '{}' (use `let mut`)",
                                    field, name
                                ),
                            });
                        }
                        // Push vec, then compute (vec.field op value), then VecSet*, store
                        self.chunk.write_op(get_op, line);
                        self.chunk.write_u16(slot, line);
                        self.chunk.write_op(get_op, line);
                        self.chunk.write_u16(slot, line);
                        self.chunk.write_op(get_field_op, line);
                        self.compile_expr(value)?;
                        self.chunk.write_op(math_op, line);
                        self.chunk.write_op(set_field_op, line);
                        self.chunk.write_op(store_op, line);
                        self.chunk.write_u16(slot, line);
                        self.chunk.write_op(Opcode::Pop, line);
                    }
                    _ => {
                        return Err(CompileError {
                            message: "Compound assignment target must be a variable or vector field"
                                .into(),
                        });
                    }
                }
            }

            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.chunk.write_op(Opcode::Pop, expr.span().line);
            }

            Stmt::Return(expr, span) => {
                let line = span.line;
                if let Some(e) = expr {
                    self.compile_expr(e)?;
                } else {
                    self.chunk.write_op(Opcode::Nil, line);
                }
                self.chunk.write_op(Opcode::Return, line);
            }

            Stmt::Yield(span) => {
                self.chunk.write_op(Opcode::Yield, span.line);
            }

            Stmt::Break(span) => {
                let line = span.line;
                let Some(loop_ctx) = self.loop_stack.last() else {
                    return Err(CompileError {
                        message: "'break' used outside of a loop".into(),
                    });
                };
                let pops = self.locals.len().saturating_sub(loop_ctx.local_count);
                let meow_exits = self.meow_depth.saturating_sub(loop_ctx.meow_depth);
                for _ in 0..meow_exits {
                    self.chunk.write_op(Opcode::MeowExit, line);
                }
                for _ in 0..pops {
                    self.chunk.write_op(Opcode::Pop, line);
                }
                self.chunk.write_op(Opcode::Jump, line);
                let jump = self.emit_jump_placeholder(line);
                self.loop_stack.last_mut().unwrap().break_jumps.push(jump);
            }

            Stmt::Continue(span) => {
                let line = span.line;
                let Some(loop_ctx) = self.loop_stack.last() else {
                    return Err(CompileError {
                        message: "'continue' used outside of a loop".into(),
                    });
                };
                let loop_start = loop_ctx.loop_start;
                let pops = self.locals.len().saturating_sub(loop_ctx.local_count);
                let meow_exits = self.meow_depth.saturating_sub(loop_ctx.meow_depth);
                for _ in 0..meow_exits {
                    self.chunk.write_op(Opcode::MeowExit, line);
                }
                for _ in 0..pops {
                    self.chunk.write_op(Opcode::Pop, line);
                }
                self.emit_loop(loop_start, line)?;
            }

            Stmt::While { cond, body, span } => {
                let loop_start = self.chunk.code.len();
                let line = span.line;

                self.compile_expr(cond)?;
                self.chunk.write_op(Opcode::JumpIfFalse, line);
                let exit_jump = self.emit_jump_placeholder(line);

                self.chunk.write_op(Opcode::Pop, line); // pop truthy condition
                self.loop_stack.push(LoopContext {
                    loop_start,
                    local_count: self.locals.len(),
                    meow_depth: self.meow_depth,
                    break_jumps: Vec::new(),
                });

                self.begin_scope();
                for s in &body.stmts {
                    self.compile_stmt(s)?;
                }
                if let Some(expr) = &body.expr {
                    self.compile_expr(expr)?;
                    self.chunk.write_op(Opcode::Pop, expr.span().line);
                }
                self.end_scope(line);

                self.emit_loop(loop_start, line)?;
                self.patch_jump(exit_jump)?;
                self.chunk.write_op(Opcode::Pop, line); // pop falsy condition

                let ctx = self.loop_stack.pop().unwrap();
                for bj in ctx.break_jumps {
                    self.patch_jump(bj)?;
                }
            }
        }
        Ok(())
    }

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        let line = expr.span().line;
        match expr {
            Expr::Literal(lit, _) => self.compile_literal(lit, line)?,

            Expr::Ident(name, _) => {
                match self.resolve_var(name) {
                    Some(VarBinding::Local { slot, .. }) => {
                        self.chunk.write_op(Opcode::GetLocal, line);
                        self.chunk.write_u16(slot, line);
                    }
                    Some(VarBinding::Global { slot, .. }) => {
                        self.chunk.write_op(Opcode::GetGlobal, line);
                        self.chunk.write_u16(slot, line);
                    }
                    None => {
                        return Err(CompileError {
                            message: format!("Undefined variable '{}'", name),
                        });
                    }
                }
            }

            Expr::Vec2 { x, y, .. } => {
                self.compile_expr(x)?;
                self.compile_expr(y)?;
                self.chunk.write_op(Opcode::MakeVec2, line);
            }

            Expr::Vec3 { x, y, z, .. } => {
                self.compile_expr(x)?;
                self.compile_expr(y)?;
                self.compile_expr(z)?;
                self.chunk.write_op(Opcode::MakeVec3, line);
            }

            Expr::Binary {
                left, op, right, ..
            } => match op {
                BinOp::And => {
                    self.compile_expr(left)?;
                    self.chunk.write_op(Opcode::JumpIfFalse, line);
                    let end_jump = self.emit_jump_placeholder(line);
                    self.chunk.write_op(Opcode::Pop, line);
                    self.compile_expr(right)?;
                    self.patch_jump(end_jump)?;
                }
                BinOp::Or => {
                    self.compile_expr(left)?;
                    self.chunk.write_op(Opcode::JumpIfFalse, line);
                    let rhs_jump = self.emit_jump_placeholder(line);
                    self.chunk.write_op(Opcode::Jump, line);
                    let end_jump = self.emit_jump_placeholder(line);
                    self.patch_jump(rhs_jump)?;
                    self.chunk.write_op(Opcode::Pop, line);
                    self.compile_expr(right)?;
                    self.patch_jump(end_jump)?;
                }
                _ => {
                    self.compile_expr(left)?;
                    self.compile_expr(right)?;
                    match op {
                        BinOp::Add => self.chunk.write_op(Opcode::Add, line),
                        BinOp::Sub => self.chunk.write_op(Opcode::Sub, line),
                        BinOp::Mul => self.chunk.write_op(Opcode::Mul, line),
                        BinOp::Div => self.chunk.write_op(Opcode::Div, line),
                        BinOp::Mod => self.chunk.write_op(Opcode::Mod, line),
                        BinOp::Eq => self.chunk.write_op(Opcode::Eq, line),
                        BinOp::NotEq => self.chunk.write_op(Opcode::NotEq, line),
                        BinOp::Lt => self.chunk.write_op(Opcode::Lt, line),
                        BinOp::LtEq => self.chunk.write_op(Opcode::LtEq, line),
                        BinOp::Gt => self.chunk.write_op(Opcode::Gt, line),
                        BinOp::GtEq => self.chunk.write_op(Opcode::GtEq, line),
                        BinOp::And | BinOp::Or => unreachable!(),
                    }
                }
            },

            Expr::Unary { op, expr, .. } => {
                self.compile_expr(expr)?;
                match op {
                    UnOp::Neg => self.chunk.write_op(Opcode::Neg, line),
                    UnOp::Not => self.chunk.write_op(Opcode::Not, line),
                }
            }

            Expr::Call { callee, args, .. } => {
                let fn_name = match callee.as_ref() {
                    Expr::Ident(name, _) => name,
                    _ => {
                        return Err(CompileError {
                            message: "Call target must be a function identifier".into(),
                        })
                    }
                };
                if args.len() > u8::MAX as usize {
                    return Err(CompileError {
                        message: "Too many arguments in function call".into(),
                    });
                }
                let argc = args.len() as u8;
                if let Some(&expected_arity) = self.fn_arities.get(fn_name) {
                    if expected_arity != argc {
                        return Err(CompileError {
                            message: format!(
                                "Function '{}' expects {} argument(s), got {}",
                                fn_name, expected_arity, argc
                            ),
                        });
                    }
                }
                for arg in args {
                    self.compile_expr(arg)?;
                }
                if let Some(&fn_idx) = self.fn_indices.get(fn_name) {
                    self.chunk.write_op(Opcode::Call, line);
                    self.chunk.write_u16(fn_idx, line);
                    self.chunk.write_byte(argc, line);
                } else {
                    let name_idx = self
                        .chunk
                        .try_add_constant(Constant::String(fn_name.clone()))
                        .map_err(|message| CompileError { message })?;
                    self.chunk.write_op(Opcode::CallNative, line);
                    self.chunk.write_u16(name_idx, line);
                    self.chunk.write_byte(argc, line);
                }
            }

            Expr::MeowBlock { block, span } => {
                self.chunk.write_op(Opcode::MeowEnter, span.line);
                self.meow_depth += 1;
                self.compile_block_value(block)?;
                self.meow_depth -= 1;
                self.chunk.write_op(Opcode::MeowExit, span.line);
            }

            Expr::Wait { duration, span } => {
                self.compile_expr(duration)?;
                self.chunk.write_op(Opcode::Wait, span.line);
                self.chunk.write_op(Opcode::Nil, span.line);
            }

            Expr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                self.compile_expr(cond)?;
                self.chunk.write_op(Opcode::JumpIfFalse, span.line);
                let else_jump = self.emit_jump_placeholder(span.line);

                self.chunk.write_op(Opcode::Pop, span.line); // pop truthy cond
                self.compile_block_value(then_branch)?;

                self.chunk.write_op(Opcode::Jump, span.line);
                let end_jump = self.emit_jump_placeholder(span.line);

                self.patch_jump(else_jump)?;
                self.chunk.write_op(Opcode::Pop, span.line); // pop falsy cond

                if let Some(else_b) = else_branch {
                    self.compile_block_value(else_b)?;
                } else {
                    self.chunk.write_op(Opcode::Nil, span.line);
                }

                self.patch_jump(end_jump)?;
            }

            Expr::MethodCall {
                target,
                method,
                args,
                ..
            } => {
                self.compile_expr(target)?;
                match method.as_str() {
                    "normalized" => {
                        if !args.is_empty() {
                            return Err(CompileError {
                                message: format!(
                                    "Method 'normalized' expects 0 arguments, got {}",
                                    args.len()
                                ),
                            });
                        }
                        self.chunk.write_op(Opcode::Vec2Normalize, line);
                    }
                    "length" => {
                        if !args.is_empty() {
                            return Err(CompileError {
                                message: format!(
                                    "Method 'length' expects 0 arguments, got {}",
                                    args.len()
                                ),
                            });
                        }
                        self.chunk.write_op(Opcode::Vec2Length, line);
                    }
                    "dot" => {
                        if args.len() != 1 {
                            return Err(CompileError {
                                message: format!(
                                    "Method 'dot' expects 1 argument, got {}",
                                    args.len()
                                ),
                            });
                        }
                        self.compile_expr(&args[0])?;
                        self.chunk.write_op(Opcode::Vec2Dot, line);
                    }
                    "cross" => {
                        if args.len() != 1 {
                            return Err(CompileError {
                                message: format!(
                                    "Method 'cross' expects 1 argument, got {}",
                                    args.len()
                                ),
                            });
                        }
                        self.compile_expr(&args[0])?;
                        self.chunk.write_op(Opcode::Vec3Cross, line);
                    }
                    "distance" => {
                        if args.len() != 1 {
                            return Err(CompileError {
                                message: format!(
                                    "Method 'distance' expects 1 argument, got {}",
                                    args.len()
                                ),
                            });
                        }
                        self.compile_expr(&args[0])?;
                        self.chunk.write_op(Opcode::Sub, line);
                        self.chunk.write_op(Opcode::Vec2Length, line);
                    }
                    other => {
                        return Err(CompileError {
                            message: format!("Unknown method '{}'", other),
                        });
                    }
                }
            }

            Expr::FieldAccess { target, field, .. } => {
                self.compile_expr(target)?;
                match field.as_str() {
                    "x" => self.chunk.write_op(Opcode::VecGetX, line),
                    "y" => self.chunk.write_op(Opcode::VecGetY, line),
                    "z" => self.chunk.write_op(Opcode::VecGetZ, line),
                    other => {
                        return Err(CompileError {
                            message: format!("Unknown field '{}'", other),
                        });
                    }
                }
            }

            Expr::Block(b) => {
                self.compile_block_value(b)?;
            }

            Expr::Match { value, arms, span } => {
                self.begin_scope();
                self.compile_expr(value)?;
                let match_slot = self.locals.len() as u16;
                self.locals.push(Local {
                    name: String::new(), // hidden anonymous match scrutinee
                    depth: self.scope_depth,
                    is_mut: false,
                });

                let mut end_jumps = Vec::with_capacity(arms.len());
                let mut has_catchall = false;

                for arm in arms {
                    let arm_line = arm.span.line;
                    match &arm.pattern {
                        Pattern::Wildcard => {
                            self.compile_expr(&arm.body)?;
                            self.chunk.write_op(Opcode::Jump, arm_line);
                            end_jumps.push(self.emit_jump_placeholder(arm_line));
                            has_catchall = true;
                            break;
                        }
                        Pattern::Ident(binding) => {
                            self.begin_scope();
                            self.chunk.write_op(Opcode::GetLocal, arm_line);
                            self.chunk.write_u16(match_slot, arm_line);
                            self.locals.push(Local {
                                name: binding.clone(),
                                depth: self.scope_depth,
                                is_mut: false,
                            });
                            self.compile_expr(&arm.body)?;
                            self.end_scope_keeping_top(arm_line)?;
                            self.chunk.write_op(Opcode::Jump, arm_line);
                            end_jumps.push(self.emit_jump_placeholder(arm_line));
                            has_catchall = true;
                            break;
                        }
                        Pattern::Literal(lit) => {
                            self.chunk.write_op(Opcode::GetLocal, arm_line);
                            self.chunk.write_u16(match_slot, arm_line);
                            self.compile_literal(lit, arm_line)?;
                            self.chunk.write_op(Opcode::Eq, arm_line);
                            self.chunk.write_op(Opcode::JumpIfFalse, arm_line);
                            let next_arm = self.emit_jump_placeholder(arm_line);
                            self.chunk.write_op(Opcode::Pop, arm_line);
                            self.compile_expr(&arm.body)?;
                            self.chunk.write_op(Opcode::Jump, arm_line);
                            end_jumps.push(self.emit_jump_placeholder(arm_line));
                            self.patch_jump(next_arm)?;
                            self.chunk.write_op(Opcode::Pop, arm_line);
                        }
                        Pattern::Variant { variant_name, .. } => {
                            self.chunk.write_op(Opcode::GetLocal, arm_line);
                            self.chunk.write_u16(match_slot, arm_line);
                            self.emit_const(Constant::String(variant_name.clone()), arm_line)?;
                            self.chunk.write_op(Opcode::Eq, arm_line);
                            self.chunk.write_op(Opcode::JumpIfFalse, arm_line);
                            let next_arm = self.emit_jump_placeholder(arm_line);
                            self.chunk.write_op(Opcode::Pop, arm_line);
                            self.compile_expr(&arm.body)?;
                            self.chunk.write_op(Opcode::Jump, arm_line);
                            end_jumps.push(self.emit_jump_placeholder(arm_line));
                            self.patch_jump(next_arm)?;
                            self.chunk.write_op(Opcode::Pop, arm_line);
                        }
                    }
                }

                if !has_catchall {
                    self.chunk.write_op(Opcode::Nil, span.line);
                }
                for j in end_jumps {
                    self.patch_jump(j)?;
                }
                self.end_scope_keeping_top(span.line)?;
            }
        }
        Ok(())
    }

    fn compile_literal(&mut self, lit: &Literal, line: usize) -> Result<(), CompileError> {
        match lit {
            Literal::Int(i) => self.emit_const(Constant::Int(*i), line)?,
            Literal::Float(f) => self.emit_const(Constant::Float(*f), line)?,
            Literal::Bool(b) => {
                if *b {
                    self.chunk.write_op(Opcode::True, line);
                } else {
                    self.chunk.write_op(Opcode::False, line);
                }
            }
            Literal::String(s) => self.emit_const(Constant::String(s.clone()), line)?,
        }
        Ok(())
    }

    fn emit_jump_placeholder(&mut self, line: usize) -> usize {
        self.chunk.write_byte(0xFF, line);
        self.chunk.write_byte(0xFF, line);
        self.chunk.code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) -> Result<(), CompileError> {
        let jump = self.chunk.code.len() - offset - 2;
        if jump > u16::MAX as usize {
            return Err(CompileError {
                message: "Jump offset exceeds 65535 bytes".into(),
            });
        }
        self.chunk.code[offset] = (jump >> 8) as u8;
        self.chunk.code[offset + 1] = (jump & 0xFF) as u8;
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize, line: usize) -> Result<(), CompileError> {
        self.chunk.write_op(Opcode::Loop, line);
        let offset = self.chunk.code.len() - loop_start + 2;
        if offset > u16::MAX as usize {
            return Err(CompileError {
                message: "Loop body exceeds 65535 bytes".into(),
            });
        }
        self.chunk.write_u16(offset as u16, line);
        Ok(())
    }
}
