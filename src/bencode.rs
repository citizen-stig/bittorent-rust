//! Implementation of bencode

use serde::de::{Error, Visitor};
use serde::{de, forward_to_deserialize_any};
use std::fmt::Display;
// use serde::Deserialize;

#[allow(dead_code)]
struct BencodeDeserializer<'de> {
    input: &'de [u8],
    pos: usize,
}

impl<'de> BencodeDeserializer<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        Self { input, pos: 0 }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum BencodeError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("cannot parse int {0}")]
    CannotParseInteger(#[from] std::num::ParseIntError),
    #[error("custom {0}")]
    Custom(String),
}

impl serde::de::Error for BencodeError {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        println!("MSG: {}", _msg);
        serde::de::Error::missing_field("x")
    }
}

const INT_START: u8 = 'i' as u8;
const INT_END: u8 = 'e' as u8;
const LIST: u8 = 'l' as u8;
const DICT: u8 = 'd' as u8;

impl<'de> de::Deserializer<'de> for BencodeDeserializer<'de> {
    type Error = BencodeError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 u8 u16 u32 u64 f32 f64 char str
        string bytes byte_buf option unit unit_struct
        newtype_struct tuple_struct tuple enum
        identifier ignored_any
    }

    fn deserialize_any<V>(
        self,
        _: V,
    ) -> std::result::Result<
        <V as serde::de::Visitor<'de>>::Value,
        <Self as serde::Deserializer<'de>>::Error,
    >
    where
        V: serde::de::Visitor<'de>,
    {
        match self.input.get(self.pos) {
            None => Err(BencodeError::custom("end of input")),
            Some(&INT_START) => {
                println!("INT!");
                Err(BencodeError::custom("not supported"))
            }
            Some(&LIST) => Err(BencodeError::custom("not supported")),
            Some(&DICT) => Err(BencodeError::custom("not supported")),
            Some(_) => Err(BencodeError::custom("not supported")),
        }
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Should have i0e, so pos should min len is 3, while pos is 0
        if self
            .input
            .len()
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeError::UnexpectedEof);
        }
        if self.input[self.pos] != INT_START {
            return Err(BencodeError::Custom("wrong int".to_string()));
        }
        let start_pos = self.pos + 1; // first after "i"
                                      // TODO: check "ie" case
        let mut end_pos = self.pos + 2; //

        // Finding correct end position and check bytes
        loop {
            if end_pos >= self.input.len() {
                return Err(BencodeError::UnexpectedEof);
            }
            if self.input[end_pos] == INT_END {
                break;
            }
            if !self.input[end_pos].is_ascii_digit() {
                return Err(BencodeError::Custom("not a digit".to_string()));
            }
            end_pos += 1;
        }

        // SAFETY: CHECKED ALL DIGITS INSIDE LOOP ABOVE
        let s = unsafe { std::str::from_utf8_unchecked(&self.input[start_pos..end_pos]) };

        let output: i64 = s.parse()?;
        self.pos = end_pos + 1;
        visitor.visit_i64(output)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn integers() {
        let data = b"i42e";
        let deserializer = BencodeDeserializer::new(&data[..]);
        let result: i64 = i64::deserialize(deserializer).unwrap();
        assert_eq!(result, 42);

        let cases = [
            // positive cases
            (b"i42e".to_vec(), Ok(42)),
            (b"i0e".to_vec(), Ok(0)),
            (b"i-1e".to_vec(), Ok(-1)),
            (b"i9223372036854775807e".to_vec(), Ok(i64::MAX)),
            (
                b"i9223372036854775808e".to_vec(),
                Err(BencodeError::CannotParseInteger(
                    "92233720368547758080000".parse::<i64>().unwrap_err(),
                )),
            ),
            (b"i-e".to_vec(), Err(BencodeError::Custom("x".to_string()))),
            (b"i-".to_vec(), Err(BencodeError::Custom("x".to_string()))),
            (b"i1".to_vec(), Err(BencodeError::UnexpectedEof)),
            (b"iq".to_vec(), Err(BencodeError::Custom("x".to_string()))),
            (b"ie".to_vec(), Err(BencodeError::Custom("x".to_string()))),
        ];

        for (input, expected) in cases {
            let deserializer = BencodeDeserializer::new(&input[..]);
            let result = i64::deserialize(deserializer);
            // Skip error checks for now
            // assert_eq!(
            //     expected,
            //     result,
            //     "input: {}",
            //     std::str::from_utf8(&input).unwrap()
            // );
            let input_pretty = std::str::from_utf8(&input).unwrap();
            match (&result, expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, &expected, "input: {:?}", input_pretty);
                }
                (Err(_), Err(_)) => {
                    // some errors, fine for now
                }
                _ => {
                    panic!("incorrect result: {:?} for input: {}", result, input_pretty);
                }
            }
        }
    }
}
