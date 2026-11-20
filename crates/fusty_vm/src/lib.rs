pub mod vm;
pub mod arena;
pub mod value;

#[cfg(test)]
mod tests {
    use super::vm::*;
    use super::value::*;
    use fusty_bytecode::chunk::{Chunk, Constant};
    use fusty_bytecode::compiler::Compiler;
    use fusty_bytecode::opcode::Opcode;
    use fusty_lexer::Lexer;
    use fusty_parser::Parser;

    fn compile_src(src: &str) -> Chunk {
        let tokens = Lexer::new(src).tokenize().expect("lex failed");
        let program = Parser::new(tokens).parse_program().expect("parse failed");
        Compiler::new().compile_program(&program).expect("compile failed")
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
            fn step_attack(a: i32, b: i32) -> i32 {
                let sum = a + b;
                wait 0.25;
                return sum * 2;
            }

            let out = step_attack(10, 5);
            return out;
        "#;
        let chunk = compile_src(src);
        let mut vm = Vm::new();
        assert_eq!(vm.run(&chunk).unwrap(), VmResult::Waiting { duration_sec: 0.25 });
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

        let meow_ok = compile_src(r#"
            meow {
                raw_teleport(vec2(10.0, 20.0));
            }
        "#);
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
        let err = vm.run_with_host_budget(&chunk, &mut host, 1_000).unwrap_err();
        assert!(err.message.contains("budget exceeded"));
    }

    #[test]
    fn test_globals_accessible_inside_functions() {
        let src = r#"
            let speed = 4.0;
            let mut counter = 0;

            fn bump(v: f64) -> f64 {
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


}
