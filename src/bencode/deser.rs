use crate::bencode::core::{BencodeType, DICT, END, INT, LIST};
use crate::bencode::{BencodeDeserializer, BencodeError};
use serde::de::{DeserializeSeed, Visitor};
use serde::forward_to_deserialize_any;

impl<'de> serde::de::Deserializer<'de> for &mut BencodeDeserializer<'de> {
    type Error = BencodeError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 u8 u16 u32 u64 f32 f64 char str string
        unit unit_struct newtype_struct tuple
        tuple_struct identifier enum ignored_any option
        byte_buf
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input.get(self.pos) {
            None => Err(BencodeError::UnexpectedEof),
            Some(&INT) => self.deserialize_i64(visitor),
            Some(&LIST) => self.deserialize_seq(visitor),
            Some(&DICT) => self.deserialize_map(visitor),
            Some(b'0'..=b'9') => self.deserialize_bytes(visitor),
            Some(b) => Err(BencodeError::UnexpectedBencodeType {
                expected: None,
                actual: BencodeType::from_byte_to_received(*b),
            }),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_integer()?;
        visitor.visit_i64(value)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(self.parse_bytes()?)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input.get(self.pos) {
            None => {
                return Err(BencodeError::UnexpectedEof);
            }
            Some(b'0'..=b'9') => {
                let elements = self.parse_bytes()?.to_vec();
                let s = serde::de::value::SeqDeserializer::new(elements.into_iter());
                return visitor.visit_seq(s);
            }
            Some(_) => (),
        }
        self.check_for_container_type()?;
        let seq_access = BencodeSeqAccess::new_list(self)?;
        visitor.visit_seq(seq_access)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_for_container_type()?;
        let map_access = BencodeSeqAccess::new_dict(self)?;
        visitor.visit_map(map_access)
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

struct BencodeSeqAccess<'de, 'a> {
    de: &'a mut BencodeDeserializer<'de>,
}

impl<'de, 'a> BencodeSeqAccess<'de, 'a> {
    pub(crate) fn new_list(de: &'a mut BencodeDeserializer<'de>) -> Result<Self, BencodeError> {
        de.check_type(BencodeType::List)?;
        de.pos = de.pos.checked_add(1).expect("Position overflow");
        Ok(Self { de })
    }

    pub(crate) fn new_dict(de: &'a mut BencodeDeserializer<'de>) -> Result<Self, BencodeError> {
        de.check_type(BencodeType::Dict)?;
        de.pos = de.pos.checked_add(1).expect("Position overflow");
        Ok(Self { de })
    }
}

impl<'de> serde::de::SeqAccess<'de> for BencodeSeqAccess<'de, '_> {
    type Error = BencodeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.input.get(self.de.pos) == Some(&END) {
            self.de.pos += 1;
            return Ok(None);
        }

        let value = seed.deserialize(&mut *self.de)?;
        Ok(Some(value))
    }
}

impl<'de> serde::de::MapAccess<'de> for BencodeSeqAccess<'de, '_> {
    type Error = BencodeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.input.get(self.de.pos) == Some(&END) {
            self.de.pos += 1;
            return Ok(None);
        }
        match self.de.input.get(self.de.pos) {
            None => {
                return Err(BencodeError::UnexpectedEof);
            }
            Some(b'0'..=b'9') => {}
            Some(b) => {
                return Err(BencodeError::InvalidKey {
                    actual: BencodeType::from_byte_to_received(*b),
                })
            }
        }

        // Key is effectively a byte string by that point.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::core::BencodeType::Integer;
    use crate::bencode::error::ReceivedBencodeType;
    use serde::Deserialize;

