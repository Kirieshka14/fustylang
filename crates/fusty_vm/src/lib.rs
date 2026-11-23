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
}
