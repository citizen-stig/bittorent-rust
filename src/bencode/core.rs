use crate::bencode::error::{BencodeError, ReceivedBencodeType};
use std::fmt::Formatter;

#[allow(dead_code)]
pub struct BencodeDeserializer<'de> {
    pub(crate) input: &'de [u8],
    pub(crate) pos: usize,
    stack_depth: usize,
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

pub(crate) const INT: u8 = 'i' as u8;
pub(crate) const END: u8 = 'e' as u8;
pub(crate) const LIST: u8 = 'l' as u8;
pub(crate) const DICT: u8 = 'd' as u8;

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
}
