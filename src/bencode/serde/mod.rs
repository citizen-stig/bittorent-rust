use crate::bencode::core::{BencodeType, DICT, END, INT, LIST};
use crate::bencode::error::BencodeSerializationError;
use crate::bencode::{BencodeDeserializationError, BencodeDeserializer};
use serde::de::{DeserializeSeed, Visitor};
use serde::{forward_to_deserialize_any, Serialize};
use std::fmt::Display;

impl serde::ser::Error for BencodeSerializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        BencodeSerializationError::Custom(std::borrow::Cow::Owned(msg.to_string()))
    }
}

impl<'de> serde::de::Deserializer<'de> for &mut BencodeDeserializer<'de> {
    type Error = BencodeDeserializationError;

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
            None => Err(BencodeDeserializationError::UnexpectedEof),
            Some(&INT) => self.deserialize_i64(visitor),
            Some(&LIST) => self.deserialize_seq(visitor),
            Some(&DICT) => self.deserialize_map(visitor),
            Some(b'0'..=b'9') => self.deserialize_bytes(visitor),
            Some(b) => Err(BencodeDeserializationError::UnexpectedBencodeType {
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
                return Err(BencodeDeserializationError::UnexpectedEof);
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

impl serde::de::Error for BencodeDeserializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        BencodeDeserializationError::Custom(std::borrow::Cow::Owned(msg.to_string()))
    }
}

struct BencodeSeqAccess<'de, 'a> {
    de: &'a mut BencodeDeserializer<'de>,
}

impl<'de, 'a> BencodeSeqAccess<'de, 'a> {
    pub(crate) fn new_list(
        de: &'a mut BencodeDeserializer<'de>,
    ) -> Result<Self, BencodeDeserializationError> {
        de.check_type(BencodeType::List)?;
        de.pos = de.pos.checked_add(1).expect("Position overflow");
        Ok(Self { de })
    }

    pub(crate) fn new_dict(
        de: &'a mut BencodeDeserializer<'de>,
    ) -> Result<Self, BencodeDeserializationError> {
        de.check_type(BencodeType::Dict)?;
        de.pos = de.pos.checked_add(1).expect("Position overflow");
        Ok(Self { de })
    }
}

impl<'de> serde::de::SeqAccess<'de> for BencodeSeqAccess<'de, '_> {
    type Error = BencodeDeserializationError;

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
    type Error = BencodeDeserializationError;

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
                return Err(BencodeDeserializationError::UnexpectedEof);
            }
            Some(b'0'..=b'9') => {}
            Some(b) => {
                return Err(BencodeDeserializationError::InvalidKey {
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

struct BencodeSerializer;

pub fn to_bencode<T: ?Sized + Serialize>(value: &T) -> Result<Vec<u8>, BencodeSerializationError> {
    T::serialize(value, BencodeSerializer)
}

struct BencodeListSerializer {
    output: Vec<u8>,
}

#[allow(dead_code)]
impl BencodeListSerializer {
    pub(crate) fn new() -> Self {
        Self { output: vec![LIST] }
    }

    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.output.push(END);
        self.output
    }
}

impl serde::ser::SerializeSeq for BencodeListSerializer {
    type Ok = Vec<u8>;
    type Error = BencodeSerializationError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(BencodeSerializer)?;
        self.output.extend(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.finish())
    }
}

type PreSerializeKey = (Vec<u8>, Vec<u8>);
type SerializedValue = Vec<u8>;

struct BencodeMapSerializer {
    key_values: Vec<(PreSerializeKey, SerializedValue)>,
}

impl BencodeMapSerializer {
    pub(crate) fn new() -> Self {
        Self { key_values: vec![] }
    }

    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.key_values.sort_unstable_by(|a, b| a.0 .1.cmp(&b.0 .1));
        let total_len = self
            .key_values
            .iter()
            .map(|(k, v)| k.0.len() + k.1.len() + v.len())
            .sum::<usize>()
            + 2;

        let mut output = Vec::with_capacity(total_len);
        output.push(DICT);

        for (key, value) in self.key_values {
            output.extend(key.0);
            output.extend(key.1);
            output.extend(value);
        }

        output.push(END);
        output
    }
}

struct KeySerializer {}

impl serde::Serializer for KeySerializer {
    type Ok = PreSerializeKey;
    type Error = BencodeSerializationError;

    type SerializeSeq = serde::ser::Impossible<PreSerializeKey, BencodeSerializationError>;
    type SerializeTuple = serde::ser::Impossible<PreSerializeKey, BencodeSerializationError>;
    type SerializeTupleStruct = serde::ser::Impossible<PreSerializeKey, BencodeSerializationError>;
    type SerializeTupleVariant = serde::ser::Impossible<PreSerializeKey, BencodeSerializationError>;
    type SerializeMap = serde::ser::Impossible<PreSerializeKey, BencodeSerializationError>;
    type SerializeStruct = serde::ser::Impossible<PreSerializeKey, BencodeSerializationError>;
    type SerializeStructVariant =
    serde::ser::Impossible<PreSerializeKey, BencodeSerializationError>;

