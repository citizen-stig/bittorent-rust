use std::fmt::Display;
use serde::de::{DeserializeSeed, Visitor};
use serde::forward_to_deserialize_any;
use crate::bencode::{BencodeDeserializationError, BencodeDeserializer};
use crate::bencode::core::{BencodeType, DICT, END, INT, LIST};

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
