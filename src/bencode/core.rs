use crate::bencode::error::{BencodeDeserializationError, ReceivedBencodeType};
use std::collections::BTreeMap;
use std::fmt::Formatter;

#[allow(dead_code)]
pub struct BencodeDeserializer<'de> {
    pub(crate) input: &'de [u8],
    pub(crate) pos: usize,
}

impl std::fmt::Debug for BencodeDeserializer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BencodeDeserializer {{ input: {:?}, pos: {} }}",
            String::from_utf8_lossy(self.input),
            self.pos
        )
    }
}

pub(crate) const INT: u8 = b'i';
pub(crate) const END: u8 = b'e';
pub(crate) const LIST: u8 = b'l';
pub(crate) const DICT: u8 = b'd';

#[derive(Debug, Clone, PartialEq)]
pub enum BencodeType {
    Integer,
    ByteString,
    List,
    Dict,
}

impl BencodeType {
    pub fn from_byte_to_received(b: u8) -> ReceivedBencodeType {
        match Self::from_byte(b) {
            Some(bencode_type) => ReceivedBencodeType::Known(bencode_type),
            None => ReceivedBencodeType::Unknown(char::from(b)),
        }
    }

    fn from_byte(b: u8) -> Option<Self> {
        match b {
            INT => Some(BencodeType::Integer),
            LIST => Some(BencodeType::List),
            DICT => Some(BencodeType::Dict),
            b'0'..=b'9' => Some(BencodeType::ByteString),
            _ => None,
        }
    }
}

