pub mod arena;
pub mod value;
pub mod vm;

#[cfg(test)]
mod tests {
    use super::value::*;
    use super::vm::*;
    use fusty_bytecode::chunk::{Chunk, Constant};
    use fusty_bytecode::compiler::Compiler;
    use fusty_bytecode::opcode::Opcode;
    use fusty_lexer::Lexer;
    use fusty_parser::Parser;

    fn compile_src(src: &str) -> Chunk {
        let tokens = Lexer::new(src).tokenize().expect("lex failed");
        let program = Parser::new(tokens).parse_program().expect("parse failed");
        Compiler::new()
            .compile_program(&program)
            .expect("compile failed")
    }

    #[test]
    fn test_vm_vector_math() {
        let mut chunk = Chunk::new();
        chunk.write_const(Constant::Float(10.0), 1);
        chunk.write_const(Constant::Float(20.0), 1);
        chunk.write_op(Opcode::MakeVec2, 1);
        chunk.write_const(Constant::Float(2.0), 1);
        chunk.write_op(Opcode::Mul, 1);
        chunk.write_op(Opcode::Halt, 1);

        let mut vm = Vm::new();
        let res = vm.run(&chunk).expect("VM run failed");
        assert_eq!(res, VmResult::Done(Value::V2(Vec2::new(20.0, 40.0))));
    }

    #[test]
    fn test_vm_coroutine_wait() {
        let mut chunk = Chunk::new();
        chunk.write_const(Constant::Float(0.35), 1);
        chunk.write_op(Opcode::Wait, 1);
        chunk.write_const(Constant::Int(42), 2);
        chunk.write_op(Opcode::Halt, 2);

        let mut vm = Vm::new();
        let res = vm.run(&chunk).expect("VM run failed");
        assert_eq!(res, VmResult::Waiting { duration_sec: 0.35 });

        let resumed = vm.run(&chunk).expect("Resume failed");
        assert_eq!(resumed, VmResult::Done(Value::Int(42)));
    }

    #[test]
    fn test_vm_meow_mode() {
        let mut chunk = Chunk::new();
        chunk.write_op(Opcode::MeowEnter, 1);
        chunk.write_op(Opcode::MeowExit, 1);
        chunk.write_op(Opcode::Halt, 1);

        let mut vm = Vm::new();
        let res = vm.run(&chunk).expect("Meow execution failed");
        assert_eq!(res, VmResult::Done(Value::Nil));
    }

    #[test]
    fn test_vm_user_functions_and_nested_wait() {
        let src = r#"
            nya step_attack(a: i32, b: i32) -> i32 {
                let sum = a + b;
                wait 0.25;
                return sum * 2;
            }

            let out = step_attack(10, 5);
            return out;
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        assert_eq!(
            vm.run(&chunk).unwrap(),
            VmResult::Waiting { duration_sec: 0.25 }
        );
        assert_eq!(vm.run(&chunk).unwrap(), VmResult::Done(Value::Int(30)));
    }

    #[test]
    fn test_vm_control_flow_while_and_short_circuit() {
        let src = r#"
            let mut acc = 0;
            let mut i = 1;
            while i <= 5 {
                if i == 3 {
                    acc += 100;
                } else if i == 5 || i == 2 {
                    acc += 10;
                } else if i > 0 && i < 10 {
                    acc += 1;
                }
                i += 1;
            }
            return acc;
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        // i=1 -> +1 (1)
        // i=2 -> +10 (11)
        // i=3 -> +100 (111)
        // i=4 -> +1 (112)
        // i=5 -> +10 (122)
        assert_eq!(vm.run(&chunk).unwrap(), VmResult::Done(Value::Int(122)));
    }

    struct MockHost {
        teleports: usize,
    }

    impl Host for MockHost {
        fn call_native(
            &mut self,
            name: &str,
            args: &[Value],
            meow_mode: bool,
        ) -> Result<Value, String> {
            match name {
                "raw_teleport" => {
                    if !meow_mode {
                        return Err("Requires meow mode".into());
                    }
                    self.teleports += args.len();
                    Ok(Value::Bool(true))
                }
                _ => Err(format!("Unknown native {}", name)),
            }
        }
    }

    #[test]
    fn test_vm_native_host_meow_enforcement() {
        let safe_fail = compile_src("raw_teleport(vec2(10.0, 20.0));");
        let mut host = MockHost { teleports: 0 };
        let mut vm = Vm::new();
        assert!(vm.run_with_host(&safe_fail, &mut host).is_err());
        assert_eq!(host.teleports, 0);

        let meow_ok = compile_src(
            r#"
            meow {
                raw_teleport(vec2(10.0, 20.0));
            }
        "#,
        );
        let mut vm2 = Vm::new();
        assert_eq!(
            vm2.run_with_host(&meow_ok, &mut host).unwrap(),
            VmResult::Done(Value::Nil)
        );
        assert_eq!(host.teleports, 1);
    }

