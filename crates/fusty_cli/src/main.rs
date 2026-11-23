use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use fusty_bytecode::compiler::Compiler;
use fusty_jit::JitMode;
use fusty_lexer::Lexer;
use fusty_parser::Parser;
use fusty_vm::value::Value;
use fusty_vm::vm::{Host, Vm, VmResult};

#[derive(Debug, Clone, Copy)]
struct RunOptions {
    jit_mode: JitMode,
    show_jit_stats: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            jit_mode: JitMode::default(),
            show_jit_stats: false,
        }
    }
}

struct CliHost;

impl Host for CliHost {
    fn call_native(
        &mut self,
        name: &str,
        args: &[Value],
        meow_mode: bool,
    ) -> Result<Value, String> {
        match name {
            "log" => {
                let text = args
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("  [LOG] {}", text);
                Ok(Value::Nil)
            }
            "purr" => {
                let text = args
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("  [PURR ~nya] {}", text);
                Ok(Value::Nil)
            }
            "sin" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.sin()))
                .ok_or_else(|| "B-baka! 'sin' expects 1 number! >.<".into()),
            "cos" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.cos()))
                .ok_or_else(|| "B-baka! 'cos' expects 1 number! >.<".into()),
            "sqrt" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.sqrt()))
                .ok_or_else(|| "B-baka! 'sqrt' expects 1 number! >.<".into()),
            "abs" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.abs()))
                .ok_or_else(|| "B-baka! 'abs' expects 1 number! >.<".into()),
            "min" if args.len() == 2 => {
                let a = args[0]
                    .as_f64()
                    .ok_or("B-baka! 'min' expects numbers! >.<")?;
                let b = args[1]
                    .as_f64()
                    .ok_or("B-baka! 'min' expects numbers! >.<")?;
                Ok(Value::Float(a.min(b)))
            }
            "max" if args.len() == 2 => {
                let a = args[0]
                    .as_f64()
                    .ok_or("B-baka! 'max' expects numbers! >.<")?;
                let b = args[1]
                    .as_f64()
                    .ok_or("B-baka! 'max' expects numbers! >.<")?;
                Ok(Value::Float(a.max(b)))
            }
            "clamp" if args.len() == 3 => {
                let x = args[0]
                    .as_f64()
                    .ok_or("B-baka! 'clamp' expects numbers! >.<")?;
                let lo = args[1]
                    .as_f64()
                    .ok_or("B-baka! 'clamp' expects numbers! >.<")?;
                let hi = args[2]
                    .as_f64()
                    .ok_or("B-baka! 'clamp' expects numbers! >.<")?;
                Ok(Value::Float(x.clamp(lo, hi)))
            }
            "is_meow" => Ok(Value::Bool(meow_mode)),
            other => Err(format!(
                "B-baka! I don't know any native function called '{}'! >///<",
                other
            )),
        }
    }
}

fn jit_mode_label(mode: JitMode) -> &'static str {
    match mode {
        JitMode::Auto { .. } => "Auto Tiered x86-64 JIT",
        JitMode::Eager => "Eager x86-64 JIT (--jit)",
        JitMode::Disabled => "Bytecode VM only (--no-jit)",
    }
}

fn run_source(source: &str, file_name: &str, opts: RunOptions) -> bool {
    println!(
        "=== [FustyLang ~nya] Compiling '{}' (it's not like I wanted to or anything!) ===",
        file_name
    );

    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err((err, span)) => {
            eprintln!("  [LEXER BAKA! >///<] {}:{}: {}", span.line, span.col, err);
            return false;
        }
    };
    println!("  [1/3] Lexed {} tokens.", tokens.len());

    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "  [PARSER BAKA! >///<] {}:{}: {}",
                e.span.line, e.span.col, e.message
            );
            return false;
        }
    };
    println!(
        "  [2/3] Parsed AST: {} top-level items.",
        program.items.len()
    );

    let compiler = Compiler::new();
    let chunk = match compiler.compile_program(&program) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  [COMPILER BAKA! >.<] {}", e.message);
            return false;
        }
    };
    println!(
        "  [3/3] Emitted {} bytes of bytecode, {} constants.",
        chunk.code.len(),
        chunk.constants.len()
    );

    println!(
        "\n=== [FustyLang ~nya] Running in Ferrium VM [{}] ===",
        jit_mode_label(opts.jit_mode)
    );
    let mut vm = Vm::with_jit_mode(opts.jit_mode);
    let mut host = CliHost;

    loop {
        match vm.run_with_host(&chunk, &mut host) {
            Ok(VmResult::Waiting { duration_sec }) => {
                println!(
                    "  [TICK] Coroutine requested 'wait {:.2}s' (yielding to Ferrium game loop)",
                    duration_sec
                );
                vm.reset_frame();
            }
            Ok(VmResult::Yielded) => {
                println!("  [YIELD] Script yielded execution (resuming next tick).");
                vm.reset_frame();
            }
            Ok(VmResult::Done(val)) => {
                println!(
                    "  [DONE ~nya] Hmph... fine, your script finished with result: {}",
                    val
                );
                if opts.show_jit_stats {
                    let s = vm.jit_stats();
                    println!(
                        "  [JIT >w<] Compiled {} hot nya-function(s) ({} bytes x86-64), {} native call(s), {} VM fallback(s)",
                        s.compiled_functions, s.compiled_bytes, s.native_calls, s.vm_fallbacks
                    );
                    if !s.compiled_names.is_empty() {
                        println!(
                            "  [JIT >w<] Native symbols: {}",
                            s.compiled_names.join(", ")
                        );
                    }
                }
                return true;
            }
            Err(e) => {
                eprintln!("  [RUNTIME ERROR] Line {}: {}", e.line, e.message);
                return false;
            }
        }
    }
}

