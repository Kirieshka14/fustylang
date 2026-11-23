use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len > 0.00001 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self { x: 0.0, y: 0.0 }
        }
    }

    pub fn dot(&self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len > 0.00001 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
        }
    }

    pub fn dot(&self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: Vec3) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    V2(Vec2),
    V3(Vec3),
    String(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::V2(v) => write!(f, "vec2({:.2}, {:.2})", v.x, v.y),
            Value::V3(v) => write!(f, "vec3({:.2}, {:.2}, {:.2})", v.x, v.y, v.z),
            Value::String(s) => write!(f, "{}", s),
        }
    }
}

impl Value {
    #[inline]
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            _ => true,
        }
    }

    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn add(&self, rhs: &Value) -> Result<Value, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => a
                .checked_add(*b)
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in addition".to_string()),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::V2(a), Value::V2(b)) => Ok(Value::V2(*a + *b)),
            (Value::V3(a), Value::V3(b)) => Ok(Value::V3(*a + *b)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (a, b) => Err(format!("Cannot add {} and {}", a, b)),
        }
    }

    pub fn sub(&self, rhs: &Value) -> Result<Value, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => a
                .checked_sub(*b)
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in subtraction".to_string()),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            (Value::V2(a), Value::V2(b)) => Ok(Value::V2(*a - *b)),
            (Value::V3(a), Value::V3(b)) => Ok(Value::V3(*a - *b)),
            (a, b) => Err(format!("Cannot subtract {} and {}", a, b)),
        }
    }

    pub fn mul(&self, rhs: &Value) -> Result<Value, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => a
                .checked_mul(*b)
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in multiplication".to_string()),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            (Value::V2(a), Value::Float(b)) => Ok(Value::V2(*a * (*b as f32))),
            (Value::Float(a), Value::V2(b)) => Ok(Value::V2(*b * (*a as f32))),
            (Value::V2(a), Value::Int(b)) => Ok(Value::V2(*a * (*b as f32))),
            (Value::Int(a), Value::V2(b)) => Ok(Value::V2(*b * (*a as f32))),
            (Value::V3(a), Value::Float(b)) => Ok(Value::V3(*a * (*b as f32))),
            (Value::Float(a), Value::V3(b)) => Ok(Value::V3(*b * (*a as f32))),
            (Value::V3(a), Value::Int(b)) => Ok(Value::V3(*a * (*b as f32))),
            (Value::Int(a), Value::V3(b)) => Ok(Value::V3(*b * (*a as f32))),
            (a, b) => Err(format!("Cannot multiply {} and {}", a, b)),
        }
    }

    pub fn div(&self, rhs: &Value) -> Result<Value, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("B-baka! Division by zero?! Are you trying to break me?! >///<".into())
                } else {
                    a.checked_div(*b)
                        .map(Value::Int)
                        .ok_or_else(|| "Hmph! Integer overflow in division, dummy! >.<".to_string())
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("B-baka! Division by zero?! Are you trying to break me?! >///<".into())
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Division by zero".into())
                } else {
                    Ok(Value::Float(*a as f64 / b))
                }
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Division by zero".into())
                } else {
                    Ok(Value::Float(a / *b as f64))
                }
            }
            (Value::V2(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Division by zero".into())
                } else {
                    Ok(Value::V2(*a / (*b as f32)))
                }
            }
            (Value::V2(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Division by zero".into())
                } else {
                    Ok(Value::V2(*a / (*b as f32)))
                }
            }
            (Value::V3(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Division by zero".into())
                } else {
                    Ok(Value::V3(*a / (*b as f32)))
                }
            }
            (Value::V3(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Division by zero".into())
                } else {
                    Ok(Value::V3(*a / (*b as f32)))
                }
            }
            (a, b) => Err(format!("Cannot divide {} by {}", a, b)),
        }
    }

    pub fn rem(&self, rhs: &Value) -> Result<Value, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Modulo by zero".into())
                } else {
                    a.checked_rem(*b)
                        .map(Value::Int)
                        .ok_or_else(|| "Integer overflow in modulo".to_string())
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Modulo by zero".into())
                } else {
                    Ok(Value::Float(a % b))
                }
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Modulo by zero".into())
                } else {
                    Ok(Value::Float((*a as f64) % b))
                }
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    Err("Modulo by zero".into())
                } else {
                    Ok(Value::Float(a % (*b as f64)))
                }
            }
            (a, b) => Err(format!("Cannot compute remainder of {} and {}", a, b)),
        }
    }

    pub fn neg(&self) -> Result<Value, String> {
        match self {
            Value::Int(i) => i
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in negation".to_string()),
            Value::Float(f) => Ok(Value::Float(-f)),
            Value::V2(v) => Ok(Value::V2(Vec2::new(-v.x, -v.y))),
            Value::V3(v) => Ok(Value::V3(Vec3::new(-v.x, -v.y, -v.z))),
            other => Err(format!("Cannot negate {}", other)),
        }
    }

    pub fn equals(&self, rhs: &Value) -> bool {
        match (self, rhs) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::V2(a), Value::V2(b)) => a == b,
            (Value::V3(a), Value::V3(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }

    pub fn cmp_ord(&self, rhs: &Value) -> Result<Ordering, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b)),
            (Value::Float(a), Value::Float(b)) => a
                .partial_cmp(b)
                .ok_or_else(|| "Cannot compare NaN values".to_string()),
            (Value::Int(a), Value::Float(b)) => (*a as f64)
                .partial_cmp(b)
                .ok_or_else(|| "Cannot compare NaN values".to_string()),
            (Value::Float(a), Value::Int(b)) => a
                .partial_cmp(&(*b as f64))
                .ok_or_else(|| "Cannot compare NaN values".to_string()),
            (Value::String(a), Value::String(b)) => Ok(a.cmp(b)),
            (a, b) => Err(format!("Cannot compare {} and {}", a, b)),
        }
    }
}