    #[test]
    fn test_vm_step_budget_prevents_infinite_loop_hang() {
        let chunk = compile_src("while true { let x = 1; }");
        let mut vm = Vm::new();
        let mut host = NoopHost;
        let err = vm
            .run_with_host_budget(&chunk, &mut host, 1_000)
            .unwrap_err();
        assert!(err.message.contains("budget exceeded"));
    }

    #[test]
    fn test_globals_accessible_inside_functions() {
        let src = r#"
            let speed = 4.0;
            let mut counter = 0;

            nya bump(v: f64) -> f64 {
                counter += 1;
                v * speed
            }

            let r = bump(3.0);
            return r + counter;
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        assert_eq!(vm.run(&chunk).unwrap(), VmResult::Done(Value::Float(13.0)));
    }

    #[test]
    fn test_immutability_enforced() {
        let tokens = Lexer::new("let x = 5; x = 10;").tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let err = Compiler::new().compile_program(&program).unwrap_err();
        assert!(err.message.contains("immutable"));
    }

    #[test]
    fn test_nested_meow_does_not_exit_early() {
        let src = r#"
            meow {
                meow {
                    let a = 1;
                }
                raw_teleport(vec2(1.0, 2.0));
            }
        "#;
        let chunk = compile_src(src);
        let mut host = MockHost { teleports: 0 };
        let mut vm = Vm::new();
        vm.run_with_host(&chunk, &mut host).unwrap();
        assert_eq!(host.teleports, 1);
        assert!(!vm.meow_mode);
    }

    #[test]
    fn test_if_else_if_block_and_match_expressions() {
        let src = r#"
            nya classify(n: i32) -> i32 {
                let base = if n < 0 {
                    -10
                } else if n == 0 {
                    0
                } else {
                    let tmp = n * 2;
                    tmp + 1
                };
                match base {
                    -10 => 100,
                    0 => 200,
                    v => v + 5,
                }
            }
            return classify(-1) + classify(0) + classify(4);
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        // classify(-1)=100, classify(0)=200, classify(4)=9+5=14 -> 314
        assert_eq!(vm.run(&chunk).unwrap(), VmResult::Done(Value::Int(314)));
    }

    #[test]
    fn test_break_continue_and_yield() {
        let src = r#"
            let mut sum = 0;
            let mut i = 0;
            while i < 10 {
                i += 1;
                if i == 3 {
                    continue;
                }
                if i == 6 {
                    break;
                }
                sum += i;
                if i == 2 {
                    yield;
                }
            }
            return sum;
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        assert_eq!(vm.run(&chunk).unwrap(), VmResult::Yielded);
        // sum of 1, 2, 4, 5 = 12
        assert_eq!(vm.run(&chunk).unwrap(), VmResult::Done(Value::Int(12)));
    }

    #[test]
    fn test_vector_field_assign_and_cross_and_div_zero() {
        let src = r#"
            let mut v = vec3(1.0, 0.0, 0.0);
            v.y += 2.0;
            v.z = 3.0;
            let c = vec3(1.0, 0.0, 0.0).cross(vec3(0.0, 1.0, 0.0));
            return v + c;
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        assert_eq!(
            vm.run(&chunk).unwrap(),
            VmResult::Done(Value::V3(Vec3::new(1.0, 2.0, 4.0)))
        );

        let div_zero = compile_src("let v = vec2(1.0, 2.0) / 0.0;");
        let mut vm2 = Vm::new();
        assert!(vm2.run(&div_zero).is_err());

        let mod_zero = compile_src("let x = 5.0 % 0.0;");
        let mut vm3 = Vm::new();
        assert!(vm3.run(&mod_zero).is_err());
    }

    #[test]
    fn test_call_stack_overflow_and_lexer_unterminated_comment() {
        let rec = compile_src("nya boom() { boom(); } boom();");
        let mut vm = Vm::new();
        let err = vm.run(&rec).unwrap_err();
        assert!(err.message.contains("Call stack overflow"));

        let lex_err = Lexer::new("let a = 1; /* unterminated").tokenize();
        assert!(lex_err.is_err());

        let mut arena = super::arena::FrameArena::new(64);
        assert!(arena.alloc_bytes(8, 0).is_none());
        assert!(arena.alloc_bytes(8, 3).is_none());
        let ptr = arena.alloc_bytes(16, 8).unwrap();
        assert_eq!(ptr as usize % 8, 0);
    }

