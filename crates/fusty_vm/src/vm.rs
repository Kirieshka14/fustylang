use std::cmp::Ordering;

use crate::arena::FrameArena;
use crate::value::{Value, Vec2, Vec3};
use fusty_bytecode::chunk::{Chunk, Constant};
use fusty_bytecode::opcode::Opcode;

pub trait Host {
    fn call_native(&mut self, name: &str, args: &[Value], meow_mode: bool)
        -> Result<Value, String>;
}

pub struct NoopHost;

impl Host for NoopHost {
    fn call_native(
        &mut self,
        name: &str,
        _args: &[Value],
        _meow_mode: bool,
    ) -> Result<Value, String> {
        Err(format!("Unknown native function '{}'", name))
    }
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub return_ip: usize,
    pub base_slot: usize,
    pub prev_meow_depth: usize,
}

#[derive(Debug, PartialEq)]
pub enum VmResult {
    Done(Value),
    Waiting { duration_sec: f64 },
    Yielded,
}

#[derive(Debug)]
pub struct VmError {
    pub message: String,
    pub line: usize,
}

pub struct Vm {
    pub frame_arena: FrameArena,
    pub stack: Vec<Value>,
    pub globals: Vec<Value>,
    pub frames: Vec<CallFrame>,
    pub ip: usize,
    pub meow_mode: bool, // Safety disabled when inside one or more `meow` scopes
    meow_depth: usize,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn line_at(chunk: &Chunk, ip: usize) -> usize {
    chunk.lines.get(ip).copied().unwrap_or(0)
}

impl Vm {
    pub const DEFAULT_STEP_BUDGET: usize = 200_000;
    pub const MAX_CALL_DEPTH: usize = 256;

    pub fn new() -> Self {
        Self {
            frame_arena: FrameArena::new(1024 * 1024), // 1MB frame arena
            stack: Vec::with_capacity(512),
            globals: Vec::with_capacity(32),
            frames: Vec::with_capacity(32),
            ip: 0,
            meow_mode: false,
            meow_depth: 0,
        }
    }

    pub fn reset_frame(&mut self) {
        self.frame_arena.reset();
    }

    pub fn reset_execution(&mut self) {
        self.stack.clear();
        self.globals.clear();
        self.frames.clear();
        self.ip = 0;
        self.meow_depth = 0;
        self.meow_mode = false;
        self.frame_arena.reset();
    }

    pub fn run(&mut self, chunk: &Chunk) -> Result<VmResult, VmError> {
        let mut host = NoopHost;
        self.run_with_host(chunk, &mut host)
    }

    pub fn run_with_host(
        &mut self,
        chunk: &Chunk,
        host: &mut dyn Host,
    ) -> Result<VmResult, VmError> {
        self.run_with_host_budget(chunk, host, Self::DEFAULT_STEP_BUDGET)
    }

