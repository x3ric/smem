#[derive(Clone)]
pub struct MemoryRegion {
    pub start: usize, // Start address
    pub end: usize,   // End address
}

#[derive(Clone)]
pub struct RegionGroup {
    pub name: String,               // Region name
    pub enabled: bool,              // Is region enabled?
    pub regions: Vec<MemoryRegion>, // List of memory regions
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Int8(i8), Int16(i16), Int32(i32), Int64(i64),     // Signed integers
    UInt8(u8), UInt16(u16), UInt32(u32), UInt64(u64), // Unsigned integers
    Float32(f32), Float64(f64),                       // Floating-point types
    Size(usize), Pointer(usize),                      // Special numeric types like size and pointers
    Bool(bool),                                       // Boolean type
}

impl ValueType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match *self {
            Self::Int32(x) => x.to_ne_bytes().to_vec(),
            Self::Int64(x) => x.to_ne_bytes().to_vec(),
            Self::Float32(x) => x.to_ne_bytes().to_vec(),
            Self::Float64(x) => x.to_ne_bytes().to_vec(),
            Self::UInt8(x) => vec![x],
            Self::UInt16(x) => x.to_ne_bytes().to_vec(),
            Self::UInt32(x) => x.to_ne_bytes().to_vec(),
            Self::UInt64(x) => x.to_ne_bytes().to_vec(),
            Self::Int8(x) => vec![x as u8],
            Self::Int16(x) => x.to_ne_bytes().to_vec(),
            Self::Size(x) => x.to_ne_bytes().to_vec(),
            Self::Pointer(x) => x.to_ne_bytes().to_vec(),
            Self::Bool(x) => vec![x as u8],
        }
    }

    pub fn from_bytes(bytes: Vec<u8>, type_hint: ValueType) -> Self {
        match type_hint {
            Self::Int32(_) => Self::Int32(i32::from_ne_bytes(bytes[0..4].try_into().unwrap())),
            Self::Int64(_) => Self::Int64(i64::from_ne_bytes(bytes[0..8].try_into().unwrap())),
            Self::Float32(_) => Self::Float32(f32::from_ne_bytes(bytes[0..4].try_into().unwrap())),
            Self::Float64(_) => Self::Float64(f64::from_ne_bytes(bytes[0..8].try_into().unwrap())),
            Self::UInt8(_) => Self::UInt8(bytes[0]),
            Self::UInt16(_) => Self::UInt16(u16::from_ne_bytes(bytes[0..2].try_into().unwrap())),
            Self::UInt32(_) => Self::UInt32(u32::from_ne_bytes(bytes[0..4].try_into().unwrap())),
            Self::UInt64(_) => Self::UInt64(u64::from_ne_bytes(bytes[0..8].try_into().unwrap())),
            Self::Int8(_) => Self::Int8(i8::from_ne_bytes(bytes[0..1].try_into().unwrap())),
            Self::Int16(_) => Self::Int16(i16::from_ne_bytes(bytes[0..2].try_into().unwrap())),
            Self::Size(_) => Self::Size(usize::from_ne_bytes(bytes[0..8].try_into().unwrap())),
            Self::Pointer(_) => Self::Pointer(usize::from_ne_bytes(bytes[0..8].try_into().unwrap())),
            Self::Bool(_) => Self::Bool(bytes[0] != 0),
            _ => unimplemented!(),
        }
    }

    pub fn scan_types(val: &ValueType) -> Vec<ValueType> {
        match val {
            ValueType::Int8(_) => vec![ValueType::Int8(0), ValueType::Int16(0), ValueType::Int32(0)],
            ValueType::Int16(_) => vec![ValueType::Int16(0), ValueType::Int8(0), ValueType::Int32(0)],
            ValueType::Int32(_) => vec![ValueType::Int32(0), ValueType::Int64(0), ValueType::Int16(0)],
            ValueType::Int64(_) => vec![ValueType::Int64(0), ValueType::Int32(0)],
            ValueType::UInt8(_) => vec![ValueType::UInt8(0), ValueType::UInt16(0), ValueType::UInt32(0)],
            ValueType::UInt16(_) => vec![ValueType::UInt16(0), ValueType::UInt8(0), ValueType::UInt32(0)],
            ValueType::UInt32(_) => vec![ValueType::UInt32(0), ValueType::UInt64(0), ValueType::UInt16(0)],
            ValueType::UInt64(_) => vec![ValueType::UInt64(0), ValueType::UInt32(0)],
            ValueType::Float32(_) => vec![ValueType::Float32(0.0), ValueType::Float64(0.0)],
            ValueType::Float64(_) => vec![ValueType::Float64(0.0), ValueType::Float32(0.0)],
            ValueType::Size(_) => vec![ValueType::Size(0), ValueType::UInt64(0), ValueType::Int64(0)],
            ValueType::Pointer(_) => vec![ValueType::Pointer(0), ValueType::UInt64(0)],
            ValueType::Bool(_) => vec![ValueType::Bool(false), ValueType::UInt8(0)],
        }
    }

    pub fn type_size(type_hint: &ValueType) -> usize {
        match type_hint {
            ValueType::Int8(_) | ValueType::UInt8(_) | ValueType::Bool(_) => 1,
            ValueType::Int16(_) | ValueType::UInt16(_) => 2,
            ValueType::Int32(_) | ValueType::UInt32(_) | ValueType::Float32(_) => 4,
            ValueType::Int64(_) | ValueType::UInt64(_) | ValueType::Float64(_) | ValueType::Size(_) | ValueType::Pointer(_) => 8,
        }
    }

    pub fn type_to_string(value_type: &ValueType) -> &'static str {
        match value_type {
            ValueType::Int8(_) => "Int8",
            ValueType::Int16(_) => "Int16",
            ValueType::Int32(_) => "Int32",
            ValueType::Int64(_) => "Int64",
            ValueType::UInt8(_) => "UInt8",
            ValueType::UInt16(_) => "UInt16",
            ValueType::UInt32(_) => "UInt32",
            ValueType::UInt64(_) => "UInt64",
            ValueType::Float32(_) => "Float32",
            ValueType::Float64(_) => "Float64",
            ValueType::Size(_) => "Size",
            ValueType::Pointer(_) => "Pointer",
            ValueType::Bool(_) => "Bool",
        }
    }

    pub fn string_to_type(type_str: &str) -> Option<ValueType> {
        match type_str {
            "Int8" => Some(ValueType::Int8(0)),
            "Int16" => Some(ValueType::Int16(0)),
            "Int32" => Some(ValueType::Int32(0)),
            "Int64" => Some(ValueType::Int64(0)),
            "UInt8" => Some(ValueType::UInt8(0)),
            "UInt16" => Some(ValueType::UInt16(0)),
            "UInt32" => Some(ValueType::UInt32(0)),
            "UInt64" => Some(ValueType::UInt64(0)),
            "Float32" => Some(ValueType::Float32(0.0)),
            "Float64" => Some(ValueType::Float64(0.0)),
            "Size" => Some(ValueType::Size(0)),
            "Pointer" => Some(ValueType::Pointer(0)),
            "Bool" => Some(ValueType::Bool(false)),
            _ => None,
        }
    }

    fn float_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < f32::EPSILON
    }

    fn double_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

    pub fn equals(&self, other: &ValueType) -> bool {
        use ValueType::*;
        match (self, other) {
            (Int8(a), Int8(b)) => a == b,
            (UInt8(a), UInt8(b)) => a == b,
            (Bool(a), Bool(b)) => a == b,
            (Int16(a), Int16(b)) => a == b,
            (UInt16(a), UInt16(b)) => a == b,
            (Int32(a), Int32(b)) => a == b,
            (UInt32(a), UInt32(b)) => a == b,
            (Int64(a), Int64(b)) => a == b,
            (UInt64(a), UInt64(b)) => a == b,
            (Float32(a), Float32(b)) => Self::float_eq(*a, *b),
            (Float64(a), Float64(b)) => Self::double_eq(*a, *b),
            (Size(a), Size(b)) => a == b,
            (Pointer(a), Pointer(b)) => a == b,
            _ => false,
        }
    }

    pub fn greater(&self, other: &ValueType) -> bool {
        use ValueType::*;
        match (self, other) {
            (Int8(a), Int8(b)) => a > b,
            (UInt8(a), UInt8(b)) => a > b,
            (Int16(a), Int16(b)) => a > b,
            (UInt16(a), UInt16(b)) => a > b,
            (Int32(a), Int32(b)) => a > b,
            (UInt32(a), UInt32(b)) => a > b,
            (Int64(a), Int64(b)) => a > b,
            (UInt64(a), UInt64(b)) => a > b,
            (Float32(a), Float32(b)) => !Self::float_eq(*a, *b) && a > b,
            (Float64(a), Float64(b)) => !Self::double_eq(*a, *b) && a > b,
            (Size(a), Size(b)) => a > b,
            (Pointer(a), Pointer(b)) => a > b,
            (Bool(a), Bool(b)) => *a && !*b,
            _ => false,
        }
    }

    pub fn less(&self, other: &ValueType) -> bool {
        use ValueType::*;
        match (self, other) {
            (Int8(a), Int8(b)) => a < b,
            (UInt8(a), UInt8(b)) => a < b,
            (Int16(a), Int16(b)) => a < b,
            (UInt16(a), UInt16(b)) => a < b,
            (Int32(a), Int32(b)) => a < b,
            (UInt32(a), UInt32(b)) => a < b,
            (Int64(a), Int64(b)) => a < b,
            (UInt64(a), UInt64(b)) => a < b,
            (Float32(a), Float32(b)) => !Self::float_eq(*a, *b) && a < b,
            (Float64(a), Float64(b)) => !Self::double_eq(*a, *b) && a < b,
            (Size(a), Size(b)) => a < b,
            (Pointer(a), Pointer(b)) => a < b,
            (Bool(a), Bool(b)) => !*a && *b,
            _ => false,
        }
    }

    pub fn add(&self, other: &ValueType) -> Option<ValueType> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a.wrapping_add(*b))),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a.wrapping_add(*b))),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a.wrapping_add(*b))),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a.wrapping_add(*b))),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a.wrapping_add(*b))),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a.wrapping_add(*b))),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a.wrapping_add(*b))),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a.wrapping_add(*b))),
            (Self::Float32(a), Self::Float32(b)) => Some(Self::Float32(a + b)),
            (Self::Float64(a), Self::Float64(b)) => Some(Self::Float64(a + b)),
            (Self::Size(a), Self::Size(b)) => Some(Self::Size(a + b)),
            (Self::Pointer(a), Self::Pointer(b)) => Some(Self::Pointer(a + b)),
            (Self::Bool(a), Self::Bool(b)) => Some(Self::Bool(*a || *b)),
            _ => None,
        }
    }

    pub fn sub(&self, other: &ValueType) -> Option<ValueType> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a.wrapping_sub(*b))),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a.wrapping_sub(*b))),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a.wrapping_sub(*b))),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a.wrapping_sub(*b))),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a.wrapping_sub(*b))),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a.wrapping_sub(*b))),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a.wrapping_sub(*b))),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a.wrapping_sub(*b))),
            (Self::Float32(a), Self::Float32(b)) => Some(Self::Float32(a - b)),
            (Self::Float64(a), Self::Float64(b)) => Some(Self::Float64(a - b)),
            (Self::Size(a), Self::Size(b)) => Some(Self::Size(a - b)),
            (Self::Pointer(a), Self::Pointer(b)) => Some(Self::Pointer(a - b)),
            (Self::Bool(a), Self::Bool(b)) => Some(Self::Bool(*a && !*b)),
            _ => None,
        }
    }

    pub fn comparator(scan_mode: &str, old_val: &ValueType, new_val: &ValueType, inp: &ValueType) -> bool {
        match scan_mode {
            "Exact" => new_val.equals(inp),
            "Changed" => !new_val.equals(old_val),
            "Unchanged" => new_val.equals(old_val),
            "Increased" => new_val.greater(old_val),
            "Increased or Greater" => new_val.greater(old_val) || new_val.equals(old_val),
            "Increased by" => match old_val.add(inp) {
                Some(val) => new_val.equals(&val),
                None => false,
            },
            "Decreased" => new_val.less(old_val),
            "Decreased or Less" => new_val.less(old_val) || new_val.equals(old_val),
            "Decreased by" => match old_val.sub(inp) {
                Some(val) => new_val.equals(&val),
                None => false,
            },
            _ => false,
        }
    }
    
    pub fn parse_user_value(input: &str) -> Option<ValueType> {
        let trimmed = input.trim();
        match trimmed.split_once(':') {
            Some(("bool", v)) => v.parse().ok().map(ValueType::Bool),
            Some(("byte", v)) => v.parse().ok().map(ValueType::UInt8),
            Some(("hex", v)) => u64::from_str_radix(v, 16).ok().map(|i| match i {
                i if i <= u8::MAX as u64 => ValueType::UInt8(i as u8),
                i if i <= u16::MAX as u64 => ValueType::UInt16(i as u16),
                i if i <= u32::MAX as u64 => ValueType::UInt32(i as u32),
                i => ValueType::UInt64(i),
            }),
            Some(("int8", v)) => v.parse().ok().map(ValueType::Int8),
            Some(("int16", v)) => v.parse().ok().map(ValueType::Int16),
            Some(("int32", v)) => v.parse().ok().map(ValueType::Int32),
            Some(("int64", v)) => v.parse().ok().map(ValueType::Int64),
            Some(("float32", v)) => v.parse().ok().map(ValueType::Float32),
            Some(("float64", v)) => v.parse().ok().map(ValueType::Float64),
            Some(("size", v)) => v.parse().ok().map(ValueType::Size),
            Some(("ptr", v)) => v.parse().ok().map(ValueType::Pointer),
            _ => trimmed.parse::<i64>().ok().map(|n| match n {
                n if n >= i8::MIN as i64 && n <= i8::MAX as i64 => ValueType::Int8(n as i8),
                n if n >= i16::MIN as i64 && n <= i16::MAX as i64 => ValueType::Int16(n as i16),
                n if n >= i32::MIN as i64 && n <= i32::MAX as i64 => ValueType::Int32(n as i32),
                n => ValueType::Int64(n),
            }).or_else(|| trimmed.parse::<f64>().ok().map(|n| 
                if n.abs() <= f32::MAX as f64 { ValueType::Float32(n as f32) } 
                else { ValueType::Float64(n) }
            ))
        }
    }
}