    #[test]
    fn test_vm_baka_panic_with_and_without_message() {
        let src = r#"
            nya guard(hp: i64) -> i64 {
                if hp <= 0 {
                    baka!("out of headpats");
                }
                return hp;
            }
            return guard(0);
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        let err = vm.run(&chunk).unwrap_err();
        assert!(err.message.contains("[BAKA! >///<]"));
        assert!(err.message.contains("out of headpats"));

        let empty_baka = compile_src("baka!();");
        let mut vm2 = Vm::new();
        let err2 = vm2.run(&empty_baka).unwrap_err();
        assert!(err2.message.contains("[BAKA! >///<]"));
        assert!(err2.message.contains("explicit baka! panic"));
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    fn test_vm_tiered_jit_auto_and_eager_execution() {
        let src = r#"
            nya fib(n: i64) -> i64 {
                if n <= 1 {
                    return n;
                }
                return fib(n - 1) + fib(n - 2);
            }

            let mut total = 0;
            let mut i = 0;
            while i < 5 {
                total += fib(12);
                i += 1;
            }
            return total;
        "#;
        let chunk = compile_src(src);

        // VM-only baseline
        let mut vm_interp = Vm::with_jit_mode(fusty_jit::JitMode::Disabled);
        let interp_res = vm_interp.run(&chunk).unwrap();
        assert_eq!(interp_res, VmResult::Done(Value::Int(144 * 5)));
        assert_eq!(vm_interp.jit_stats().native_calls, 0);

        // Auto-JIT (inner recursive calls hit hot_threshold immediately and tier up to native!)
        let mut vm_auto = Vm::with_jit_mode(fusty_jit::JitMode::Auto { hot_threshold: 3 });
        let auto_res = vm_auto.run(&chunk).unwrap();
        assert_eq!(auto_res, interp_res);
        assert_eq!(vm_auto.jit_stats().compiled_functions, 1);
        assert!(vm_auto.jit_stats().native_calls >= 3);

        // Eager JIT (compiles on 1st call, all 5 calls native)
        let mut vm_eager = Vm::with_jit_mode(fusty_jit::JitMode::Eager);
        let eager_res = vm_eager.run(&chunk).unwrap();
        assert_eq!(eager_res, interp_res);
        assert_eq!(vm_eager.jit_stats().compiled_functions, 1);
        assert_eq!(vm_eager.jit_stats().native_calls, 5);
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    fn test_jit_vm_parity_arithmetic_comparisons_and_f64() {
        let src = r#"
            nya collatz_steps(start: i64) -> i64 {
                let mut n = start;
                let mut steps = 0;
                while n != 1 {
                    if n % 2 == 0 {
                        n = n / 2;
                    } else {
                        n = n * 3 + 1;
                    }
                    steps += 1;
                }
                return steps;
            }

            nya poly_f64(x: f64, iters: f64) -> f64 {
                let mut acc = x;
                let mut i = 0.0;
                while i < iters {
                    acc = (acc * 1.25 - 0.5) / 1.1;
                    i = i + 1.0;
                }
                return acc;
            }

            let c = collatz_steps(27);
            let p = poly_f64(4.0, 6.0);
            return p + c;
        "#;
        let chunk = compile_src(src);

        let mut vm_interp = Vm::with_jit_mode(fusty_jit::JitMode::Disabled);
        let interp_val = match vm_interp.run(&chunk).unwrap() {
            VmResult::Done(Value::Float(f)) => f,
            other => panic!("Unexpected VM result: {:?}", other),
        };

        let mut vm_jit = Vm::with_jit_mode(fusty_jit::JitMode::Eager);
        let jit_val = match vm_jit.run(&chunk).unwrap() {
            VmResult::Done(Value::Float(f)) => f,
            other => panic!("Unexpected JIT result: {:?}", other),
        };

        assert!((interp_val - jit_val).abs() < 1e-12);
        assert_eq!(vm_jit.jit_stats().compiled_functions, 2);
        assert_eq!(vm_jit.jit_stats().native_calls, 2);
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    fn test_jit_div_zero_and_baka_fallback_in_eager_mode() {
        let div_src = r#"
            nya bad_div(a: i64, b: i64) -> i64 {
                return a / b;
            }
            return bad_div(10, 0);
        "#;
        let chunk = compile_src(div_src);
        let mut vm = Vm::with_jit_mode(fusty_jit::JitMode::Eager);
        let err = vm.run(&chunk).unwrap_err();
        assert!(err.message.contains("B-baka!"));
        assert!(err.message.contains("Division by zero"));

        let baka_src = r#"
            nya angry(x: i64) -> i64 {
                if x < 0 {
                    baka!("too negative!");
                }
                return x;
            }
            return angry(-5);
        "#;
        let chunk2 = compile_src(baka_src);
        let mut vm2 = Vm::with_jit_mode(fusty_jit::JitMode::Eager);
        let err2 = vm2.run(&chunk2).unwrap_err();
        assert!(err2.message.contains("[BAKA! >///<]"));
        assert!(err2.message.contains("too negative!"));
        assert_eq!(vm2.jit_stats().compiled_functions, 0);
    }
}
