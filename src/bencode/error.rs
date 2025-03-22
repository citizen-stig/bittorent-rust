use std::fmt::Display;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum BencodeError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("cannot parse int {0}")]
    CannotParseInteger(#[from] std::num::ParseIntError),
    #[error("custom {0}")]
    // TODO: Cow?
    Custom(String),
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
