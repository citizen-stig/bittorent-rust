//! Implementation of bencode

use serde::de::{DeserializeSeed, Visitor};
use serde::{de, forward_to_deserialize_any};
use std::fmt::{Display, Formatter};
// use serde::Deserialize;

#[allow(dead_code)]
struct BencodeDeserializer<'de> {
    input: &'de [u8],
    pos: usize,
}

impl<'de> std::fmt::Debug for BencodeDeserializer<'de> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BencodeDeserializer {{ input: {:?}, pos: {} }}",
            String::from_utf8_lossy(self.input),
            self.pos
        )
    }
}

impl<'de> BencodeDeserializer<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_integer(&mut self) -> Result<i64, BencodeError> {
        if self
            .input
            .len()
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeError::UnexpectedEof);
        }
        if self.input[self.pos] != INT {
            return Err(BencodeError::Custom(format!("wrong int: {:?}", self)));
        }
        let start_pos = self.pos + 1; // first after "i"
                                      // TODO: check "ie" case
        let mut end_pos = self.pos + 2; //

        // Finding correct end position and check bytes
        loop {
            if end_pos >= self.input.len() {
                return Err(BencodeError::UnexpectedEof);
            }
            if self.input[end_pos] == END {
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
        Ok(output)
    }

    fn parse_str(&mut self) -> Result<&str, BencodeError> {
        // "1" without
        if self
            .input
            .len()
            // WTF is this addition?
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

        let end_index = colon_index + 1 + length;
        let string_slice = &self.input[colon_index + 1..end_index];

        let s =
            std::str::from_utf8(string_slice).map_err(|e| BencodeError::Custom(e.to_string()))?;
        self.pos = end_index;
        Ok(s)
    }
}

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
        // Check if we’re at list end (e.g. see an 'e' byte or run out of bytes).
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
        K: DeserializeSeed<'de>
    {
        println!("MAP KEY SEED: {:?}", self.de);
        if self.de.input.get(self.de.pos) == Some(&END) {
            self.de.pos += 1;
            return Ok(None);
        }
        let key = seed.deserialize(&mut *self.de)?;
        Ok(Some(key))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>
    {
        println!("MAP VALUE SEED: {:?}", self.de);
        seed.deserialize(&mut *self.de)
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

const INT: u8 = 'i' as u8;
const END: u8 = 'e' as u8;
const LIST: u8 = 'l' as u8;
const DICT: u8 = 'd' as u8;

impl<'de> de::Deserializer<'de> for &mut BencodeDeserializer<'de> {
    type Error = BencodeError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 u8 u16 u32 u64 f32 f64 char str
        bytes byte_buf option unit unit_struct
        newtype_struct tuple_struct tuple enum
        identifier ignored_any
    }

    fn deserialize_any<V>(
        self,
        _v: V,
    ) -> std::result::Result<
        <V as serde::de::Visitor<'de>>::Value,
        <Self as serde::Deserializer<'de>>::Error,
    >
    where
        V: serde::de::Visitor<'de>,
    {
        println!("DESERIALIZE ANY: {}", self.pos);
        todo!()
        // match self.input.get(self.pos) {
        //     None => Err(BencodeError::Custom("end of input".to_string())),
        //     Some(&INT) => self.deserialize_i64(v),
        //     Some(&LIST) => Err(BencodeError::Custom("list not supported".to_string())),
        //     Some(&DICT) => Err(BencodeError::Custom("dict not supported".to_string())),
        //     Some(b'0'..=b'9') => Err(BencodeError::Custom(
        //         "byte string not supported".to_string(),
        //     )),
        //     Some(_) => Err(BencodeError::Custom("other not supported".to_string())),
        // }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let output = self.parse_integer()?;
        visitor.visit_i64(output)
    }

    // deserialize_str ??

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let output = self.parse_str()?;
        visitor.visit_str(output)
    }

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
        if self.input[self.pos] != LIST {
            return Err(BencodeError::Custom("wrong list".to_string()));
        }

        self.pos += 1;
        let seq_access = BencodeSeqAccess { de: &mut self };

        visitor.visit_seq(seq_access)
    }

    fn deserialize_map<V>(mut self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("STARTED DES MAP: {:?}", self);
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
        _visitor.visit_map(seq_access)
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
    use std::collections::HashMap;
    use super::*;
    use proptest::prelude::*;
    use serde::Deserialize;

    fn from_bencode<'a, T>(mut deserializer: BencodeDeserializer<'a>) -> Result<T, BencodeError>
    where
        T: Deserialize<'a>,
    {
        T::deserialize(&mut deserializer)
    }

    #[test]
    fn integers() {
        let data = b"i42e";
        let mut deserializer = BencodeDeserializer::new(&data[..]);
        let result: i64 = i64::deserialize(&mut deserializer).unwrap();
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
            let mut deserializer = BencodeDeserializer::new(&input[..]);
            let result = i64::deserialize(&mut deserializer);
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
            let mut deserializer = BencodeDeserializer::new(&input[..]);
            let result = String::deserialize(&mut deserializer);
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
    fn list_two_numbers() {
        let data = b"li42ei12ee";
        let deserializer = BencodeDeserializer::new(&data[..]);
        let result: Vec<i64> = from_bencode(deserializer).unwrap();

        assert_eq!(result, vec![42, 12]);
    }

    #[test]
    fn list_of_strings() {
        let data = b"l1:a2:bbe";
        let deserializer = BencodeDeserializer::new(&data[..]);
        let result: Vec<String> = from_bencode(deserializer).unwrap();

        assert_eq!(result, vec!["a", "bb"]);
    }

    #[test]
    fn list_of_list_of_ints() {
        let data = b"lli42ei12eeli1ei2eee";
        let deserializer = BencodeDeserializer::new(&data[..]);
        let result: Vec<Vec<i64>> = from_bencode(deserializer).unwrap();

        let expected = vec![vec![42, 12], vec![1, 2]];
        assert_eq!(result, expected);
    }


    #[test]
    fn dict_to_number() {
        // Keys are byte strings and must appear in lexicographical order.
        let data = b"d7:meaningi42e4:wakai12ee";
        let deserializer = BencodeDeserializer::new(&data[..]);
        let result: HashMap<String, i64>= from_bencode(deserializer).unwrap();


    }
}
