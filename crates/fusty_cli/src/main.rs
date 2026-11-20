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

fn main() {}
