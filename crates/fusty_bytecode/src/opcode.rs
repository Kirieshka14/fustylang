#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Constants & Stack
    Const = 0,
    Nil,
    True,
    False,
    Pop,
    PopUnder, // operand: u16 count of slots to drop underneath top-of-stack value

    // Locals & Globals
    GetLocal,
    SetLocal,
    GetGlobal,
    SetGlobal,

    // Math & Logic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Not,

    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,


    Halt,
}
