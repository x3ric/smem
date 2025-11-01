#[derive(Clone)]
pub struct MemoryRegion{pub start:usize,pub end:usize}

#[derive(Clone)]
pub struct RegionGroup{pub name:String,pub enabled:bool,pub regions:Vec<MemoryRegion>}

#[derive(Debug,Clone,PartialEq)]
pub enum ValueType{
    Int8(i8),Int16(i16),Int32(i32),Int64(i64),
    UInt8(u8),UInt16(u16),UInt32(u32),UInt64(u64),
    Float32(f32),Float64(f64),
    Size(usize),Pointer(usize),
    Bool(bool),
}
impl ValueType{
    #[inline] fn r<const N:usize>(b:&[u8])->[u8;N]{let mut a=[0u8;N];let n=b.len().min(N);a[..n].copy_from_slice(&b[..n]);a}
    #[cfg(target_pointer_width="64")] #[inline] fn read_usize(b:&[u8])->usize{usize::from_ne_bytes(Self::r::<8>(b))}
    #[cfg(target_pointer_width="32")] #[inline] fn read_usize(b:&[u8])->usize{usize::from_ne_bytes(Self::r::<4>(b))}

    pub fn to_bytes(&self)->Vec<u8>{
        match *self{
            Self::Int8(x)=>vec![x as u8],
            Self::UInt8(x)=>vec![x],
            Self::Bool(x)=>vec![x as u8],
            Self::Int16(x)=>x.to_ne_bytes().to_vec(),
            Self::UInt16(x)=>x.to_ne_bytes().to_vec(),
            Self::Int32(x)=>x.to_ne_bytes().to_vec(),
            Self::UInt32(x)=>x.to_ne_bytes().to_vec(),
            Self::Float32(x)=>x.to_ne_bytes().to_vec(),
            Self::Int64(x)=>x.to_ne_bytes().to_vec(),
            Self::UInt64(x)=>x.to_ne_bytes().to_vec(),
            Self::Float64(x)=>x.to_ne_bytes().to_vec(),
            Self::Size(x)|Self::Pointer(x)=>x.to_ne_bytes().to_vec(),
        }
    }
    pub fn from_bytes(bytes:Vec<u8>,hint:ValueType)->Self{
        match hint{
            Self::Int8(_)=>Self::Int8(i8::from_ne_bytes(Self::r::<1>(&bytes))),
            Self::UInt8(_)=>Self::UInt8(Self::r::<1>(&bytes)[0]),
            Self::Bool(_)=>Self::Bool(Self::r::<1>(&bytes)[0]!=0),
            Self::Int16(_)=>Self::Int16(i16::from_ne_bytes(Self::r::<2>(&bytes))),
            Self::UInt16(_)=>Self::UInt16(u16::from_ne_bytes(Self::r::<2>(&bytes))),
            Self::Int32(_)=>Self::Int32(i32::from_ne_bytes(Self::r::<4>(&bytes))),
            Self::UInt32(_)=>Self::UInt32(u32::from_ne_bytes(Self::r::<4>(&bytes))),
            Self::Float32(_)=>Self::Float32(f32::from_ne_bytes(Self::r::<4>(&bytes))),
            Self::Int64(_)=>Self::Int64(i64::from_ne_bytes(Self::r::<8>(&bytes))),
            Self::UInt64(_)=>Self::UInt64(u64::from_ne_bytes(Self::r::<8>(&bytes))),
            Self::Float64(_)=>Self::Float64(f64::from_ne_bytes(Self::r::<8>(&bytes))),
            Self::Size(_)=>Self::Size(Self::read_usize(&bytes)),
            Self::Pointer(_)=>Self::Pointer(Self::read_usize(&bytes)),
        }
    }
    pub fn scan_types(v:&ValueType)->Vec<ValueType>{
        use ValueType::*;match v{
            Int8(_)=>vec![Int8(0),Int16(0),Int32(0)],
            Int16(_)=>vec![Int16(0),Int8(0),Int32(0)],
            Int32(_)=>vec![Int32(0),Int64(0),Int16(0)],
            Int64(_)=>vec![Int64(0),Int32(0)],
            UInt8(_)=>vec![UInt8(0),UInt16(0),UInt32(0)],
            UInt16(_)=>vec![UInt16(0),UInt8(0),UInt32(0)],
            UInt32(_)=>vec![UInt32(0),UInt64(0),UInt16(0)],
            UInt64(_)=>vec![UInt64(0),UInt32(0)],
            Float32(_)=>vec![Float32(0.0),Float64(0.0)],
            Float64(_)=>vec![Float64(0.0),Float32(0.0)],
            Size(_)=>vec![Size(0),UInt64(0),Int64(0)],
            Pointer(_)=>vec![Pointer(0),UInt64(0)],
            Bool(_)=>vec![Bool(false),UInt8(0)],
        }
    }
    pub fn type_size(h:&ValueType)->usize{
        use ValueType::*;match h{
            Int8(_)|UInt8(_)|Bool(_)=>1,
            Int16(_)|UInt16(_)=>2,
            Int32(_)|UInt32(_)|Float32(_)=>4,
            Int64(_)|UInt64(_)|Float64(_)=>8,
            Size(_)|Pointer(_)=>std::mem::size_of::<usize>(),
        }
    }
    pub fn type_to_string(t:&ValueType)->&'static str{
        use ValueType::*;match t{
            Int8(_)=>"Int8",Int16(_)=>"Int16",Int32(_)=>"Int32",Int64(_)=>"Int64",
            UInt8(_)=>"UInt8",UInt16(_)=>"UInt16",UInt32(_)=>"UInt32",UInt64(_)=>"UInt64",
            Float32(_)=>"Float32",Float64(_)=>"Float64",
            Size(_)=>"Size",Pointer(_)=>"Pointer",Bool(_)=>"Bool",
        }
    }
    pub fn string_to_type(s:&str)->Option<ValueType>{
        use ValueType::*;Some(match s{
            "Int8"=>Int8(0),"Int16"=>Int16(0),"Int32"=>Int32(0),"Int64"=>Int64(0),
            "UInt8"=>UInt8(0),"UInt16"=>UInt16(0),"UInt32"=>UInt32(0),"UInt64"=>UInt64(0),
            "Float32"=>Float32(0.0),"Float64"=>Float64(0.0),
            "Size"=>Size(0),"Pointer"=>Pointer(0),"Bool"=>Bool(false),
            _=>return None
        })
    }
    #[inline] fn float_eq(a:f32,b:f32)->bool{(a-b).abs()<f32::EPSILON}
    #[inline] fn double_eq(a:f64,b:f64)->bool{(a-b).abs()<f64::EPSILON}

    pub fn equals(&self,o:&ValueType)->bool{
        use ValueType::*;
        match(self,o){
            (Int8(a),Int8(b))=>a==b,
            (UInt8(a),UInt8(b))=>a==b,
            (Bool(a),Bool(b))=>a==b,
            (Int16(a),Int16(b))=>a==b,
            (UInt16(a),UInt16(b))=>a==b,
            (Int32(a),Int32(b))=>a==b,
            (UInt32(a),UInt32(b))=>a==b,
            (Int64(a),Int64(b))=>a==b,
            (UInt64(a),UInt64(b))=>a==b,
            (Float32(a),Float32(b))=>Self::float_eq(*a,*b),
            (Float64(a),Float64(b))=>Self::double_eq(*a,*b),
            (Size(a),Size(b))=>a==b,
            (Pointer(a),Pointer(b))=>a==b,
            _=>false
        }
    }
    pub fn greater(&self,o:&ValueType)->bool{
        use ValueType::*;
        match(self,o){
            (Int8(a),Int8(b))=>a>b,
            (UInt8(a),UInt8(b))=>a>b,
            (Int16(a),Int16(b))=>a>b,
            (UInt16(a),UInt16(b))=>a>b,
            (Int32(a),Int32(b))=>a>b,
            (UInt32(a),UInt32(b))=>a>b,
            (Int64(a),Int64(b))=>a>b,
            (UInt64(a),UInt64(b))=>a>b,
            (Float32(a),Float32(b))=>!Self::float_eq(*a,*b)&&a>b,
            (Float64(a),Float64(b))=>!Self::double_eq(*a,*b)&&a>b,
            (Size(a),Size(b))=>a>b,
            (Pointer(a),Pointer(b))=>a>b,
            (Bool(a),Bool(b))=>*a&&!*b,
            _=>false
        }
    }
    pub fn less(&self,o:&ValueType)->bool{
        use ValueType::*;
        match(self,o){
            (Int8(a),Int8(b))=>a<b,
            (UInt8(a),UInt8(b))=>a<b,
            (Int16(a),Int16(b))=>a<b,
            (UInt16(a),UInt16(b))=>a<b,
            (Int32(a),Int32(b))=>a<b,
            (UInt32(a),UInt32(b))=>a<b,
            (Int64(a),Int64(b))=>a<b,
            (UInt64(a),UInt64(b))=>a<b,
            (Float32(a),Float32(b))=>!Self::float_eq(*a,*b)&&a<b,
            (Float64(a),Float64(b))=>!Self::double_eq(*a,*b)&&a<b,
            (Size(a),Size(b))=>a<b,
            (Pointer(a),Pointer(b))=>a<b,
            (Bool(a),Bool(b))=>!*a&&*b,
            _=>false
        }
    }
    pub fn add(&self,o:&ValueType)->Option<ValueType>{
        use ValueType::*;Some(match(self,o){
            (Int8(a),Int8(b))=>Int8(a.wrapping_add(*b)),
            (UInt8(a),UInt8(b))=>UInt8(a.wrapping_add(*b)),
            (Int16(a),Int16(b))=>Int16(a.wrapping_add(*b)),
            (UInt16(a),UInt16(b))=>UInt16(a.wrapping_add(*b)),
            (Int32(a),Int32(b))=>Int32(a.wrapping_add(*b)),
            (UInt32(a),UInt32(b))=>UInt32(a.wrapping_add(*b)),
            (Int64(a),Int64(b))=>Int64(a.wrapping_add(*b)),
            (UInt64(a),UInt64(b))=>UInt64(a.wrapping_add(*b)),
            (Float32(a),Float32(b))=>Float32(a+b),
            (Float64(a),Float64(b))=>Float64(a+b),
            (Size(a),Size(b))=>Size(a+b),
            (Pointer(a),Pointer(b))=>Pointer(a+b),
            (Bool(a),Bool(b))=>Bool(*a||*b),
            _=>return None
        })
    }
    pub fn sub(&self,o:&ValueType)->Option<ValueType>{
        use ValueType::*;Some(match(self,o){
            (Int8(a),Int8(b))=>Int8(a.wrapping_sub(*b)),
            (UInt8(a),UInt8(b))=>UInt8(a.wrapping_sub(*b)),
            (Int16(a),Int16(b))=>Int16(a.wrapping_sub(*b)),
            (UInt16(a),UInt16(b))=>UInt16(a.wrapping_sub(*b)),
            (Int32(a),Int32(b))=>Int32(a.wrapping_sub(*b)),
            (UInt32(a),UInt32(b))=>UInt32(a.wrapping_sub(*b)),
            (Int64(a),Int64(b))=>Int64(a.wrapping_sub(*b)),
            (UInt64(a),UInt64(b))=>UInt64(a.wrapping_sub(*b)),
            (Float32(a),Float32(b))=>Float32(a-b),
            (Float64(a),Float64(b))=>Float64(a-b),
            (Size(a),Size(b))=>Size(a-b),
            (Pointer(a),Pointer(b))=>Pointer(a-b),
            (Bool(a),Bool(b))=>Bool(*a&&!*b),
            _=>return None
        })
    }
    pub fn comparator(mode:&str,old:&ValueType,new:&ValueType,inp:&ValueType)->bool{
        match mode{
            "Exact"=>new.equals(inp),
            "Changed"=>!new.equals(old),
            "Unchanged"=>new.equals(old),
            "Increased"=>new.greater(old),
            "Increased or Greater"=>new.greater(old)||new.equals(old),
            "Increased by"=>old.add(inp).map_or(false,|v|new.equals(&v)),
            "Decreased"=>new.less(old),
            "Decreased or Less"=>new.less(old)||new.equals(old),
            "Decreased by"=>old.sub(inp).map_or(false,|v|new.equals(&v)),
            _=>false
        }
    }
    pub fn parse_user_value(input:&str)->Option<ValueType>{
        let t=input.trim();
        match t.split_once(':'){
            Some(("bool"|"boolean",v))=>v.parse().ok().map(ValueType::Bool),
            Some(("byte"|"b",v))=>v.parse().ok().map(ValueType::UInt8),
            Some(("hex"|"h",v))=>u64::from_str_radix(v,16).ok().map(|i|if i<=u8::MAX as u64{ValueType::UInt8(i as u8)}else if i<=u16::MAX as u64{ValueType::UInt16(i as u16)}else if i<=u32::MAX as u64{ValueType::UInt32(i as u32)}else{ValueType::UInt64(i)}),
            Some(("int8"|"i8"|"char",v))=>v.parse().ok().map(ValueType::Int8),
            Some(("int16"|"i16"|"short",v))=>v.parse().ok().map(ValueType::Int16),
            Some(("int32"|"i32"|"int",v))=>v.parse().ok().map(ValueType::Int32),
            Some(("int64"|"i64"|"long",v))=>v.parse().ok().map(ValueType::Int64),
            Some(("float32"|"f32"|"float",v))=>v.parse().ok().map(ValueType::Float32),
            Some(("float64"|"f64"|"double",v))=>v.parse().ok().map(ValueType::Float64),
            Some(("size"|"s",v))=>v.parse().ok().map(ValueType::Size),
            Some(("ptr"|"pointer",v))=>v.parse().ok().map(ValueType::Pointer),
            _=>t.parse::<i64>().ok().map(|n|if (i8::MIN as i64..=i8::MAX as i64).contains(&n){ValueType::Int8(n as i8)}else if (i16::MIN as i64..=i16::MAX as i64).contains(&n){ValueType::Int16(n as i16)}else if (i32::MIN as i64..=i32::MAX as i64).contains(&n){ValueType::Int32(n as i32)}else{ValueType::Int64(n)})
                .or_else(||t.parse::<f64>().ok().map(|n|if n.abs()<=(f32::MAX as f64){ValueType::Float32(n as f32)}else{ValueType::Float64(n)}))
        }
    }
}