    // Everything else errors out explicitly
    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }

    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }

    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        // TODO: Improve copy/cloning
        let prefix = format!("{}:", v.len()).as_bytes().to_vec();
        let value = v.as_bytes().to_vec();
        Ok((prefix, value))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let prefix = format!("{}:", v.len()).as_bytes().to_vec();
        Ok((prefix, v.to_vec()))
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_some<T>(self, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }

    fn serialize_newtype_struct<T>(self, _: &'static str, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(BencodeSerializationError::InvalidMapKey)
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(BencodeSerializationError::InvalidMapKey)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(BencodeSerializationError::InvalidMapKey)
    }
}

impl serde::ser::SerializeMap for BencodeMapSerializer {
    type Ok = Vec<u8>;
    type Error = BencodeSerializationError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = key.serialize(KeySerializer {})?;

        self.key_values.push((key, Vec::new()));

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let serialized_value = value.serialize(BencodeSerializer)?;
        let pair = self.key_values.last_mut().expect("No key");
        pair.1 = serialized_value;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.finish())
    }
}

impl serde::ser::SerializeStruct for BencodeMapSerializer {
    type Ok = Vec<u8>;
    type Error = BencodeSerializationError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let serialized_key = key.serialize(KeySerializer {})?;
        let serialize_value = value.serialize(BencodeSerializer)?;
        self.key_values.push((serialized_key, serialize_value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.finish())
    }
}

impl serde::Serializer for BencodeSerializer {
    type Ok = Vec<u8>;
    type Error = BencodeSerializationError;
    type SerializeSeq = BencodeListSerializer;
    type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = BencodeMapSerializer;
    type SerializeStruct = BencodeMapSerializer;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("bool"))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(format!("i{}e", v).as_bytes().to_vec())
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("f32"))
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("f64"))
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("char"))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let len = v.len();
        let value = format!("{}:{}", len, v).as_bytes().to_vec();
        Ok(value)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let len = v.len();
        let mut prefix = format!("{}:", len).as_bytes().to_vec();
        prefix.extend_from_slice(v);
        Ok(prefix)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("none"))
    }

    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(BencodeSerializationError::UnsupportedType("some"))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("unit"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("unit_struct"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("unit_variant"))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(BencodeSerializationError::UnsupportedType("newtype_struct"))
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(BencodeSerializationError::UnsupportedType(
            "newtype_variant",
        ))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(BencodeListSerializer::new())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("tuple"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("tuple_struct"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("tuple_variant"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(BencodeMapSerializer::new())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(BencodeMapSerializer::new())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(BencodeSerializationError::UnsupportedType("struct_variant"))
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
        expected_error: BencodeDeserializationError,
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
            (&b"i"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"ie"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"i42"[..], BencodeDeserializationError::UnexpectedEof),
            (
                &b"i42x"[..],
                BencodeDeserializationError::InvalidInteger('x'),
            ),
            (
                &b"i9223372036854775808e"[..],
                BencodeDeserializationError::CannotParseInteger(
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
            (&b"l"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"li42e"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"li42"[..], BencodeDeserializationError::UnexpectedEof),
            (
                &b"lxi42ee"[..],
                BencodeDeserializationError::UnexpectedBencodeType {
                    expected: Some(Integer),
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
            (&b"d"[..], BencodeDeserializationError::UnexpectedEof),
            (&b"d3:foo"[..], BencodeDeserializationError::UnexpectedEof),
            (
                &b"di42ei43ee"[..],
                BencodeDeserializationError::InvalidKey {
                    actual: ReceivedBencodeType::Known(Integer),
                },
            ),
            (
                &b"d3:fooi42e"[..],
                BencodeDeserializationError::UnexpectedEof,
            ),
            (&b"d3:fooe"[..], BencodeDeserializationError::UnexpectedEof),
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

    // Helper function to serialize a value and compare with the expected output.
    fn test_serialize<T>(value: T, expected: &[u8])
    where
        T: Serialize,
    {
        println!("Testing serialization of {:?}", std::any::type_name::<T>());
        let serializer = BencodeSerializer;
        let result = value.serialize(serializer).expect("Serialization failed");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_serialize_integers() {
        test_serialize(42i64, b"i42e");
        test_serialize(-42i64, b"i-42e");
        test_serialize(0i64, b"i0e");

        test_serialize(42i64, b"i42e");
        test_serialize(42i64, b"i42e");
    }
}
