use crate::{Error, LinkedHashMap, Yaml};
use serde::ser::{self, SerializeMap};

pub struct YamlSerializer;

impl Default for YamlSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlSerializer {
    #[must_use] 
    pub const fn new() -> Self {
        Self
    }
}

impl ser::Serializer for YamlSerializer {
    type Ok = Yaml;
    type Error = Error;

    type SerializeSeq = VecSerializer;
    type SerializeTuple = VecSerializer;
    type SerializeTupleStruct = VecSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Boolean(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Integer(v as i64))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Integer(v as i64))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Integer(v as i64))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Integer(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Integer(v as i64))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Integer(v as i64))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Integer(v as i64))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        if v <= i64::MAX as u64 {
            Ok(Yaml::Integer(v as i64))
        } else {
            // For values larger than i64::MAX, use Real to avoid overflow
            Ok(Yaml::Real(v.to_string()))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Real(v.to_string()))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Real(v.to_string()))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
            Ok(Yaml::Integer(v as i64))
        } else {
            Ok(Yaml::Real(v.to_string()))
        }
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        if v <= i64::MAX as u128 {
            Ok(Yaml::Integer(v as i64))
        } else {
            Ok(Yaml::Real(v.to_string()))
        }
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let vec = v.iter().map(|&b| Yaml::Integer(b as i64)).collect();
        Ok(Yaml::Array(vec))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Null)
    }

    fn serialize_some<T: ?Sized + ser::Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let mut map = LinkedHashMap::new();
        map.insert(Yaml::String(variant.to_string()), value.serialize(self)?);
        Ok(Yaml::Hash(map))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(VecSerializer {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(VecSerializer {
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(VecSerializer {
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer {
            name: variant.to_string(),
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            map: LinkedHashMap::new(),
            key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(MapSerializer {
            map: LinkedHashMap::new(),
            key: None,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer {
            name: variant.to_string(),
            map: LinkedHashMap::new(),
        })
    }
}

pub struct VecSerializer {
    vec: Vec<Yaml>,
}

impl ser::SerializeSeq for VecSerializer {
    type Ok = Yaml;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.vec.push(value.serialize(YamlSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Array(self.vec))
    }
}

impl ser::SerializeTuple for VecSerializer {
    type Ok = Yaml;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for VecSerializer {
    type Ok = Yaml;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub struct TupleVariantSerializer {
    name: String,
    vec: Vec<Yaml>,
}

impl ser::SerializeTupleVariant for TupleVariantSerializer {
    type Ok = Yaml;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.vec.push(value.serialize(YamlSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = LinkedHashMap::new();
        map.insert(Yaml::String(self.name), Yaml::Array(self.vec));
        Ok(Yaml::Hash(map))
    }
}

pub struct MapSerializer {
    map: LinkedHashMap<Yaml, Yaml>,
    key: Option<Yaml>,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = Yaml;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.key = Some(key.serialize(YamlSerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::Custom("no key".to_string()))?;
        let val = value.serialize(YamlSerializer)?;
        self.map.insert(key, val);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Yaml::Hash(self.map))
    }
}

impl ser::SerializeStruct for MapSerializer {
    type Ok = Yaml;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.serialize_key(key)?;
        self.serialize_value(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeMap::end(self)
    }
}

pub struct StructVariantSerializer {
    name: String,
    map: LinkedHashMap<Yaml, Yaml>,
}

impl ser::SerializeStructVariant for StructVariantSerializer {
    type Ok = Yaml;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.map.insert(
            Yaml::String(key.to_string()),
            value.serialize(YamlSerializer)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut outer = LinkedHashMap::new();
        outer.insert(Yaml::String(self.name), Yaml::Hash(self.map));
        Ok(Yaml::Hash(outer))
    }
}
