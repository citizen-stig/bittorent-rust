//! Implementation of bencode
//!

mod core;
// mod deser;
mod error;
mod deser;

pub use crate::bencode::core::BencodeDeserializer;
pub use crate::bencode::error::BencodeDeserializationError;

#[cfg(test)]
mod tests {
    use super::*;
    // use proptest::prelude::*;
    // use std::collections::HashMap;
    // use std::path::Path;

    fn test_happy_case<'a, T>(deserializer: &mut BencodeDeserializer<'a>, expected_value: T)
    where
        T: ::serde::Deserialize<'a> + PartialEq + std::fmt::Debug,
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
        _expected_error: BencodeDeserializationError,
    ) where
        T: ::serde::Deserialize<'a> + PartialEq + std::fmt::Debug,
    {
        let deserialized = T::deserialize(deserializer);
        // Asserting error later
        // assert_eq!(deserialized, Err(expected_error));
        assert!(deserialized.is_err());
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
        let cases = [(&b"l"[..], BencodeDeserializationError::UnexpectedEof)];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_error_case::<Vec<i64>>(&mut deserializer, expected_error);
        }
    }
}
