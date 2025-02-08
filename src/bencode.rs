//! Implementation of bencode

use serde::de::Visitor;
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
    // TODO: Cow?
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
        bytes byte_buf option unit unit_struct
        newtype_struct tuple_struct tuple enum
        identifier ignored_any
    }

    fn deserialize_any<V>(
        self,
        v: V,
    ) -> std::result::Result<
        <V as serde::de::Visitor<'de>>::Value,
        <Self as serde::Deserializer<'de>>::Error,
    >
    where
        V: serde::de::Visitor<'de>,
    {
        println!("DESERIALIZE ANY: {}", self.pos);
        match self.input.get(self.pos) {
            None => Err(BencodeError::Custom("end of input".to_string())),
            Some(&INT_START) => self.deserialize_i64(v),
            Some(&LIST) => Err(BencodeError::Custom("list not supported".to_string())),
            Some(&DICT) => Err(BencodeError::Custom("dict not supported".to_string())),
            Some(b'0'..=b'9') => Err(BencodeError::Custom(
                "byte string not supported".to_string(),
            )),
            Some(_) => Err(BencodeError::Custom("other not supported".to_string())),
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

    // deserialize_str ??

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // "1" without
        if self
            .input
            .len()
            .checked_sub(self.pos.saturating_add(0))
            .is_none()
        {
            return Err(BencodeError::UnexpectedEof);
        }

        let colon_index = match self.input[self.pos..].iter().position(|&x| x == b':') {
            Some(index) => self.pos + index,
            None => return Err(BencodeError::Custom("':' not found".to_string())),
        };

        let len_slice = &self.input[self.pos..colon_index];

        for digit in len_slice {
            if !digit.is_ascii_digit() {
                return Err(BencodeError::Custom(
                    "Invalid digit specification".to_string(),
                ));
            }
        }

        let len_s = unsafe { std::str::from_utf8_unchecked(len_slice) };

        let length: usize = len_s.parse()?;
        if colon_index + 1 + length > self.input.len() {
            return Err(BencodeError::UnexpectedEof);
        }

        let string_slice = &self.input[colon_index + 1..colon_index + 1 + length];
        // Nasty clone
        let output =
            std::str::from_utf8(string_slice).map_err(|e| BencodeError::Custom(e.to_string()))?;
        visitor.visit_str(output)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Next: start here
        todo!("des seq")
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!("des map")
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
        todo!("des struct")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde::Deserialize;

    fn from_bencode<'a, T>(deserializer: BencodeDeserializer<'a>) -> Result<T, BencodeError>
    where
        T: Deserialize<'a>,
    {
        T::deserialize(deserializer)
    }

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

    #[test]
    fn byte_strings() {
        let cases = vec![
            (b"1:a".to_vec(), Ok("a".to_string())),
            (b"7:bencode".to_vec(), Ok("bencode".to_string())),
            (b"10:abcdefghij".to_vec(), Ok("abcdefghij".to_string())),
            ("12:привет".as_bytes().to_vec(), Ok("привет".to_string())),
            (
                b"100000000:a".to_vec(),
                Err(BencodeError::Custom("x".to_string())),
            ),
        ];

        for (input, expected) in cases {
            let deserializer = BencodeDeserializer::new(&input[..]);
            let result = String::deserialize(deserializer);
            let input_pretty = String::from_utf8_lossy(&input);
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

    fn string_round_trip_test(input: &str) {
        let encoded = format!("{}:{}", input.len(), input);
        let deserializer = BencodeDeserializer::new(encoded.as_bytes());
        let decoded: String = from_bencode(deserializer).expect("Failed to decode string");
        assert_eq!(input, decoded);
    }

    prop_compose! {
        fn malformed_string_input()(
            prefix in prop::collection::vec(any::<u8>(), 0..10),
            len in 0..1000usize,
            content in prop::collection::vec(any::<u8>(), 0..1000),
            suffix in prop::collection::vec(any::<u8>(), 0..10)
        ) -> Vec<u8> {
            let mut bytes = prefix;
            bytes.extend(len.to_string().bytes());
            bytes.push(b':');
            bytes.extend(content);
            bytes.extend(suffix);
            bytes
        }
    }

    proptest! {
        #[test]
        fn string_roundtrip_prop(s in ".*") {
            string_round_trip_test(&s)
        }

        #[test]
        fn string_deserialize_does_not_panic(bytes in prop::collection::vec(any::<u8>(), 0..1000)) {
            let deserializer = BencodeDeserializer::new(&bytes);
            let result: Result<String, _> = from_bencode(deserializer);

            // We don't care about the result, we just want to make sure it doesn't panic
            let _ = result;
        }

        #[test]
        fn malformed_string_deserialize_does_not_panic(
            bytes in malformed_string_input()
        ) {
            let deserializer = BencodeDeserializer::new(&bytes);
            let result: Result<String, _> = from_bencode(deserializer);

            // We don't care if it succeeds or fails, just that it doesn't panic
            let _ = result;
        }

    }

    #[test]
    #[ignore = "not implemented"]
    fn list() {
        let data = b"l1e";
        let deserializer = BencodeDeserializer::new(&data[..]);
        let result: Vec<i64> = from_bencode(deserializer).unwrap();

        assert_eq!(result, vec![1]);
    }
}
