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


}
