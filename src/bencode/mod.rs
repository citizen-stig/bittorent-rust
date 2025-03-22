//! Implementation of bencode
//!

mod core;
mod deser;
mod error;

pub use crate::bencode::core::BencodeDeserializer;
pub use crate::bencode::error::BencodeError;

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
            let mut deserializer = BencodeDeserializer::new(data);
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
            let mut deserializer = BencodeDeserializer::new(data);
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
            let mut deserializer = BencodeDeserializer::new(data);
            test_happy_case(&mut deserializer, expected_value);
        }
    }

    #[test]
    fn list_error_cases() {
        let cases = [(&b"l"[..], BencodeError::UnexpectedEof)];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_error_case::<Vec<i64>>(&mut deserializer, expected_error);
        }
    }
}
