pub mod asm;
pub mod memory;

use std::fmt;

/// Execution mode for the FustyLang native JIT compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitMode {
    /// Compile hot `nya` functions automatically after reaching the call threshold.
    Auto { hot_threshold: usize },
    /// Compile eligible `nya` functions immediately on first invocation.
    Eager,
    /// Always interpret bytecode in the VM without native compilation.
    Disabled,
}

impl Default for JitMode {
    fn default() -> Self {
        Self::Auto { hot_threshold: 3 }
    }
}

/// Scalar type specialized by a native JIT-compiled function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitValueKind {
    Int64,
    Float64,
}

/// Runtime error codes written by JIT-compiled native code into `out_err`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JitRuntimeErr {
    None = 0,
    DivisionByZero = 1,
    ModuloByZero = 2,
    StackOverflow = 3,
}

impl JitRuntimeErr {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::DivisionByZero,
            2 => Self::ModuloByZero,
            3 => Self::StackOverflow,
            _ => Self::None,
        }
    }

    pub fn to_tsundere_message(self) -> &'static str {
        match self {
            Self::None => "",
            Self::DivisionByZero => {
                "B-baka! Division by zero inside JIT-compiled nya function?! >///<"
            }
            Self::ModuloByZero => {
                "Hmph! Modulo by zero inside JIT-compiled nya function, dummy! >.<"
            }
            Self::StackOverflow => {
                "B-baka! Call stack overflow inside JIT-compiled nya function! >///<"
            }
        }
    }
}

/// Error returned when a function cannot be JIT-compiled (triggers clean VM fallback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JitError {
    pub message: String,
}

impl JitError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Telemetry counters exposed by the JIT engine.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JitStats {
    pub compiled_functions: usize,
    pub native_calls: usize,
    pub vm_fallbacks: usize,
    pub compiled_bytes: usize,
    pub compiled_names: Vec<String>,
}

/// Returns `true` if the current target supports the pure-Rust x86-64 Linux JIT backend.
pub const fn is_supported_platform() -> bool {
    cfg!(all(target_arch = "x86_64", target_os = "linux"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_defaults_and_error_codes() {
        assert_eq!(JitMode::default(), JitMode::Auto { hot_threshold: 3 });
        assert_eq!(JitRuntimeErr::from_u8(1), JitRuntimeErr::DivisionByZero);
        assert!(JitRuntimeErr::DivisionByZero
            .to_tsundere_message()
            .contains("B-baka!"));
    }
}
