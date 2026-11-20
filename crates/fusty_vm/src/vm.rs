use std::cmp::Ordering;

use crate::arena::FrameArena;
use crate::value::{Value, Vec2, Vec3};
use fusty_bytecode::chunk::{Chunk, Constant};
use fusty_bytecode::opcode::Opcode;

pub trait Host {
    fn call_native(
        &mut self,
        name: &str,
        args: &[Value],
        meow_mode: bool,
    ) -> Result<Value, String>;
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


}