    // Reusing your existing test helpers
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
        expected_error: BencodeError,
    ) where
        T: serde::Deserialize<'a> + PartialEq + std::fmt::Debug,
    {
        let deserialized = T::deserialize(deserializer);
        assert!(deserialized.is_err(), "Expected error but got success");

        assert_eq!(
            deserialized.unwrap_err().to_string(),
            expected_error.to_string()
        );
    }

    // Integer Tests
    #[test]
    fn integer_happy_cases() {
        let cases = [
            (&b"i0e"[..], 0),
            (&b"i42e"[..], 42),
            (&b"i-42e"[..], -42),
            (&b"i9223372036854775807e"[..], i64::MAX),
            (&b"i-9223372036854775808e"[..], i64::MIN),
        ];

        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_happy_case(&mut deserializer, expected_value);
        }
    }

    #[test]
    fn integer_error_cases() {
        let cases = [
            (&b"i"[..], BencodeError::UnexpectedEof),
            (&b"ie"[..], BencodeError::UnexpectedEof),
            (&b"i42"[..], BencodeError::UnexpectedEof),
            (&b"i42x"[..], BencodeError::InvalidInteger('x')),
            (
                &b"i9223372036854775808e"[..],
                BencodeError::CannotParseInteger(
                    "92233720368547758080000".parse::<i64>().unwrap_err(),
                ),
            ),
        ];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_error_case::<i64>(&mut deserializer, expected_error);
        }
    }

    // List Tests
    #[test]
    fn list_happy_cases() {
        let cases = [
            (&b"le"[..], Vec::<i64>::new()),
            (&b"li42ee"[..], vec![42i64]),
            (&b"li42ei-13ei0ee"[..], vec![42i64, -13i64, 0i64]),
        ];

        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_happy_case(&mut deserializer, expected_value);
        }
    }

    #[test]
    fn list_happy_cases_nested() {
        let cases = [
            // (&b"le"[..], Vec::<i64>::new()),
            // (&b"li42ee"[..], vec![42i64]),
            // (&b"li42ei-13ei0ee"[..], vec![42i64, -13i64, 0i64]),
            (&b"llee"[..], vec![Vec::<i64>::new()]),
            (&b"lli42eeli-13eee"[..], vec![vec![42i64], vec![-13i64]]),
        ];

        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_happy_case(&mut deserializer, expected_value);
        }
    }

    #[test]
    fn list_error_cases() {
        let cases = [
            (&b"l"[..], BencodeError::UnexpectedEof),
            (&b"li42e"[..], BencodeError::UnexpectedEof),
            (&b"li42"[..], BencodeError::UnexpectedEof),
            (
                &b"lxi42ee"[..],
                BencodeError::UnexpectedBencodeType {
                    expected: Some(BencodeType::Integer),
                    actual: ReceivedBencodeType::Unknown('x'),
                },
            ),
        ];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_error_case::<Vec<i64>>(&mut deserializer, expected_error);
        }
    }

    // Mixed Type Lists
    #[test]
    fn mixed_list_tests() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct IntWrapper(i64);

        // Test with Vec<Option<i64>> to check handling of different types
        let data = b"li42eli13eee";
        let _deserializer = BencodeDeserializer::new(&data[..]);

        // This will fail until you implement proper type handling
        // Uncomment when you implement dictionary support
        // let result: Vec<serde_json::Value> = serde_json::Value::deserialize(&mut deserializer).expect("Failed to deserialize");
        // assert_eq!(result.len(), 2);
    }

    // Map/Dictionary Tests
    #[test]
    fn map_happy_cases() {
        use std::collections::HashMap;

        // Simple map cases
        let cases = [
            // Empty map
            (&b"de"[..], HashMap::<String, i64>::new()),
            // Map with a single string -> int entry
            (&b"d3:fooi42ee"[..], {
                let mut map = HashMap::new();
                map.insert("foo".to_string(), 42i64);
                map
            }),
            // Map with multiple entries
            (&b"d3:fooi42e3:bari-13ee"[..], {
                let mut map = HashMap::new();
                map.insert("foo".to_string(), 42i64);
                map.insert("bar".to_string(), -13i64);
                map
            }),
        ];

        for (data, expected_value) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            // Comment this line out until dictionary support is implemented
            // test_happy_case(&mut deserializer, expected_value);

            // For now, just ensure it fails as expected with the TODO message
            let result: Result<HashMap<String, i64>, _> = HashMap::deserialize(&mut deserializer);
            assert_eq!(Ok(expected_value), result);
        }
    }

    #[test]
    fn map_nested_cases() {
        use std::collections::HashMap;

        // Map with nested structures
        let case1 = &b"d4:userd4:val1i100e4:val2i25eee"[..];
        let mut expected1: HashMap<String, HashMap<String, i64>> = HashMap::new();
        let mut user_map = HashMap::new();
        user_map.insert("val1".to_string(), 100i64);
        user_map.insert("val2".to_string(), 25i64);
        expected1.insert("user".to_string(), user_map);

        // // Map with list values
        // let case2 = &b"d4:listli1ei2ei3ee5:valuei42ee"[..];
        // let mut expected2 = HashMap::new();
        // expected2.insert("list".to_string(), vec![1i64, 2i64, 3i64]);
        // expected2.insert("value".to_string(), 42i64);

        // These will fail until dictionary support is implemented
        let mut deserializer1 = BencodeDeserializer::new(case1);
        test_happy_case(&mut deserializer1, expected1);

        // let mut deserializer2 = BencodeDeserializer::new(case2);
        // test_happy_case(&mut deserializer2, expected2);
    }

    #[test]
    fn map_error_cases() {
        use std::collections::HashMap;

        let cases = [
            (&b"d"[..], BencodeError::UnexpectedEof),
            (&b"d3:foo"[..], BencodeError::UnexpectedEof),
            (&b"di42ei43ee"[..], BencodeError::InvalidKey {
                actual: ReceivedBencodeType::Known(Integer),
            }),
            (&b"d3:fooi42e"[..], BencodeError::UnexpectedEof),
            (&b"d3:fooe"[..], BencodeError::UnexpectedEof),
        ];

        for (data, expected_error) in cases {
            let mut deserializer = BencodeDeserializer::new(data);
            test_error_case::<HashMap<String, i64>>(&mut deserializer, expected_error);
        }
    }
    //
    // #[test]
    // fn complex_structure_tests() {
    //     // Test with a more complex data structure that would be commonly seen in bencode
    //     // (like a torrent file structure)
    //
    //     #[derive(Debug, Deserialize, PartialEq)]
    //     struct TorrentInfo {
    //         name: String,
    //         length: i64,
    //         #[serde(rename = "piece length")]
    //         piece_length: i64,
    //     }
    //
    //     #[derive(Debug, Deserialize, PartialEq)]
    //     struct Torrent {
    //         announce: String,
    //         info: TorrentInfo,
    //     }
    //
    //     // Example of a simplified torrent file in bencode
    //     let torrent_data = b"d8:announce30:http://tracker.example.com/announce4:infod4:name10:ubuntu.iso6:lengthi123456789e12:piece lengthi16384eee";
    //
    //     // This will fail until dictionary support is implemented
    //     // let mut deserializer = BencodeDeserializer::new(&torrent_data[..]);
    //     // let torrent: Torrent = Torrent::deserialize(&mut deserializer).expect("Failed to deserialize torrent");
    //     // assert_eq!(torrent.announce, "http://tracker.example.com/announce");
    //     // assert_eq!(torrent.info.name, "ubuntu.iso");
    //     // assert_eq!(torrent.info.length, 123456789);
    //     // assert_eq!(torrent.info.piece_length, 16384);
    // }
    //
    #[test]
    fn ordered_dict_test() {
        // Bencode dictionaries should be sorted by key,
        // Test preservation of insertion order

        #[derive(Debug, Deserialize, PartialEq)]
        struct OrderedData {
            z: i64,
            a: i64,
            m: i64,
        }

        let data = b"d1:ai1e1:mi2e1:zi3ee";

        // This will fail until dictionary support is implemented
        let mut deserializer = BencodeDeserializer::new(&data[..]);
        let ordered: OrderedData =
            OrderedData::deserialize(&mut deserializer).expect("Failed to deserialize");
        assert_eq!(ordered.z, 3);
        assert_eq!(ordered.a, 1);
        assert_eq!(ordered.m, 2);
    }

    // Nested structures tests (for future implementation)
    #[test]
    fn nested_structures() {
        // This is for future implementation when you support dictionaries
        #[derive(Debug, Deserialize, PartialEq)]
        struct Person {
            name: String,
            age: i64,
            hobbies: Vec<String>,
        }

        // Will be implemented later when dictionary support is added
        let data = b"d4:name5:Alice3:agei25e7:hobbiesl7:reading5:musicee";
        let mut deserializer = BencodeDeserializer::new(&data[..]);
        let person: Person = Person::deserialize(&mut deserializer).expect("Failed to deserialize");
        assert_eq!(person.name, "Alice");
        assert_eq!(person.age, 25);
        assert_eq!(person.hobbies, vec!["reading", "music"]);
    }
}
