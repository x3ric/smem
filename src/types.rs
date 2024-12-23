
#[derive(Clone)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone)]
pub struct RegionGroup {
    pub name: String,
    pub enabled: bool,
    pub regions: Vec<MemoryRegion>,
}

#[derive(Clone, Copy)]
pub enum ValueType {
    Int(i32),
    Float(f32),
}

impl ValueType {
    pub fn from_bytes(bytes: [u8; 4], as_float: bool) -> Self {
        if as_float {
            Self::Float(f32::from_ne_bytes(bytes))
        } else {
            Self::Int(i32::from_ne_bytes(bytes))
        }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        match *self {
            Self::Int(x) => x.to_ne_bytes(),
            Self::Float(x) => x.to_ne_bytes(),
        }
    }

    pub fn equals(&self, other: &ValueType) -> bool {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => (a - b).abs() < f32::EPSILON,
            _ => false,
        }
    }

    pub fn greater(&self, other: &ValueType) -> bool {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => a > b,
            (Self::Float(a), Self::Float(b)) => a > b,
            _ => false,
        }
    }

    pub fn less(&self, other: &ValueType) -> bool {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => a < b,
            (Self::Float(a), Self::Float(b)) => a < b,
            _ => false,
        }
    }

    pub fn add(&self, other: &ValueType) -> ValueType {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a + b),
            (Self::Float(a), Self::Float(b)) => Self::Float(a + b),
            _ => *self,
        }
    }

    pub fn sub(&self, other: &ValueType) -> ValueType {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a - b),
            (Self::Float(a), Self::Float(b)) => Self::Float(a - b),
            _ => *self,
        }
    }
}

pub fn parse_user_value(input: &str) -> Option<ValueType> {
    let trimmed = input.trim();
    if trimmed.starts_with("0x") {
        if let Ok(i) = i32::from_str_radix(&trimmed[2..], 16) {
            return Some(ValueType::Int(i));
        }
    }
    if let Ok(f) = trimmed.parse::<f32>() {
        if trimmed.contains('.') {
            return Some(ValueType::Float(f));
        } else {
            return Some(ValueType::Int(f as i32));
        }
    }
    if let Ok(i) = trimmed.parse::<i32>() {
        return Some(ValueType::Int(i));
    }
    None
}