    pub fn run_with_host_budget(
        &mut self,
        chunk: &Chunk,
        host: &mut dyn Host,
        max_steps: usize,
    ) -> Result<VmResult, VmError> {
        let needed_globals = chunk.global_count as usize;
        if self.globals.len() < needed_globals {
            self.globals.resize(needed_globals, Value::Nil);
        }

        let mut steps = 0usize;
        let code = chunk.code.as_slice();
        let code_len = code.len();
        let mut base_slot = self.frames.last().map(|f| f.base_slot).unwrap_or(0);

        while self.ip < code_len {
            steps += 1;
            if steps > max_steps {
                return Err(VmError {
                    message: format!(
                        "Execution budget exceeded ({} steps without wait/yield)",
                        max_steps
                    ),
                    line: line_at(chunk, self.ip),
                });
            }

            let op_ip = self.ip;
            let byte = code[op_ip];
            self.ip = op_ip + 1;

            let op = match Opcode::from_u8(byte) {
                Some(op) => op,
                None => {
                    return Err(VmError {
                        message: format!("Unknown opcode byte: 0x{:02X}", byte),
                        line: line_at(chunk, op_ip),
                    })
                }
            };

            match op {
                Opcode::Const => {
                    let idx = self.read_u16(chunk, op_ip)?;
                    let c = chunk.constants.get(idx as usize).ok_or_else(|| VmError {
                        message: format!("Invalid constant index {}", idx),
                        line: line_at(chunk, op_ip),
                    })?;
                    let val = match c {
                        Constant::Int(i) => Value::Int(*i),
                        Constant::Float(f) => Value::Float(*f),
                        Constant::Bool(b) => Value::Bool(*b),
                        Constant::String(s) => Value::String(s.clone()),
                        Constant::Vec2(x, y) => Value::V2(Vec2::new(*x, *y)),
                        Constant::Vec3(x, y, z) => Value::V3(Vec3::new(*x, *y, *z)),
                    };
                    self.stack.push(val);
                }

                Opcode::Nil => self.stack.push(Value::Nil),
                Opcode::True => self.stack.push(Value::Bool(true)),
                Opcode::False => self.stack.push(Value::Bool(false)),

                Opcode::Pop => {
                    self.stack.pop();
                }

                Opcode::PopUnder => {
                    let count = self.read_u16(chunk, op_ip)? as usize;
                    let top = self.pop(chunk, op_ip)?;
                    if count > self.stack.len() {
                        return Err(VmError {
                            message: "Stack underflow on PopUnder".into(),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    self.stack.truncate(self.stack.len() - count);
                    self.stack.push(top);
                }

                Opcode::GetLocal => {
                    let slot = self.read_u16(chunk, op_ip)? as usize;
                    let idx = base_slot + slot;
                    let val = self.stack.get(idx).cloned().ok_or_else(|| VmError {
                        message: format!("Out-of-bounds local read slot {}", idx),
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(val);
                }

                Opcode::SetLocal => {
                    let slot = self.read_u16(chunk, op_ip)? as usize;
                    let idx = base_slot + slot;
                    let val = self.peek(0, chunk, op_ip)?.clone();
                    if idx >= self.stack.len() {
                        return Err(VmError {
                            message: format!("Out-of-bounds local write slot {}", idx),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    self.stack[idx] = val;
                }

                Opcode::GetGlobal => {
                    let slot = self.read_u16(chunk, op_ip)? as usize;
                    let val = self.globals.get(slot).cloned().ok_or_else(|| VmError {
                        message: format!("Out-of-bounds global read slot {}", slot),
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(val);
                }

                Opcode::SetGlobal => {
                    let slot = self.read_u16(chunk, op_ip)? as usize;
                    let val = self.peek(0, chunk, op_ip)?.clone();
                    if slot >= self.globals.len() {
                        self.globals.resize(slot + 1, Value::Nil);
                    }
                    self.globals[slot] = val;
                }

                Opcode::Add => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let res = a.add(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(res);
                }

                Opcode::Sub => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let res = a.sub(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(res);
                }

                Opcode::Mul => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let res = a.mul(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(res);
                }

                Opcode::Div => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let res = a.div(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(res);
                }

                Opcode::Mod => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let res = a.rem(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(res);
                }

                Opcode::Neg => {
                    let a = self.pop(chunk, op_ip)?;
                    let res = a.neg().map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(res);
                }

                Opcode::Not => {
                    let a = self.pop(chunk, op_ip)?;
                    self.stack.push(Value::Bool(!a.is_truthy()));
                }

                Opcode::Eq => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    self.stack.push(Value::Bool(a.equals(&b)));
                }

                Opcode::NotEq => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    self.stack.push(Value::Bool(!a.equals(&b)));
                }

                Opcode::Lt => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let ord = a.cmp_ord(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(Value::Bool(ord == Ordering::Less));
                }

                Opcode::LtEq => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let ord = a.cmp_ord(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(Value::Bool(ord != Ordering::Greater));
                }

                Opcode::Gt => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let ord = a.cmp_ord(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(Value::Bool(ord == Ordering::Greater));
                }

                Opcode::GtEq => {
                    let b = self.pop(chunk, op_ip)?;
                    let a = self.pop(chunk, op_ip)?;
                    let ord = a.cmp_ord(&b).map_err(|message| VmError {
                        message,
                        line: line_at(chunk, op_ip),
                    })?;
                    self.stack.push(Value::Bool(ord != Ordering::Less));
                }

                Opcode::MakeVec2 => {
                    let y = self.pop_f32(chunk, op_ip)?;
                    let x = self.pop_f32(chunk, op_ip)?;
                    self.stack.push(Value::V2(Vec2::new(x, y)));
                }

                Opcode::MakeVec3 => {
                    let z = self.pop_f32(chunk, op_ip)?;
                    let y = self.pop_f32(chunk, op_ip)?;
                    let x = self.pop_f32(chunk, op_ip)?;
                    self.stack.push(Value::V3(Vec3::new(x, y, z)));
                }

                Opcode::Vec2Add => {
                    let b = self.pop_vec2(chunk, op_ip)?;
                    let a = self.pop_vec2(chunk, op_ip)?;
                    self.stack.push(Value::V2(a + b));
                }

                Opcode::Vec2Sub => {
                    let b = self.pop_vec2(chunk, op_ip)?;
                    let a = self.pop_vec2(chunk, op_ip)?;
                    self.stack.push(Value::V2(a - b));
                }

                Opcode::Vec2MulScalar => {
                    let scalar = self.pop_f32(chunk, op_ip)?;
                    let vec = self.pop_vec2(chunk, op_ip)?;
                    self.stack.push(Value::V2(vec * scalar));
                }

                Opcode::Vec2Dot => match (self.pop(chunk, op_ip)?, self.pop(chunk, op_ip)?) {
                    (Value::V2(b), Value::V2(a)) => {
                        self.stack.push(Value::Float(a.dot(b) as f64));
                    }
                    (Value::V3(b), Value::V3(a)) => {
                        self.stack.push(Value::Float(a.dot(b) as f64));
                    }
                    (b, a) => {
                        return Err(VmError {
                            message: format!(
                                "Expected matching vectors for .dot, got {} and {}",
                                a, b
                            ),
                            line: line_at(chunk, op_ip),
                        })
                    }
                },

                Opcode::Vec2Length => match self.pop(chunk, op_ip)? {
                    Value::V2(vec) => self.stack.push(Value::Float(vec.length() as f64)),
                    Value::V3(vec) => self.stack.push(Value::Float(vec.length() as f64)),
                    other => {
                        return Err(VmError {
                            message: format!("Expected vector for .length, got {:?}", other),
                            line: line_at(chunk, op_ip),
                        })
                    }
                },

                Opcode::Vec2Normalize => match self.pop(chunk, op_ip)? {
                    Value::V2(vec) => self.stack.push(Value::V2(vec.normalized())),
                    Value::V3(vec) => self.stack.push(Value::V3(vec.normalized())),
                    other => {
                        return Err(VmError {
                            message: format!("Expected vector for .normalized, got {:?}", other),
                            line: line_at(chunk, op_ip),
                        })
                    }
                },

                Opcode::Vec3Cross => match (self.pop(chunk, op_ip)?, self.pop(chunk, op_ip)?) {
                    (Value::V3(b), Value::V3(a)) => {
                        self.stack.push(Value::V3(a.cross(b)));
                    }
                    (b, a) => {
                        return Err(VmError {
                            message: format!(
                                "Expected vec3 operands for .cross, got {} and {}",
                                a, b
                            ),
                            line: line_at(chunk, op_ip),
                        })
                    }
                },

                Opcode::VecGetX => match self.pop(chunk, op_ip)? {
                    Value::V2(v) => self.stack.push(Value::Float(v.x as f64)),
                    Value::V3(v) => self.stack.push(Value::Float(v.x as f64)),
                    other => {
                        return Err(VmError {
                            message: format!("Expected vector for .x, got {:?}", other),
                            line: line_at(chunk, op_ip),
                        })
                    }
                },

                Opcode::VecGetY => match self.pop(chunk, op_ip)? {
                    Value::V2(v) => self.stack.push(Value::Float(v.y as f64)),
                    Value::V3(v) => self.stack.push(Value::Float(v.y as f64)),
                    other => {
                        return Err(VmError {
                            message: format!("Expected vector for .y, got {:?}", other),
                            line: line_at(chunk, op_ip),
                        })
                    }
                },

                Opcode::VecGetZ => match self.pop(chunk, op_ip)? {
                    Value::V3(v) => self.stack.push(Value::Float(v.z as f64)),
                    other => {
                        return Err(VmError {
                            message: format!("Expected vec3 for .z, got {:?}", other),
                            line: line_at(chunk, op_ip),
                        })
                    }
                },

                Opcode::VecSetX => {
                    let val = self.pop_f32(chunk, op_ip)?;
                    match self.pop(chunk, op_ip)? {
                        Value::V2(mut v) => {
                            v.x = val;
                            self.stack.push(Value::V2(v));
                        }
                        Value::V3(mut v) => {
                            v.x = val;
                            self.stack.push(Value::V3(v));
                        }
                        other => {
                            return Err(VmError {
                                message: format!(
                                    "Expected vector for .x assignment, got {:?}",
                                    other
                                ),
                                line: line_at(chunk, op_ip),
                            })
                        }
                    }
                }

                Opcode::VecSetY => {
                    let val = self.pop_f32(chunk, op_ip)?;
                    match self.pop(chunk, op_ip)? {
                        Value::V2(mut v) => {
                            v.y = val;
                            self.stack.push(Value::V2(v));
                        }
                        Value::V3(mut v) => {
                            v.y = val;
                            self.stack.push(Value::V3(v));
                        }
                        other => {
                            return Err(VmError {
                                message: format!(
                                    "Expected vector for .y assignment, got {:?}",
                                    other
                                ),
                                line: line_at(chunk, op_ip),
                            })
                        }
                    }
                }

                Opcode::VecSetZ => {
                    let val = self.pop_f32(chunk, op_ip)?;
                    match self.pop(chunk, op_ip)? {
                        Value::V3(mut v) => {
                            v.z = val;
                            self.stack.push(Value::V3(v));
                        }
                        other => {
                            return Err(VmError {
                                message: format!(
                                    "Expected vec3 for .z assignment, got {:?}",
                                    other
                                ),
                                line: line_at(chunk, op_ip),
                            })
                        }
                    }
                }

                Opcode::Jump => {
                    let offset = self.read_u16(chunk, op_ip)? as usize;
                    let next_ip = self.ip.saturating_add(offset);
                    if next_ip > code_len {
                        return Err(VmError {
                            message: "Jump target out of bytecode bounds".into(),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    self.ip = next_ip;
                }

                Opcode::JumpIfFalse => {
                    let offset = self.read_u16(chunk, op_ip)? as usize;
                    if !self.peek(0, chunk, op_ip)?.is_truthy() {
                        let next_ip = self.ip.saturating_add(offset);
                        if next_ip > code_len {
                            return Err(VmError {
                                message: "Conditional jump target out of bytecode bounds".into(),
                                line: line_at(chunk, op_ip),
                            });
                        }
                        self.ip = next_ip;
                    }
                }

                Opcode::Loop => {
                    let offset = self.read_u16(chunk, op_ip)? as usize;
                    if offset > self.ip {
                        return Err(VmError {
                            message: "Invalid loop jump offset".into(),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    self.ip -= offset;
                }

                Opcode::Wait => {
                    let duration = match self.pop(chunk, op_ip)? {
                        Value::Float(f) => f,
                        Value::Int(i) => i as f64,
                        other => {
                            return Err(VmError {
                                message: format!(
                                    "Wait expects float/int duration, got {:?}",
                                    other
                                ),
                                line: line_at(chunk, op_ip),
                            })
                        }
                    };
                    if !duration.is_finite() {
                        return Err(VmError {
                            message: format!("Wait duration must be finite, got {}", duration),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    return Ok(VmResult::Waiting {
                        duration_sec: duration.max(0.0),
                    });
                }

                Opcode::Yield => {
                    return Ok(VmResult::Yielded);
                }

                Opcode::MeowEnter => {
                    self.meow_depth += 1;
                    self.meow_mode = true;
                }

                Opcode::MeowExit => {
                    self.meow_depth = self.meow_depth.saturating_sub(1);
                    self.meow_mode = self.meow_depth > 0;
                }

                Opcode::Call => {
                    let fn_idx = self.read_u16(chunk, op_ip)? as usize;
                    let argc = self.read_u8(chunk, op_ip)?;
                    if self.frames.len() >= Self::MAX_CALL_DEPTH {
                        return Err(VmError {
                            message: format!(
                                "Call stack overflow (max depth {})",
                                Self::MAX_CALL_DEPTH
                            ),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    let meta = chunk.functions.get(fn_idx).ok_or_else(|| VmError {
                        message: format!("Invalid function index {}", fn_idx),
                        line: line_at(chunk, op_ip),
                    })?;
                    if meta.arity != argc {
                        return Err(VmError {
                            message: format!(
                                "Function '{}' expected {} arguments, got {}",
                                meta.name, meta.arity, argc
                            ),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    if meta.entry_ip >= code_len {
                        return Err(VmError {
                            message: format!("Invalid entry point for function '{}'", meta.name),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    let argc_usize = argc as usize;
                    if self.stack.len() < argc_usize {
                        return Err(VmError {
                            message: "Stack underflow on function call".into(),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    let frame_base = self.stack.len() - argc_usize;
                    self.frames.push(CallFrame {
                        return_ip: self.ip,
                        base_slot: frame_base,
                        prev_meow_depth: self.meow_depth,
                    });
                    base_slot = frame_base;
                    self.ip = meta.entry_ip;
                }

                Opcode::CallNative => {
                    let name_idx = self.read_u16(chunk, op_ip)? as usize;
                    let argc = self.read_u8(chunk, op_ip)? as usize;
                    let fn_name = match chunk.constants.get(name_idx) {
                        Some(Constant::String(s)) => s.as_str(),
                        _ => {
                            return Err(VmError {
                                message: format!("Invalid native function constant {}", name_idx),
                                line: line_at(chunk, op_ip),
                            })
                        }
                    };
                    if self.stack.len() < argc {
                        return Err(VmError {
                            message: format!(
                                "Stack underflow calling native function '{}'",
                                fn_name
                            ),
                            line: line_at(chunk, op_ip),
                        });
                    }
                    let split_at = self.stack.len() - argc;
                    let meow = self.meow_mode;
                    let res = host
                        .call_native(fn_name, &self.stack[split_at..], meow)
                        .map_err(|message| VmError {
                            message,
                            line: line_at(chunk, op_ip),
                        })?;
                    self.stack.truncate(split_at);
                    self.stack.push(res);
                }

                Opcode::Return => {
                    let ret_val = self.stack.pop().unwrap_or(Value::Nil);
                    if let Some(frame) = self.frames.pop() {
                        self.stack.truncate(frame.base_slot);
                        self.ip = frame.return_ip;
                        self.meow_depth = frame.prev_meow_depth;
                        self.meow_mode = self.meow_depth > 0;
                        base_slot = self.frames.last().map(|f| f.base_slot).unwrap_or(0);
                        self.stack.push(ret_val);
                    } else {
                        self.meow_depth = 0;
                        self.meow_mode = false;
                        return Ok(VmResult::Done(ret_val));
                    }
                }

                Opcode::Halt => {
                    self.meow_depth = 0;
                    self.meow_mode = false;
                    let res = self.stack.pop().unwrap_or(Value::Nil);
                    return Ok(VmResult::Done(res));
                }
            }
        }

        self.meow_depth = 0;
        self.meow_mode = false;
        Ok(VmResult::Done(self.stack.pop().unwrap_or(Value::Nil)))
    }

    #[inline]
    fn read_u8(&mut self, chunk: &Chunk, op_ip: usize) -> Result<u8, VmError> {
        let b = chunk.code.get(self.ip).copied().ok_or_else(|| VmError {
            message: "Unexpected end of bytecode".into(),
            line: line_at(chunk, op_ip),
        })?;
        self.ip += 1;
        Ok(b)
    }

    #[inline]
    fn read_u16(&mut self, chunk: &Chunk, op_ip: usize) -> Result<u16, VmError> {
        let b1 = self.read_u8(chunk, op_ip)? as u16;
        let b2 = self.read_u8(chunk, op_ip)? as u16;
        Ok((b1 << 8) | b2)
    }

    #[inline]
    fn pop(&mut self, chunk: &Chunk, op_ip: usize) -> Result<Value, VmError> {
        self.stack.pop().ok_or_else(|| VmError {
            message: "Stack underflow".into(),
            line: line_at(chunk, op_ip),
        })
    }

    #[inline]
    fn peek(&self, distance: usize, chunk: &Chunk, op_ip: usize) -> Result<&Value, VmError> {
        if distance >= self.stack.len() {
            return Err(VmError {
                message: "Stack underflow on peek".into(),
                line: line_at(chunk, op_ip),
            });
        }
        let idx = self.stack.len() - 1 - distance;
        Ok(&self.stack[idx])
    }

    #[inline]
    fn pop_f32(&mut self, chunk: &Chunk, op_ip: usize) -> Result<f32, VmError> {
        match self.pop(chunk, op_ip)? {
            Value::Float(f) => Ok(f as f32),
            Value::Int(i) => Ok(i as f32),
            other => Err(VmError {
                message: format!("Expected number, got {:?}", other),
                line: line_at(chunk, op_ip),
            }),
        }
    }

    #[inline]
    fn pop_vec2(&mut self, chunk: &Chunk, op_ip: usize) -> Result<Vec2, VmError> {
        match self.pop(chunk, op_ip)? {
            Value::V2(v) => Ok(v),
            other => Err(VmError {
                message: format!("Expected vec2, got {:?}", other),
                line: line_at(chunk, op_ip),
            }),
        }
    }
}
