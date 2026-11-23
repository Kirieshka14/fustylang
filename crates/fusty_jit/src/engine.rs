use std::collections::{HashMap, HashSet};

use fusty_bytecode::chunk::Chunk;

use crate::compiler::{compile_function, CompiledFunction};
use crate::{JitMode, JitStats, JitValueKind};

/// Key identifying a specialized native compilation of a `nya` function.
type CacheKey = (usize, JitValueKind);

/// Tiered JIT engine with hot-call profiling, compiled code cache, and clean VM fallback.
pub struct JitEngine {
    mode: JitMode,
    call_counts: HashMap<CacheKey, usize>,
    cache: HashMap<CacheKey, CompiledFunction>,
    rejected: HashSet<CacheKey>,
    stats: JitStats,
}

impl Default for JitEngine {
    fn default() -> Self {
        Self::new(JitMode::default())
    }
}

impl JitEngine {
    pub fn new(mode: JitMode) -> Self {
        Self {
            mode,
            call_counts: HashMap::new(),
            cache: HashMap::new(),
            rejected: HashSet::new(),
            stats: JitStats::default(),
        }
    }

    #[inline]
    pub fn mode(&self) -> JitMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: JitMode) {
        self.mode = mode;
    }

    #[inline]
    pub fn stats(&self) -> &JitStats {
        &self.stats
    }

    #[inline]
    pub fn record_native_call(&mut self) {
        self.stats.native_calls += 1;
    }

    #[inline]
    pub fn record_fallback(&mut self) {
        self.stats.vm_fallbacks += 1;
    }

    pub fn clear(&mut self) {
        self.call_counts.clear();
        self.cache.clear();
        self.rejected.clear();
        self.stats = JitStats::default();
    }

    /// Profiles a call to `fn_idx` with argument specialization `kind`.
    /// Returns a reference to the compiled native function once hot (or immediately in `Eager` mode),
    /// or `None` to fall back to the bytecode VM.
    pub fn get_or_compile(
        &mut self,
        chunk: &Chunk,
        fn_idx: usize,
        kind: JitValueKind,
    ) -> Option<&CompiledFunction> {
        let threshold = match self.mode {
            JitMode::Disabled => return None,
            JitMode::Eager => 1,
            JitMode::Auto { hot_threshold } => hot_threshold.max(1),
        };

        let key = (fn_idx, kind);

        if self.cache.contains_key(&key) {
            return self.cache.get(&key);
        }

        if self.rejected.contains(&key) {
            return None;
        }

        let count = self.call_counts.entry(key).or_insert(0);
        *count += 1;

        if *count < threshold {
            return None;
        }

        match compile_function(chunk, fn_idx, kind) {
            Ok(compiled) => {
                self.stats.compiled_functions += 1;
                self.stats.compiled_bytes += compiled.code_size();
                self.stats.compiled_names.push(format!(
                    "{}<{}>",
                    compiled.name,
                    match kind {
                        JitValueKind::Int64 => "i64",
                        JitValueKind::Float64 => "f64",
                    }
                ));
                self.cache.insert(key, compiled);
                self.cache.get(&key)
            }
            Err(_) => {
                self.rejected.insert(key);
                None
            }
        }
    }
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
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    fn test_jit_engine_auto_threshold_and_cache() {
        let chunk = compile_chunk("nya sq(x: i64) -> i64 { return x * x; }");
        let mut engine = JitEngine::new(JitMode::Auto { hot_threshold: 3 });

        assert!(engine
            .get_or_compile(&chunk, 0, JitValueKind::Int64)
            .is_none());
        assert!(engine
            .get_or_compile(&chunk, 0, JitValueKind::Int64)
            .is_none());
        let compiled = engine
            .get_or_compile(&chunk, 0, JitValueKind::Int64)
            .expect("should compile on 3rd call");
        assert_eq!(compiled.call_i64(&[9]).unwrap(), 81);
        assert_eq!(engine.stats().compiled_functions, 1);
        assert!(engine.stats().compiled_bytes > 0);
    }

    #[test]
    fn test_jit_engine_disabled_and_rejection_cache() {
        let chunk = compile_chunk(r#"nya panic_fn() { baka!("no jit"); }"#);
        let mut eager = JitEngine::new(JitMode::Eager);
        assert!(eager
            .get_or_compile(&chunk, 0, JitValueKind::Int64)
            .is_none());
        assert!(eager
            .get_or_compile(&chunk, 0, JitValueKind::Int64)
            .is_none());
        assert_eq!(eager.stats().compiled_functions, 0);

        let chunk2 = compile_chunk("nya inc(x: i64) -> i64 { return x + 1; }");
        let mut disabled = JitEngine::new(JitMode::Disabled);
        assert!(disabled
            .get_or_compile(&chunk2, 0, JitValueKind::Int64)
            .is_none());
    }
}
