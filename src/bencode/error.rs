use crate::bencode::core::BencodeType;
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum ReceivedBencodeType {
    Known(BencodeType),
    Unknown(char),
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum BencodeError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("cannot parse int {0}")]
    CannotParseInteger(#[from] std::num::ParseIntError),
    #[error("len separator (':') is missing")]
    LenSeparatorMissing,
    #[error("invalid length declaration, non didit character: {0}")]
    InvalidLen(char),
    #[error("Integer contains non digit character: {0}")]
    InvalidInteger(char),
    #[error("Integer contains leading zeroes")]
    InvalidIntegerLeadingZero,
    #[error("invalid bencode data: expected {expected:?}, got {actual:?}")]
    UnexpectedBencodeType {
        expected: Option<BencodeType>,
        actual: ReceivedBencodeType,
    },
    #[error("cannot parse str: {0}")]
    InvalidString(#[from] std::str::Utf8Error),
}

// TODO: Move to serde
impl serde::de::Error for BencodeError {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        serde::de::Error::missing_field("x")
    }
}
