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
    Long(i64),
    Float(f32),
    Double(f64),
}

impl ValueType {
    pub fn from_bytes(bytes: Vec<u8>, type_hint: ValueType) -> Self {
        match type_hint {
            ValueType::Int(_) => Self::Int(i32::from_ne_bytes(bytes[0..4].try_into().unwrap())),
            ValueType::Long(_) => Self::Long(i64::from_ne_bytes(bytes[0..8].try_into().unwrap())),
            ValueType::Float(_) => Self::Float(f32::from_ne_bytes(bytes[0..4].try_into().unwrap())),
            ValueType::Double(_) => Self::Double(f64::from_ne_bytes(bytes[0..8].try_into().unwrap())),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match *self {
            Self::Int(x) => x.to_ne_bytes().to_vec(),
            Self::Long(x) => x.to_ne_bytes().to_vec(),
            Self::Float(x) => x.to_ne_bytes().to_vec(),
            Self::Double(x) => x.to_ne_bytes().to_vec(),
        }
    }

    const FLOAT_EPSILON: f32 = 1e-5;
    const DOUBLE_EPSILON: f64 = 1e-10;

    fn float_eq(a: f32, b: f32) -> bool {
        (a - b).abs() <= Self::FLOAT_EPSILON * (1.0 + a.abs() + b.abs())
    }

    fn double_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= Self::DOUBLE_EPSILON * (1.0 + a.abs() + b.abs())
    }

    pub fn equals(&self, other: &ValueType) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Long(a), Self::Long(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => Self::float_eq(*a, *b),
            (Self::Double(a), Self::Double(b)) => Self::double_eq(*a, *b),
            (Self::Int(a), Self::Long(b)) => (*a as i64) == *b,
            (Self::Long(a), Self::Int(b)) => *a == (*b as i64),
            (Self::Float(a), Self::Double(b)) => Self::double_eq(*a as f64, *b),
            (Self::Double(a), Self::Float(b)) => Self::double_eq(*a, *b as f64),
            _ => false,
        }
    }

    pub fn greater(&self, other: &ValueType) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a > b,
            (Self::Long(a), Self::Long(b)) => a > b,
            (Self::Float(a), Self::Float(b)) => {
                !Self::float_eq(*a, *b) && a > b
            },
            (Self::Double(a), Self::Double(b)) => {
                !Self::double_eq(*a, *b) && a > b
            },
            (Self::Int(a), Self::Long(b)) => (*a as i64) > *b,
            (Self::Long(a), Self::Int(b)) => *a > (*b as i64),
            (Self::Float(a), Self::Double(b)) => {
                let a_d = *a as f64;
                !Self::double_eq(a_d, *b) && a_d > *b
            },
            (Self::Double(a), Self::Float(b)) => {
                let b_d = *b as f64;
                !Self::double_eq(*a, b_d) && *a > b_d
            },
            _ => false,
        }
    }

    pub fn less(&self, other: &ValueType) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a < b,
            (Self::Long(a), Self::Long(b)) => a < b,
            (Self::Float(a), Self::Float(b)) => {
                !Self::float_eq(*a, *b) && a < b
            },
            (Self::Double(a), Self::Double(b)) => {
                !Self::double_eq(*a, *b) && a < b
            },
            (Self::Int(a), Self::Long(b)) => (*a as i64) < *b,
            (Self::Long(a), Self::Int(b)) => *a < (*b as i64),
            (Self::Float(a), Self::Double(b)) => {
                let a_d = *a as f64;
                !Self::double_eq(a_d, *b) && a_d < *b
            },
            (Self::Double(a), Self::Float(b)) => {
                let b_d = *b as f64;
                !Self::double_eq(*a, b_d) && *a < b_d
            },
            _ => false,
        }
    }

    pub fn add(&self, other: &ValueType) -> ValueType {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a + b),
            (Self::Long(a), Self::Long(b)) => Self::Long(a + b),
            (Self::Float(a), Self::Float(b)) => Self::Float(a + b),
            (Self::Double(a), Self::Double(b)) => Self::Double(a + b),
            (Self::Int(a), Self::Long(b)) => Self::Long(*a as i64 + *b),
            (Self::Long(a), Self::Int(b)) => Self::Long(*a + *b as i64),
            (Self::Float(a), Self::Double(b)) => Self::Double(*a as f64 + *b),
            (Self::Double(a), Self::Float(b)) => Self::Double(*a + *b as f64),
            _ => *self,
        }
    }

    pub fn sub(&self, other: &ValueType) -> ValueType {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a - b),
            (Self::Long(a), Self::Long(b)) => Self::Long(a - b),
            (Self::Float(a), Self::Float(b)) => Self::Float(a - b),
            (Self::Double(a), Self::Double(b)) => Self::Double(a - b),
            (Self::Int(a), Self::Long(b)) => Self::Long(*a as i64 - *b),
            (Self::Long(a), Self::Int(b)) => Self::Long(*a - *b as i64),
            (Self::Float(a), Self::Double(b)) => Self::Double(*a as f64 - *b),
            (Self::Double(a), Self::Float(b)) => Self::Double(*a - *b as f64),
            _ => *self,
        }
    }
}

pub fn parse_user_value(input: &str) -> Option<ValueType> {
    let trimmed = input.trim();
    if let Some(hex_part) = trimmed.strip_prefix("0x") {
        if let Ok(i) = i64::from_str_radix(hex_part, 16) {
            return Some(if (i32::MIN as i64..=i32::MAX as i64).contains(&i) {
                ValueType::Int(i as i32)
            } else {
                ValueType::Long(i)
            });
        }
    }
    let numeric_part = if let Some(num) = trimmed.strip_prefix('d') {
        num
    } else {
        trimmed
    };
    if numeric_part.contains('.') {
        if trimmed.starts_with('d') {
            if let Ok(num) = numeric_part.parse::<f64>() {
                return Some(ValueType::Double(num));
            }
        }
        if let Ok(num) = numeric_part.trim_end_matches('0').trim_end_matches('.').parse::<f32>() {
            return Some(ValueType::Float(num));
        }
        if let Ok(num) = numeric_part.parse::<f64>() {
            return Some(ValueType::Double(num));
        }
    }
    if let Ok(num) = numeric_part.parse::<i32>() {
        return Some(ValueType::Int(num));
    }
    if let Ok(num) = numeric_part.parse::<i64>() {
        return Some(ValueType::Long(num));
    }
    None
}
