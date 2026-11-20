use std::env;
use std::fs;
use std::process::ExitCode;

use fusty_bytecode::compiler::Compiler;
use fusty_lexer::Lexer;
use fusty_parser::Parser;
use fusty_vm::value::Value;
use fusty_vm::vm::{Host, Vm, VmResult};

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
            "sin" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.sin()))
                .ok_or_else(|| "'sin' expects 1 number".into()),
            "cos" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.cos()))
                .ok_or_else(|| "'cos' expects 1 number".into()),
            "sqrt" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.sqrt()))
                .ok_or_else(|| "'sqrt' expects 1 number".into()),
            "abs" => args
                .first()
                .and_then(|v| v.as_f64())
                .map(|x| Value::Float(x.abs()))
                .ok_or_else(|| "'abs' expects 1 number".into()),
            "min" if args.len() == 2 => {
                let a = args[0].as_f64().ok_or("'min' expects numbers")?;
                let b = args[1].as_f64().ok_or("'min' expects numbers")?;
                Ok(Value::Float(a.min(b)))
            }
            "max" if args.len() == 2 => {
                let a = args[0].as_f64().ok_or("'max' expects numbers")?;
                let b = args[1].as_f64().ok_or("'max' expects numbers")?;
                Ok(Value::Float(a.max(b)))
            }
            "clamp" if args.len() == 3 => {
                let x = args[0].as_f64().ok_or("'clamp' expects numbers")?;
                let lo = args[1].as_f64().ok_or("'clamp' expects numbers")?;
                let hi = args[2].as_f64().ok_or("'clamp' expects numbers")?;
                Ok(Value::Float(x.clamp(lo, hi)))
            }
            "is_meow" => Ok(Value::Bool(meow_mode)),
            other => Err(format!("Unknown native function '{}'", other)),
        }
    }
}

fn run_source(source: &str, file_name: &str) -> bool {
    println!("=== [FustyLang] Compiling '{}' ===", file_name);

    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err((err, span)) => {
            eprintln!("Lexer error at {}:{}: {}", span.line, span.col, err);
            return false;
        }
    };
    println!("  [1/3] Lexed {} tokens.", tokens.len());

    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parser error at {}:{}: {}", e.span.line, e.span.col, e.message);
            return false;
        }
    };
    println!("  [2/3] Parsed AST: {} top-level items.", program.items.len());

    let compiler = Compiler::new();
    let chunk = match compiler.compile_program(&program) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Compile error: {}", e.message);
            return false;
        }
    };
    println!(
        "  [3/3] Emitted {} bytes of bytecode, {} constants.",
        chunk.code.len(),
        chunk.constants.len()
    );

    println!("\n=== [FustyLang] Running in Ferrium VM (Frame Arena + Dynamic Ticks) ===");
    let mut vm = Vm::new();
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
                println!("  [DONE] Script completed successfully with result: {}", val);
                return true;
            }
            Err(e) => {
                eprintln!("  [RUNTIME ERROR] Line {}: {}", e.line, e.message);
                return false;
            }
        }
    }
}

fn main() {}
