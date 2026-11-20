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


}
