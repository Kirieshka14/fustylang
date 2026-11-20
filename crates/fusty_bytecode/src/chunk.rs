use crate::opcode::Opcode;

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Vec2(f32, f32),
    Vec3(f32, f32, f32),
}

impl Constant {
    fn same_value(&self, other: &Self) -> bool {
        match (self, other) {
            (Constant::Int(a), Constant::Int(b)) => a == b,
            (Constant::Float(a), Constant::Float(b)) => a.to_bits() == b.to_bits(),
            (Constant::Bool(a), Constant::Bool(b)) => a == b,
            (Constant::String(a), Constant::String(b)) => a == b,
            (Constant::Vec2(ax, ay), Constant::Vec2(bx, by)) => {
                ax.to_bits() == bx.to_bits() && ay.to_bits() == by.to_bits()
            }
            (Constant::Vec3(ax, ay, az), Constant::Vec3(bx, by, bz)) => {
                ax.to_bits() == bx.to_bits()
                    && ay.to_bits() == by.to_bits()
                    && az.to_bits() == bz.to_bits()
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionMeta {
    pub name: String,
    pub entry_ip: usize,
    pub arity: u8,
    pub is_meow: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Constant>,
    pub lines: Vec<usize>,
    pub functions: Vec<FunctionMeta>,
    pub global_count: u16,
}

impl Chunk {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_op(&mut self, op: Opcode, line: usize) {
        self.code.push(op as u8);
        self.lines.push(line);
    }

    pub fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn write_u16(&mut self, val: u16, line: usize) {
        self.write_byte((val >> 8) as u8, line);
        self.write_byte((val & 0xFF) as u8, line);
    }

    pub fn try_add_constant(&mut self, constant: Constant) -> Result<u16, String> {
        if let Some(pos) = self.constants.iter().position(|c| c.same_value(&constant)) {
            return Ok(pos as u16);
        }
        if self.constants.len() >= u16::MAX as usize {
            return Err("Too many constants in chunk (max 65535)".into());
        }
        self.constants.push(constant);
        Ok((self.constants.len() - 1) as u16)
    }

    pub fn add_constant(&mut self, constant: Constant) -> u16 {
        self.try_add_constant(constant)
            .expect("Chunk constant pool overflow")
    }

    pub fn write_const(&mut self, constant: Constant, line: usize) {
        let idx = self.add_constant(constant);
        self.write_op(Opcode::Const, line);
        self.write_u16(idx, line);
    }
}
