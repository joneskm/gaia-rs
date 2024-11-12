#[derive(Debug, Clone)]
pub enum Params {
    Bool(bool),
    String(String),
    Bytes(Vec<u8>), // This variant usually applies to structures
    U64(u64),
    I64(i64),
    U32(u32),
    I32(i32),
    U16(u16),
    I16(i16),
    U8(u8),
    I8(i8),
    InvalidCast(Vec<u8>),
}

impl Params {
    pub fn boolean(self) -> Option<bool> {
        match self {
            Params::Bool(var) => Some(var),
            _ => None,
        }
    }

    pub fn string(self) -> Option<String> {
        match self {
            Params::String(var) => Some(var),
            _ => None,
        }
    }

    pub fn bytes(self) -> Option<Vec<u8>> {
        match self {
            Params::Bytes(var) => Some(var),
            _ => None,
        }
    }

    pub fn unsigned_64(self) -> Option<u64> {
        match self {
            Params::U64(var) => Some(var),
            _ => None,
        }
    }

    pub fn unsigned_32(self) -> Option<u32> {
        match self {
            Params::U32(var) => Some(var),
            _ => None,
        }
    }

    pub fn unsigned_16(self) -> Option<u16> {
        match self {
            Params::U16(var) => Some(var),
            _ => None,
        }
    }

    pub fn unsigned_8(self) -> Option<u8> {
        match self {
            Params::U8(var) => Some(var),
            _ => None,
        }
    }

    pub fn signed_64(self) -> Option<i64> {
        match self {
            Params::I64(var) => Some(var),
            _ => None,
        }
    }

    pub fn is_valid(&self) -> bool {
        matches!(self, Params::InvalidCast(_))
    }
}
