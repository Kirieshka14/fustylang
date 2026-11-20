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

    // Game-First Vectors
    MakeVec2,
    MakeVec3,
    Vec2Add,
    Vec2Sub,
    Vec2MulScalar,
    Vec2Dot,
    Vec2Length,
    Vec2Normalize,
    Vec3Cross,
    VecGetX,
    VecGetY,
    VecGetZ,
    VecSetX,
    VecSetY,
    VecSetZ,

    // Control Flow
    Jump,
    JumpIfFalse,
    Loop,

    // Coroutines & Game Timings
    Wait,
    Yield,

    // Meow (Unsafe scope)
    MeowEnter,
    MeowExit,

    // Function Calls & Execution
    Call,
    CallNative,
    Return,
    Halt,
}

impl Opcode {
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Const),
            1 => Some(Self::Nil),
            2 => Some(Self::True),
            3 => Some(Self::False),
            4 => Some(Self::Pop),
            5 => Some(Self::PopUnder),
            6 => Some(Self::GetLocal),
            7 => Some(Self::SetLocal),
            8 => Some(Self::GetGlobal),
            9 => Some(Self::SetGlobal),
            10 => Some(Self::Add),
            11 => Some(Self::Sub),
            12 => Some(Self::Mul),
            13 => Some(Self::Div),
            14 => Some(Self::Mod),
            15 => Some(Self::Neg),
            16 => Some(Self::Not),
            17 => Some(Self::Eq),
            18 => Some(Self::NotEq),
            19 => Some(Self::Lt),
            20 => Some(Self::LtEq),
            21 => Some(Self::Gt),
            22 => Some(Self::GtEq),
            23 => Some(Self::MakeVec2),
            24 => Some(Self::MakeVec3),
            25 => Some(Self::Vec2Add),
            26 => Some(Self::Vec2Sub),
            27 => Some(Self::Vec2MulScalar),
            28 => Some(Self::Vec2Dot),
            29 => Some(Self::Vec2Length),
            30 => Some(Self::Vec2Normalize),
            31 => Some(Self::Vec3Cross),
            32 => Some(Self::VecGetX),
            33 => Some(Self::VecGetY),
            34 => Some(Self::VecGetZ),
            35 => Some(Self::VecSetX),
            36 => Some(Self::VecSetY),
            37 => Some(Self::VecSetZ),
            38 => Some(Self::Jump),
            39 => Some(Self::JumpIfFalse),
            40 => Some(Self::Loop),
            41 => Some(Self::Wait),
            42 => Some(Self::Yield),
            43 => Some(Self::MeowEnter),
            44 => Some(Self::MeowExit),
            45 => Some(Self::Call),
            46 => Some(Self::CallNative),
            47 => Some(Self::Return),
            48 => Some(Self::Halt),
            _ => None,
        }
    }
}
