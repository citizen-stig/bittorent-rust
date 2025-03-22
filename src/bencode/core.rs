use crate::bencode::error::{BencodeError, ReceivedBencodeType};
use std::fmt::Formatter;

#[allow(dead_code)]
pub struct BencodeDeserializer<'de> {
    pub(crate) input: &'de [u8],
    pub(crate) pos: usize,
    stack_depth: usize,
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
    pub fn from_byte(b: u8) -> ReceivedBencodeType {
        match b {
            INT => ReceivedBencodeType::Known(BencodeType::Integer),
            LIST => ReceivedBencodeType::Known(BencodeType::List),
            DICT => ReceivedBencodeType::Known(BencodeType::Dict),
            b'0'..=b'9' => ReceivedBencodeType::Known(BencodeType::ByteString),
            _ => ReceivedBencodeType::Unknown(char::from(b)),
        }
    }
}

impl<'de> BencodeDeserializer<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        Self {
            input,
            pos: 0,
            stack_depth: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn is_consumed(&self) -> bool {
        self.pos == self.input.len()
    }

    pub(crate) fn parse_integer(&mut self) -> Result<i64, BencodeError> {
        if self
            .input
            .len()
            .checked_sub(self.pos.saturating_add(2))
            .is_none()
        {
            return Err(BencodeError::UnexpectedEof);
        }
        if self.input[self.pos] != INT {
            return Err(BencodeError::UnexpectedBencodeType {
                expected: Some(BencodeType::Integer),
                actual: BencodeType::from_byte(self.input[self.pos]),
            });
        }
        let start_pos = self.pos + 1; // first after "i"
        let mut end_pos = self.pos + 2; //

        // Finding the correct end position and check bytes
        loop {
            if end_pos >= self.input.len() {
                return Err(BencodeError::UnexpectedEof);
            }
            if self.input[end_pos] == END {
                break;
            }
            if !self.input[end_pos].is_ascii_digit() {
                return Err(BencodeError::InvalidInteger(self.input[end_pos].into()));
            }
            end_pos += 1;
        }

        // SAFETY: checked all digits inside the loop above
        let s = unsafe { std::str::from_utf8_unchecked(&self.input[start_pos..end_pos]) };

        if s.len() > 1 && s.starts_with('0') {
            return Err(BencodeError::InvalidIntegerLeadingZero);
        }

        let output: i64 = s.parse()?;
        self.pos = end_pos + 1;
        Ok(output)
    }

    pub(crate) fn parse_bytes(&mut self) -> Result<&'de [u8], BencodeError> {
        if self
            .input
            .len()
            // WTF is this addition?
            .checked_sub(self.pos)
            .is_none()
        {
            return Err(BencodeError::UnexpectedEof);
        }
        let colon_index = match self.input[self.pos..].iter().position(|&x| x == b':') {
            Some(index) => self.pos + index,
            None => return Err(BencodeError::LenSeparatorMissing),
        };
        let len_slice = &self.input[self.pos..colon_index];

        for digit in len_slice {
            if !digit.is_ascii_digit() {
                return Err(BencodeError::InvalidLen(char::from(*digit)));
            }
        }

        let len_s = unsafe { std::str::from_utf8_unchecked(len_slice) };

        let length: usize = len_s.parse()?;

        let end_index = colon_index + 1 + length;
        if end_index > self.input.len() {
            return Err(BencodeError::UnexpectedEof);
        }
        self.pos = end_index;
        Ok(&self.input[colon_index + 1..end_index])
    }

    pub(crate) fn parse_str(&mut self) -> Result<&'de str, BencodeError> {
        let string_slice = self.parse_bytes()?;

        let s = std::str::from_utf8(string_slice).map_err(|e| {
            println!("OOOPS: {}", String::from_utf8_lossy(string_slice));
            BencodeError::InvalidString(e)
        })?;
        // let s = match std::str::from_utf8(string_slice) {
        //     Ok(res) => res,
        //     Err(_e) => "MISSING",
        // };
        Ok(s)
    }

    // Advancing iterator without actual parsing
    pub(crate) fn skip_value(&mut self) -> Result<(), BencodeError> {
        match self.input.get(self.pos) {
            None => Err(BencodeError::UnexpectedEof),
            Some(&INT) => {
                let _ = self.parse_integer()?;
                Ok(())
            }
            Some(&LIST) => {
                self.pos += 1; // Skip 'l'
                while self.pos < self.input.len() && self.input[self.pos] != END {
                    self.skip_value()?;
                }
                if self.pos < self.input.len() {
                    self.pos += 1; // Skip 'e'
                    Ok(())
                } else {
                    Err(BencodeError::UnexpectedEof)
                }
            }
            Some(&DICT) => {
                self.pos += 1; // Skip 'd'
                while self.pos < self.input.len() && self.input[self.pos] != END {
                    // Skip key
                    self.skip_value()?;
                    // Skip value
                    if self.pos < self.input.len() {
                        self.skip_value()?;
                    } else {
                        return Err(BencodeError::UnexpectedEof);
                    }
                }
                if self.pos < self.input.len() {
                    self.pos += 1; // Skip 'e'
                    Ok(())
                } else {
                    Err(BencodeError::UnexpectedEof)
                }
            }
            Some(b'0'..=b'9') => {
                let _ = self.parse_bytes()?;
                Ok(())
            }
            Some(b) => Err(BencodeError::UnexpectedBencodeType {
                expected: None,
                actual: BencodeType::from_byte(*b),
            }),
        }
    }
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
                BencodeError::CannotParseInteger(
                    "92233720368547758080000".parse::<i64>().unwrap_err(),
                ),
            ),
            (
                &b"i-e"[..],
                BencodeError::CannotParseInteger(invalid_digit_err.clone()),
            ),
            (&b"i000500e"[..], BencodeError::InvalidIntegerLeadingZero),
            (&b"i-"[..], BencodeError::UnexpectedEof),
            (&b"i-0"[..], BencodeError::UnexpectedEof),
            (&b"i1"[..], BencodeError::UnexpectedEof),
            (
                &b"ioe"[..],
                BencodeError::CannotParseInteger(invalid_digit_err.clone()),
            ),
            (&b"iq"[..], BencodeError::UnexpectedEof),
            (&b"ie"[..], BencodeError::UnexpectedEof),
            // Missing terminator
            (&b"i10"[..], BencodeError::UnexpectedEof),
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
            (&b"2:a"[..], BencodeError::UnexpectedEof),
            (&b"1:"[..], BencodeError::UnexpectedEof),
            (&b"-1:a"[..], BencodeError::InvalidLen('-')),
            (&b"2aa"[..], BencodeError::LenSeparatorMissing),
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

    // This is where things gets interesting.
}