fn execute_silent(
    chunk: &fusty_bytecode::chunk::Chunk,
    mode: JitMode,
) -> Result<(Value, fusty_jit::JitStats), String> {
    let mut vm = Vm::with_jit_mode(mode);
    let mut host = CliHost;
    loop {
        match vm.run_with_host_budget(chunk, &mut host, 50_000_000) {
            Ok(VmResult::Waiting { .. } | VmResult::Yielded) => vm.reset_frame(),
            Ok(VmResult::Done(val)) => return Ok((val, vm.jit_stats().clone())),
            Err(e) => return Err(format!("Line {}: {}", e.line, e.message)),
        }
    }
}

fn bench_source(source: &str, file_name: &str) -> bool {
    println!("=== [FustyLang ~nya] Benchmarking '{}' ===", file_name);
    let tokens = match Lexer::new(source).tokenize() {
        Ok(t) => t,
        Err((err, span)) => {
            eprintln!("  [LEXER BAKA! >///<] {}:{}: {}", span.line, span.col, err);
            return false;
        }
    };
    let program = match Parser::new(tokens).parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "  [PARSER BAKA! >///<] {}:{}: {}",
                e.span.line, e.span.col, e.message
            );
            return false;
        }
    };
    let chunk = match Compiler::new().compile_program(&program) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  [COMPILER BAKA! >.<] {}", e.message);
            return false;
        }
    };

    let t0 = Instant::now();
    let (vm_val, _) = match execute_silent(&chunk, JitMode::Disabled) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  [VM ERROR] {}", e);
            return false;
        }
    };
    let vm_dur = t0.elapsed();

    let t1 = Instant::now();
    let (jit_val, jit_stats) = match execute_silent(&chunk, JitMode::Eager) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  [JIT ERROR] {}", e);
            return false;
        }
    };
    let jit_dur = t1.elapsed();

    if vm_val != jit_val {
        eprintln!(
            "  [BAKA! >///<] Result mismatch! VM={} vs JIT={}",
            vm_val, jit_val
        );
        return false;
    }

    let vm_ms = vm_dur.as_secs_f64() * 1000.0;
    let jit_ms = jit_dur.as_secs_f64() * 1000.0;
    let speedup = if jit_ms > 0.0 { vm_ms / jit_ms } else { 1.0 };

    println!("  Result:      {}", jit_val);
    println!("  VM (no-jit): {:>8.3} ms", vm_ms);
    println!(
        "  JIT (x86-64):{:>8.3} ms ({} compiled, {} native calls, {} bytes)",
        jit_ms, jit_stats.compiled_functions, jit_stats.native_calls, jit_stats.compiled_bytes
    );
    println!(
        "  Speedup:     {:.2}x  (H-hmph! Don't get used to me running this fast for you... >///<)",
        speedup
    );
    true
}

fn parse_flags(args: &[String]) -> (RunOptions, Vec<String>) {
    let mut opts = RunOptions::default();
    let mut positional = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--jit" => opts.jit_mode = JitMode::Eager,
            "--no-jit" => opts.jit_mode = JitMode::Disabled,
            "--jit-stats" => opts.show_jit_stats = true,
            _ => positional.push(arg.clone()),
        }
    }
    (opts, positional)
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let (opts, positional) = parse_flags(&raw_args);

    if positional.first().map(|s| s.as_str()) == Some("run") {
        let Some(path) = positional.get(1) else {
            eprintln!(
                "B-baka! Usage: fusty run <path_to_script.fy> [--jit|--no-jit] [--jit-stats]"
            );
            return ExitCode::FAILURE;
        };
        match fs::read_to_string(path) {
            Ok(content) => {
                if run_source(&content, path, opts) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("Hmph! Couldn't read '{}', dummy! ({}) >.<", path, e);
                ExitCode::FAILURE
            }
        }
    } else if positional.first().map(|s| s.as_str()) == Some("bench") {
        let Some(path) = positional.get(1) else {
            eprintln!("B-baka! Usage: fusty bench <path_to_script.fy>");
            return ExitCode::FAILURE;
        };
        match fs::read_to_string(path) {
            Ok(content) => {
                if bench_source(&content, path) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("Hmph! Couldn't read '{}', dummy! ({}) >.<", path, e);
                ExitCode::FAILURE
            }
        }
    } else {
        println!(
            "FustyLang (~nya) v0.2.0 — Fast, Tsundere Rust-like Scripting + Native x86-64 JIT"
        );
        println!(
            "(Run with `fusty run <file.fy> [--jit|--no-jit] [--jit-stats]` or `fusty bench <file.fy>`... b-baka!)\n"
        );

        let demo_script = r#"
            // Fast Combat & Arena Demo (~nya edition)
            nya boost_speed(base: f64, mult: f64) -> f64 {
                return base * mult;
            }

            let mut player_pos = vec2(100.0, 50.0);
            let target_pos = vec2(300.0, 200.0);

            let to_target = target_pos - player_pos;
            let dir = to_target.normalized();
            let vel = dir * boost_speed(250.0, 1.0);

            player_pos += vel * 0.016; // 1 frame tick (60 FPS)

            wait 0.25;

            meow {
                let fast_val = 1337;
                purr("meow fast_val =", fast_val);
            }

            wait 0.15;
            return player_pos;
        "#;

        if run_source(demo_script, "<embedded demo>", opts) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}