impl<'de> BencodeDeserializer<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        Self { input, pos: 0 }
    }

    pub(crate) fn check_for_container_type(&self) -> Result<(), BencodeDeserializationError> {
        if self
            .input
            .len()
            // 1 for "l/d" and 1 for "e"
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeDeserializationError::UnexpectedEof);
        }
        Ok(())
    }

    pub(crate) fn check_type(
        &self,
        expected: BencodeType,
    ) -> Result<(), BencodeDeserializationError> {
        let recovered_type = BencodeType::from_byte(self.input[self.pos]);

        if recovered_type.as_ref() != Some(&expected) {
            return Err(BencodeDeserializationError::UnexpectedBencodeType {
                expected: Some(expected),
                actual: BencodeType::from_byte_to_received(self.input[self.pos]),
            });
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn is_consumed(&self) -> bool {
        self.pos == self.input.len()
    }

    pub(crate) fn parse_integer(&mut self) -> Result<i64, BencodeDeserializationError> {
        if self
            .input
            .len()
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeDeserializationError::UnexpectedEof);
        }
        self.check_type(BencodeType::Integer)?;
        let start_pos = self.pos + 1; // first after "i"
        let mut end_pos = self.pos + 2; //

        // Finding the correct end position and check bytes
        loop {
            if end_pos >= self.input.len() {
                return Err(BencodeDeserializationError::UnexpectedEof);
            }
            if self.input[end_pos] == END {
                break;
            }
            if !self.input[end_pos].is_ascii_digit() {
                return Err(BencodeDeserializationError::InvalidInteger(
                    self.input[end_pos].into(),
                ));
            }
            end_pos += 1;
        }

        // SAFETY: checked all digits inside the loop above
        let s = unsafe { std::str::from_utf8_unchecked(&self.input[start_pos..end_pos]) };

        if s.len() > 1 && s.starts_with('0') {
            return Err(BencodeDeserializationError::InvalidIntegerLeadingZero);
        }

        let output: i64 = s.parse()?;
        self.pos = end_pos + 1;
        Ok(output)
    }

    pub(crate) fn parse_bytes(&mut self) -> Result<&'de [u8], BencodeDeserializationError> {
        if self.input.len().checked_sub(self.pos).is_none() {
            return Err(BencodeDeserializationError::UnexpectedEof);
        }
        let colon_index = match self.input[self.pos..].iter().position(|&x| x == b':') {
            Some(index) => self.pos + index,
            None => return Err(BencodeDeserializationError::LenSeparatorMissing),
        };
        let len_slice = &self.input[self.pos..colon_index];

        for digit in len_slice {
            if !digit.is_ascii_digit() {
                return Err(BencodeDeserializationError::InvalidLen(char::from(*digit)));
            }
        }

        let len_s = unsafe { std::str::from_utf8_unchecked(len_slice) };

        let length: usize = len_s.parse()?;

        let end_index = colon_index + 1 + length;
        if end_index > self.input.len() {
            return Err(BencodeDeserializationError::UnexpectedEof);
        }
        self.pos = end_index;
        Ok(&self.input[colon_index + 1..end_index])
    }

    #[allow(dead_code)]
    pub(crate) fn parse_str(&mut self) -> Result<&'de str, BencodeDeserializationError> {
        let string_slice = self.parse_bytes()?;

        let s = std::str::from_utf8(string_slice).map_err(|e| {
            println!("OOOPS: {}", String::from_utf8_lossy(string_slice));
            BencodeDeserializationError::InvalidString(e)
        })?;
        // let s = match std::str::from_utf8(string_slice) {
        //     Ok(res) => res,
        //     Err(_e) => "MISSING",
        // };
        Ok(s)
    }

    fn get_integer(&mut self) -> Result<Bencode<'de>, BencodeDeserializationError> {
        Ok(Bencode::Integer(self.parse_integer()?))
    }

    fn get_bytes(&mut self) -> Result<Bencode<'de>, BencodeDeserializationError> {
        Ok(Bencode::Bytes(self.parse_bytes()?))
    }

    fn get_any(&mut self) -> Result<Bencode<'de>, BencodeDeserializationError> {
        match self.input.get(self.pos) {
            None => Err(BencodeDeserializationError::UnexpectedEof),
            Some(&INT) => self.get_integer(),
            Some(&LIST) => self.get_list(),
            Some(&DICT) => self.get_dict(),
            Some(b'0'..=b'9') => self.get_bytes(),
            Some(b) => Err(BencodeDeserializationError::UnexpectedBencodeType {
                expected: None,
                actual: BencodeType::from_byte_to_received(*b),
            }),
        }
    }

    #[allow(dead_code)]
    fn get_list(&mut self) -> Result<Bencode<'de>, BencodeDeserializationError> {
        let mut items = Vec::new();
        if self
            .input
            .len()
            // 1 for "l" and 1 for "e"
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeDeserializationError::UnexpectedEof);
        }

        self.check_type(BencodeType::List)?;
        self.pos = self.pos.checked_add(1).expect("Position overflow");

        while self.input.get(self.pos) != Some(&END) {
            let item = self.get_any()?;
            items.push(item);
        }
        self.pos = self.pos.checked_add(1).expect("Position overflow");
        Ok(Bencode::List(items))
    }

    #[allow(dead_code)]
    fn get_dict(&mut self) -> Result<Bencode<'de>, BencodeDeserializationError> {
        let mut map = BTreeMap::new();
        if self
            .input
            .len()
            // 1 for "d" and 1 for "e"
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeDeserializationError::UnexpectedEof);
        }

        self.check_type(BencodeType::Dict)?;

        self.pos = self.pos.checked_add(1).expect("Position overflow");

        while self.input.get(self.pos) != Some(&END) {
            let key = self.parse_bytes()?;
            let value = self.get_any()?;
            map.insert(key, value);
        }

        Ok(Bencode::Dict(map))
    }
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum Bencode<'a> {
    Integer(i64),
    Bytes(&'a [u8]),
    // TODO: Zero copy those.
    List(Vec<Bencode<'a>>),
    Dict(BTreeMap<&'a [u8], Bencode<'a>>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer_happy_cases() {
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
            let value = deserializer.parse_integer().unwrap();
            assert_eq!(value, expected_value);
        }
    }

    #[test]
    fn parse_integer_error_cases() {
        let invalid_digit_err = "-".parse::<i64>().unwrap_err();
        let cases = [
            (
                &b"i9223372036854775808e"[..],
                BencodeDeserializationError::CannotParseInteger(
                    "92233720368547758080000".parse::<i64>().unwrap_err(),
                ),
            ),
            (
                &b"i-e"[..],
                BencodeDeserializationError::CannotParseInteger(invalid_digit_err.clone()),
            ),
            (
                &b"i000500e"[..],
                BencodeDeserializationError::InvalidIntegerLeadingZero,
            ),
            (&b"i-"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"i-0"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"i1"[..], BencodeDeserializationError::UnexpectedEof),
            (
                &b"ioe"[..],
                BencodeDeserializationError::CannotParseInteger(invalid_digit_err.clone()),
            ),
            (&b"iq"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"ie"[..], BencodeDeserializationError::UnexpectedEof),
            // Missing terminator
            (&b"i10"[..], BencodeDeserializationError::UnexpectedEof),
        ];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            let err = deserializer.parse_integer().unwrap_err();
            assert_eq!(
                err,
                expected_error,
                "Unexpected for input: {}",
                String::from_utf8_lossy(data)
            );
        }
    }

    #[test]
    fn parse_bytes_happy_cases() {
        let cases = [
            (&b"1:a"[..], "a".as_bytes()),
            (&b"4:aaaa"[..], "aaaa".as_bytes()),
            (&b"7:bencode"[..], "bencode".as_bytes()),
            (&b"0:"[..], &[]),
            ("12:привет".as_bytes(), "привет".as_bytes()),
            (&b"3:\xFF\xFE\xFD"[..], &[0xFF, 0xFE, 0xFD][..]),
        ];
        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            let value = deserializer.parse_bytes().unwrap();
            assert_eq!(value, expected_value);
        }
    }

    #[test]
    fn parse_bytes_error_cases() {
        let cases = [
            (&b"2:a"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"1:"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"-1:a"[..], BencodeDeserializationError::InvalidLen('-')),
            (
                &b"2aa"[..],
                BencodeDeserializationError::LenSeparatorMissing,
            ),
        ];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            let err = deserializer.parse_bytes().unwrap_err();
            assert_eq!(
                err,
                expected_error,
                "Unexpected for input: {}",
                String::from_utf8_lossy(data)
            );
        }
    }

    #[test]
    fn parse_list() {
        let cases = [
            (&b"le"[..], Bencode::List(vec![])),
            (
                &b"li42ei12ee"[..],
                Bencode::List(vec![Bencode::Integer(42), Bencode::Integer(12)]),
            ),
            (
                &b"li42e1:ae"[..],
                Bencode::List(vec![Bencode::Integer(42), Bencode::Bytes(&b"a"[..])]),
            ),
            // Nested lists
            (
                &b"ll3:fooee"[..],
                Bencode::List(vec![Bencode::List(vec![Bencode::Bytes(&b"foo"[..])])]),
            ),
            (
                &b"lli42eeli12eee"[..],
                Bencode::List(vec![
                    Bencode::List(vec![Bencode::Integer(42)]),
                    Bencode::List(vec![Bencode::Integer(12)]),
                ]),
            ),
            // List with multiple data types
            (
                &b"li42e3:bar4:spami-10ee"[..],
                Bencode::List(vec![
                    Bencode::Integer(42),
                    Bencode::Bytes(&b"bar"[..]),
                    Bencode::Bytes(&b"spam"[..]),
                    Bencode::Integer(-10),
                ]),
            ),
            // List with empty byte string
            (&b"l0:e"[..], Bencode::List(vec![Bencode::Bytes(&[])])),
            // List with deep nesting
            (
                &b"llleee"[..],
                Bencode::List(vec![Bencode::List(vec![Bencode::List(vec![])])]),
            ),
            // List with empty list elements
            (
                &b"llelei42ee"[..],
                Bencode::List(vec![
                    Bencode::List(Vec::new()),
                    Bencode::List(Vec::new()),
                    Bencode::Integer(42),
                ]),
            ),
            // List with dictionary
            (
                &b"lld3:foo3:baree"[..],
                Bencode::List(vec![Bencode::List(vec![Bencode::Dict({
                    let mut map = BTreeMap::new();
                    map.insert(&b"foo"[..], Bencode::Bytes(&b"bar"[..]));
                    map
                })])]),
            ),
        ];

        for (input, expected) in cases {
            let mut deserializer = BencodeDeserializer::new(input);
            let actual = deserializer.get_list().unwrap_or_else(|_| {
                panic!(
                    "Unexpected error for input: {}, output: {:?}",
                    String::from_utf8_lossy(input),
                    expected,
                )
            });
            assert_eq!(actual, expected);
        }
    }

    // TODO: List error cases

    #[test]
    fn parse_dict() {
        let cases = [
            (&b"de"[..], Bencode::Dict(Default::default())),
            (
                &b"d1:ai42ee"[..],
                Bencode::Dict([(&[b'a'][..], Bencode::Integer(42))].into()),
            ),
            // TODO: More happy cases
        ];
        for (input, expected) in cases {
            let mut deserializer = BencodeDeserializer::new(input);
            let actual = deserializer.get_dict().unwrap();
            assert_eq!(actual, expected);
        }
    }

    // TODO: dict error cases
}
