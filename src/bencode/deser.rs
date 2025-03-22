use crate::bencode::core::{BencodeType, DICT, END, INT, LIST};
use crate::bencode::{BencodeDeserializer, BencodeError};
use serde::de::{DeserializeSeed, Visitor};
use serde::forward_to_deserialize_any;

#[derive(Debug)]
struct BencodeSeqAccess<'de, 'a> {
    de: &'a mut BencodeDeserializer<'de>,
}

impl<'de> serde::de::SeqAccess<'de> for BencodeSeqAccess<'de, '_> {
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
            None => Err(BencodeError::UnexpectedEof),
            Some(&INT) => self.deserialize_i64(v),
            Some(&LIST) => self.deserialize_seq(v),
            Some(&DICT) => self.deserialize_map(v),
            // THIS OVERFLOWS
            Some(b'0'..=b'9') => self.deserialize_bytes(v),
            // THIS WORKS:
            // Some(b'0'..=b'9') => self.deserialize_str(v),
            Some(b) => Err(BencodeError::UnexpectedBencodeType {
                expected: None,
                actual: BencodeType::from_byte(*b),
            }),
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

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
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
            return Err(BencodeError::UnexpectedBencodeType {
                expected: Some(BencodeType::List),
                actual: BencodeType::from_byte(self.input[self.pos]),
            });
        }

        self.pos += 1;
        let seq_access = BencodeSeqAccess { de: self };

        visitor.visit_seq(seq_access)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
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
            return Err(BencodeError::UnexpectedBencodeType {
                expected: Some(BencodeType::Dict),
                actual: BencodeType::from_byte(self.input[self.pos]),
            });
        }
        self.pos += 1;
        let seq_access = BencodeSeqAccess { de: self };
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
