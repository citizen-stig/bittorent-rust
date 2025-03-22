//! Implementation of bencode
//!

mod core;
mod deser;
mod error;

pub use crate::bencode::core::BencodeDeserializer;
use crate::bencode::core::{DICT, END, LIST};
pub use crate::bencode::error::BencodeError;
use serde::de::{DeserializeSeed, Visitor};
use serde::forward_to_deserialize_any;
use std::fmt::{Display, Formatter};
// use serde::Deserialize;

#[derive(Debug)]
struct BencodeSeqAccess<'a, 'de> {
    de: &'a mut BencodeDeserializer<'de>,
}

impl<'de, 'a> serde::de::SeqAccess<'de> for BencodeSeqAccess<'a, 'de> {
    type Error = BencodeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        // Check if we’re at a list end (e.g., see an 'e' byte or run out of bytes).
        if self.de.input.get(self.de.pos) == Some(&END) {
            self.de.pos += 1;
            return Ok(None);
        }

        // Otherwise, parse the next element. We delegate to the main deserializer:
        let value = seed.deserialize(&mut *self.de)?;
        Ok(Some(value))
    }
}

impl<'de, 'a> serde::de::MapAccess<'de> for BencodeSeqAccess<'a, 'de> {
    type Error = BencodeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.input.get(self.de.pos) == Some(&END) {
            self.de.pos += 1;
            return Ok(None);
        }
        let key = seed.deserialize(&mut *self.de)?;
        Ok(Some(key))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

