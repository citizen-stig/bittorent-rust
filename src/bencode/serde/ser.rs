use crate::bencode::core::{DICT, END, LIST};
use crate::bencode::error::BencodeSerializationError;
use serde::Serialize;
use std::fmt::Display;

struct BencodeSerializer;

impl serde::ser::Error for BencodeSerializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        BencodeSerializationError::Custom(std::borrow::Cow::Owned(msg.to_string()))
    }
}

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
