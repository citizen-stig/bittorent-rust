use crate::bencode::core::BencodeType;
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum ReceivedBencodeType {
    Known(BencodeType),
    Unknown(char),
}

impl From<BencodeType> for ReceivedBencodeType {
    fn from(bencode_type: BencodeType) -> Self {
        ReceivedBencodeType::Known(bencode_type)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum BencodeDeserializationError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("cannot parse int {0}")]
    CannotParseInteger(#[from] std::num::ParseIntError),
    #[error("len separator (':') is missing")]
    LenSeparatorMissing,
    #[error("invalid length declaration, non didit character: {0}")]
    InvalidLen(char),
    #[error("integer contains non digit character: {0}")]
    InvalidInteger(char),
    #[error("integer contains leading zeroes")]
    InvalidIntegerLeadingZero,
    #[error("invalid bencode data: expected {expected:?}, got {actual:?}")]
    UnexpectedBencodeType {
        expected: Option<BencodeType>,
        actual: ReceivedBencodeType,
    },
    #[error("cannot parse str: {0}")]
    InvalidString(#[from] std::str::Utf8Error),
    #[error("invalid map key, it should be byt string, but got {actual:?}")]
    InvalidKey { actual: ReceivedBencodeType },
    #[error("custom: {0}")]
    Custom(&'static str),
}

// TODO: Move to serde
impl serde::de::Error for BencodeDeserializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        println!("DESER ERROR: {}", msg);
        BencodeDeserializationError::Custom("custom")
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum BencodeSerializationError {
    #[error("unsupported type: {0}")]
    UnsupportedType(&'static str),
    // TODO: Add static string for error
    #[error("invalid map key, it should be byt string, but got something else")]
    InvalidMapKey,
    #[error("custom: {0}")]
    Custom(&'static str),
}

impl serde::ser::Error for BencodeSerializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        println!("SERIALIZATION ERROR: {}", msg);
        BencodeSerializationError::Custom("custom")
    }
}
