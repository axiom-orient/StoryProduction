//! Private ordered JSON DOM for the authoring producer byte profile.
//! It does not change serde_json's workspace-wide map representation or feature set.

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    fmt,
    ops::{Index, IndexMut},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Map(indexmap::IndexMap<String, Value>);
impl Map {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
    pub fn insert(&mut self, key: String, value: Value) {
        self.0.insert(key, value);
    }
    pub fn remove(&mut self, key: &str) {
        self.0.shift_remove(key);
    }
}
// JSON object equality is independent of field order; field order affects only publication bytes.
impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .all(|(key, value)| other.get(key) == Some(value))
    }
}
impl Eq for Map {}
impl Serialize for Map {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in self.iter() {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Value>),
    Object(Map),
}
impl Value {
    pub fn as_object(&self) -> Option<&Map> {
        if let Self::Object(map) = self {
            Some(map)
        } else {
            None
        }
    }
    pub fn as_object_mut(&mut self) -> Option<&mut Map> {
        if let Self::Object(map) = self {
            Some(map)
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        if let Self::Array(values) = self {
            Some(values)
        } else {
            None
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        if let Self::Number(value) = self {
            value.as_u64()
        } else {
            None
        }
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object().and_then(|map| map.get(key))
    }
    pub fn to_semantic(&self) -> serde_json::Value {
        // Conversion is used only for shape/invariant admission and the public in-process result.
        // It is never used to prepare immutable authoring publication bytes.
        serde_json::to_value(self).expect("ordered DOM is valid serializable JSON")
    }
}
impl Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Value {
        static NULL: Value = Value::Null;
        self.get(key).unwrap_or(&NULL)
    }
}
impl Index<&String> for Value {
    type Output = Value;
    fn index(&self, key: &String) -> &Value {
        &self[key.as_str()]
    }
}
impl IndexMut<&str> for Value {
    fn index_mut(&mut self, key: &str) -> &mut Value {
        if self.is_null() {
            *self = Value::Object(Map::new());
        }
        let map = self
            .as_object_mut()
            .expect("field assignment requires an object");
        if !map.contains_key(key) {
            map.insert(key.to_owned(), Value::Null);
        }
        map.0.get_mut(key).expect("field inserted")
    }
}
impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}
impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}
impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}
impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => value.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(value)?;
                }
                sequence.end()
            }
            Self::Object(map) => map.serialize(serializer),
        }
    }
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        serde_json::to_string(self).map_err(|_| fmt::Error)?.fmt(f)
    }
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("strict ordered JSON")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Number(value.into()))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
                Ok(Value::Number(value.into()))
            }
            fn visit_f64<E: de::Error>(self, _value: f64) -> Result<Value, E> {
                Err(E::custom("NON_CANONICAL_JSON_NUMBER_OR_VALUE"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
                Ok(Value::String(value.to_owned()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = access.next_element()? {
                    values.push(value);
                }
                Ok(Value::Array(values))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
                let mut values = Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!("DUPLICATE_JSON_KEY:{key}")));
                    }
                    values.insert(key, access.next_value()?);
                }
                Ok(Value::Object(values))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}

pub(crate) fn to_value(value: &impl Serialize) -> Value {
    // Serialize directly into the private DOM. Going through serde_json::Value would lose
    // input object order under its default sorted-map profile.
    let bytes = serde_json::to_vec(value).expect("producer builder contains serializable JSON");
    serde_json::from_slice(&bytes).expect("producer builder contains integer-only JSON")
}
macro_rules! json {
    ({$($key:literal : $value:expr),* $(,)?}) => {{
        let mut object = $crate::codec::Map::new();
        $(object.insert($key.to_owned(), $crate::codec::value::to_value(&$value));)*
        $crate::codec::Value::Object(object)
    }};
}
pub(crate) use json;
