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

