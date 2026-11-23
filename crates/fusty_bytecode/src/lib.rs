pub mod chunk;
pub mod compiler;
pub mod opcode;

#[cfg(test)]
mod tests {
    use crate::compiler::Compiler;
    use crate::opcode::Opcode;
    use fusty_lexer::Lexer;
    use fusty_parser::Parser;

    #[test]
    fn test_compile_nya_function_bounds_and_baka_opcode() {
        let src = r#"
            nya guard(x: i64) -> i64 {
                if x < 0 {
                    baka!("negative!");
                }
                return x * 2;
            }
            return guard(5);
        "#;
        let tokens = Lexer::new(src).tokenize().expect("lex");
        let program = Parser::new(tokens).parse_program().expect("parse");
        let chunk = Compiler::new().compile_program(&program).expect("compile");

        assert_eq!(chunk.functions.len(), 1);
        let meta = &chunk.functions[0];
        assert_eq!(meta.name, "guard");
        assert_eq!(meta.arity, 1);
        assert!(meta.end_ip > meta.entry_ip);
        assert!(chunk.code[meta.entry_ip..meta.end_ip].contains(&(Opcode::Baka as u8)));
    }

    #[test]
    fn test_duplicate_nya_tsundere_error() {
        let src = "nya dup() { 1 } nya dup() { 2 }";
        let tokens = Lexer::new(src).tokenize().expect("lex");
        let program = Parser::new(tokens).parse_program().expect("parse");
        let err = Compiler::new().compile_program(&program).unwrap_err();
        assert!(err.message.contains("B-baka!"));
    }
}