impl<'de> serde::de::Deserializer<'de> for &mut BencodeDeserializer<'de> {
    type Error = BencodeError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 u8 u16 u32 u64 f32 f64 char byte_buf
        option unit unit_struct
        newtype_struct tuple_struct tuple enum
        identifier ignored_any
    }

    fn deserialize_any<V>(
        self,
        v: V,
    ) -> Result<<V as Visitor<'de>>::Value, <Self as serde::Deserializer<'de>>::Error>
    where
        V: Visitor<'de>,
    {
        match self.input.get(self.pos) {
            None => Err(BencodeError::Custom("end of input".to_string())),
            Some(&INT) => self.deserialize_i64(v),
            // Some(&LIST) => self.deserialize_seq(v),
            Some(&LIST) => self.deserialize_seq(v),
            Some(&DICT) => self.deserialize_map(v),
            // THIS OVERFLOWS
            Some(b'0'..=b'9') => self.deserialize_bytes(v),
            // THIS WORKS:
            // Some(b'0'..=b'9') => self.deserialize_str(v),
            Some(_) => Err(BencodeError::Custom("other not supported".to_string())),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_integer()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.parse_str()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let output = self.parse_str()?;
        visitor.visit_string(output.to_string())
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(self.parse_bytes()?)
    }

    // fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: Visitor<'de>,
    // {
    //     let output = self.parse_bytes()?;
    //     visitor.visit_byte_buf(output.to_vec())
    // }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self
            .input
            .len()
            // 1 for "l" and 1 for "e"
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeError::UnexpectedEof);
        }

        if self.input[self.pos].is_ascii_digit() {
            return self.deserialize_bytes(visitor);
        } else if self.input[self.pos] != LIST {
            return Err(BencodeError::Custom("wrong list".to_string()));
        }

        self.pos += 1;
        let seq_access = BencodeSeqAccess { de: &mut self };

        visitor.visit_seq(seq_access)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self
            .input
            .len()
            // 1 for "d" and 1 for "e"
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeError::UnexpectedEof);
        }
        if self.input[self.pos] != DICT {
            return Err(BencodeError::Custom("wrong dict".to_string()));
        }
        self.pos += 1;
        let seq_access = BencodeSeqAccess { de: &mut self };
        visitor.visit_map(seq_access)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use proptest::prelude::*;
    // use std::collections::HashMap;
    // use std::path::Path;

    fn test_happy_case<'a, T>(deserializer: &mut BencodeDeserializer<'a>, expected_value: T)
    where
        T: serde::Deserialize<'a> + PartialEq + std::fmt::Debug,
    {
        let deserialized = T::deserialize(&mut *deserializer).expect("Failed to deserialize");
        assert_eq!(
            deserialized, expected_value,
            "Unexpected value deserialized"
        );
        assert!(
            deserializer.is_consumed(),
            "deserializer should be consumed"
        );
    }

    fn test_error_case<'a, T>(
        deserializer: &mut BencodeDeserializer<'a>,
        _expected_error: BencodeError,
    ) where
        T: serde::Deserialize<'a> + PartialEq + std::fmt::Debug,
    {
        let deserialized = T::deserialize(deserializer);
        // Asserting error later
        // assert_eq!(deserialized, Err(expected_error));
        assert!(deserialized.is_err());
    }

    #[test]
    fn integers_happy_cases() {
        let cases = [
            (&b"i42e"[..], 42),
            (&b"i500e"[..], 500),
            (&b"i0e"[..], 0),
            (&b"i-1e"[..], -1),
            (&b"i-3200e"[..], -3200),
            (&b"i9223372036854775807e"[..], i64::MAX),
            (&b"i-9223372036854775808e"[..], i64::MIN),
        ];

        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_happy_case(&mut deserializer, expected_value);
        }
    }

    #[test]
    fn integers_error_cases() {
        let cases = [
            (
                &b"i9223372036854775808e"[..],
                BencodeError::CannotParseInteger(
                    "92233720368547758080000".parse::<i64>().unwrap_err(),
                ),
            ),
            // TODO: actual errors
            (&b"i-e"[..], BencodeError::UnexpectedEof),
            (&b"i-"[..], BencodeError::UnexpectedEof),
            (&b"i-0"[..], BencodeError::UnexpectedEof),
            (&b"i1"[..], BencodeError::UnexpectedEof),
            (&b"ioe"[..], BencodeError::UnexpectedEof),
            (&b"iq"[..], BencodeError::UnexpectedEof),
            (&b"ie"[..], BencodeError::UnexpectedEof),
            // Missing terminator
            (&b"i10"[..], BencodeError::UnexpectedEof),
        ];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_error_case::<i64>(&mut deserializer, expected_error);
        }
    }

    #[test]
    fn bytes_happy_cases() {
        let cases = [
            (&b"1:a"[..], "a".as_bytes()),
            (&b"4:aaaa"[..], "aaaa".as_bytes()),
            (&b"7:bencode"[..], "bencode".as_bytes()),
            (&b"0:"[..], &[]),
            ("12:привет".as_bytes(), "привет".as_bytes()),
            (&b"3:\xFF\xFE\xFD"[..], &[0xFF, 0xFE, 0xFD][..]),
        ];
        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(&data[..]);
            test_happy_case::<&[u8]>(&mut deserializer, expected_value);
        }
    }

    #[test]
    fn bytes_error_cases() {
        let cases = [
            // TODO: Actual errors
            (&b"2:a"[..], BencodeError::UnexpectedEof),
            (&b"1:"[..], BencodeError::UnexpectedEof),
            (&b"-1:a"[..], BencodeError::UnexpectedEof),
            (&b"2aa"[..], BencodeError::UnexpectedEof),
        ];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(&data[..]);
            test_error_case::<&[u8]>(&mut deserializer, expected_error);
        }
    }

    #[test]
    fn list_of_integers_happy_cases() {
        let cases = [
            (&b"le"[..], vec![]),
            (&b"li42ei12ee"[..], vec![42i64, 12i64]),
        ];

        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(&data[..]);
            test_happy_case(&mut deserializer, expected_value);
        }
    }

    #[test]
    fn list_error_cases() {
        let cases = [(&b"l"[..], BencodeError::UnexpectedEof)];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(&data[..]);
            test_error_case::<Vec<i64>>(&mut deserializer, expected_error);
        }
    }
}
