use crate::bencode::core::{BencodeType, DICT, END, INT, LIST};
use crate::bencode::{BencodeDeserializer, BencodeError};
use serde::de::{DeserializeSeed, Visitor};

#[derive(Debug)]
struct BencodeSeqAccess<'de, 'a> {
    de: &'a mut BencodeDeserializer<'de>,
}

// #[derive(thiserror::Error, Debug, PartialEq)]
// pub enum BencodeDeserializerError {
//     ParsingError(#[from] BencodeError),
//     #[error("{0}")]
//     UnsupportedType(&'static str),
// }

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
        println!("DESERIALIZE map access: {}", self.de.pos);
        if self.de.input.get(self.de.pos) == Some(&END) {
            self.de.pos += 1;
            return Ok(None);
        }
        let key = seed.deserialize(&mut *self.de)?;
        println!("DESERIALIZE map access key: {}", self.de.pos);
        Ok(Some(key))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        println!("DESERIALIZE next value seed : {}", self.de.pos);
        seed.deserialize(&mut *self.de)
    }
}

impl<'de> serde::de::Deserializer<'de> for &mut BencodeDeserializer<'de> {
    type Error = BencodeError;

    // forward_to_deserialize_any! {
    //     bool i8 i16 i32 u8 u16 u32 u64 f32 f64 char
    //     option unit unit_struct
    //     newtype_struct tuple_struct tuple enum
    //     identifier ignored_any
    // }

    fn deserialize_any<V>(
        self,
        v: V,
    ) -> Result<<V as Visitor<'de>>::Value, <Self as serde::Deserializer<'de>>::Error>
    where
        V: Visitor<'de>,
    {
        println!("DESERIALIZE ANY: {}", self.pos);
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

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("DESERIALIZE i64: {}", self.pos);
        visitor.visit_i64(self.parse_integer()?)
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("DESERIALIZE u64: {}", self.pos);
        todo!()
        // visitor.visit_i64(self.parse_integer()?)
        // let integer = self.parse_integer()?.try_into().unwrap();
        // visitor.visit_u64(integer)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
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

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let output = self.parse_bytes()?;
        visitor.visit_byte_buf(output.to_vec())
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

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

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("DESERIALIZE map: {}", self.pos);
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
        println!("DESERIALIZE struct: {}", self.pos);
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // // In Bencode, dictionary keys are byte strings
        // let iden= self.parse_str()?;
        // visitor.visit_str(iden)
        println!("DESERIALIZE ident: {}", self.pos);
        let bytes = self.parse_bytes()?;

        // Try to convert the bytes to a string
        match std::str::from_utf8(bytes) {
            Ok(s) => visitor.visit_str(s),
            Err(err) => {
                println!("OOOPS: {:?}", err);
                visitor.visit_bytes(bytes)
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("DESERIALIZE ignored_any: {}", self.pos);
        // Skip the current value, whatever it is
        self.skip_value()?;

        // Call visit_unit since we're ignoring the actual value
        visitor.visit_unit()
    }
}
