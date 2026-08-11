use std::borrow::Cow;
use std::fmt;
use std::iter::{IntoIterator, empty};

use serde::de::value::{MapDeserializer, SeqDeserializer};
use serde::de::{self, IntoDeserializer};

pub struct Prefixed<'a>(Cow<'a, str>);
struct Val(String, String);
struct Varname(String);

struct Deserializer<'de, Iter: Iterator<Item = (String, String)>> {
    inner: MapDeserializer<'de, Vars<Iter>, Error>,
}

struct Vars<Iter>
where
    Iter: IntoIterator<Item = (String, String)>,
{
    inner: Iter,
}

impl<'de> IntoDeserializer<'de, Error> for Val {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> IntoDeserializer<'de, Error> for Varname {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<Iter: Iterator<Item = (String, String)>> Iterator for Vars<Iter> {
    type Item = (Varname, Val);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (Varname(k.clone()), Val(k, v)))
    }
}

macro_rules! forward_parsed_vals {
    ($($ty:ident => $method:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, Error>
            where
                V: de::Visitor<'de>
            {
                match self.1.parse::<$ty>() {
                    Ok(val) => val.into_deserializer().$method(visitor),
                    Err(e) => Err(serde::de::Error::custom(format_args!(
                        "{}: while parsing '{}' (provider: {})",
                        e, self.1, self.0
                    )))
                }
            }
        )*
    };
}

impl<'de> serde::de::Deserializer<'de> for Val {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.1.into_deserializer().deserialize_any(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.1.is_empty() {
            SeqDeserializer::new(empty::<Val>()).deserialize_seq(visitor)
        } else {
            let values = self
                .1
                .split(',')
                .map(|v| Val(self.0.clone(), v.trim().to_owned()));
            SeqDeserializer::new(values).deserialize_seq(visitor)
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(self.1.into_deserializer())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    forward_parsed_vals! {
        bool => deserialize_bool,
        u8 => deserialize_u8,
        u16 => deserialize_u16,
        u32 => deserialize_u32,
        u64 => deserialize_u64,
        i8 => deserialize_i8,
        i16 => deserialize_i16,
        i32 => deserialize_i32,
        i64 => deserialize_i64,
        f32 => deserialize_f32,
        f64 => deserialize_f64,
    }

    serde::forward_to_deserialize_any! {
        char str string unit bytes byte_buf map
        unit_struct tuple_struct identifier tuple
        ignored_any
        struct
    }
}

impl<'de> serde::de::Deserializer<'de> for Varname {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.0.into_deserializer().deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        char str string unit seq option bytes byte_buf map
        unit_struct tuple_struct identifier tuple ignored_any
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 enum struct
    }
}

impl<'de, Iter: Iterator<Item = (String, String)>> Deserializer<'de, Iter> {
    fn new(vars: Iter) -> Self {
        Deserializer {
            inner: MapDeserializer::new(Vars { inner: vars }),
        }
    }
}

impl<'de, Iter: Iterator<Item = (String, String)>> serde::de::Deserializer<'de>
    for Deserializer<'de, Iter>
{
    type Error = super::env::Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self.inner)
    }

    serde::forward_to_deserialize_any! {
        char str string unit seq option bytes byte_buf
        newtype_struct unit_struct tuple_struct identifier
        tuple ignored_any bool u8 u16 u32 u64 i8 i16 i32 i64
        f32 f64 enum struct
    }
}

pub fn from_env<T>() -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let vars = dotenvy::vars();
    from_iter(vars)
}

pub fn from_iter<Iter, T>(iter: Iter) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
    Iter: IntoIterator<Item = (String, String)>,
{
    T::deserialize(Deserializer::new(iter.into_iter()))
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Custom(msg.to_string())
    }

    fn missing_field(field: &'static str) -> Self {
        Error::MissingValue(field.into())
    }
}

#[allow(clippy::wrong_self_convention, dead_code)]
impl<'a> Prefixed<'a> {
    pub fn from_env<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.from_iter(dotenvy::vars())
    }

    pub fn from_iter<Iter, T>(&self, iter: Iter) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
        Iter: IntoIterator<Item = (String, String)>,
    {
        from_iter(iter.into_iter().filter_map(|(k, v)| {
            if k.starts_with(self.0.as_ref()) {
                Some((k.trim_start_matches(self.0.as_ref()).to_owned(), v))
            } else {
                None
            }
        }))
    }
}

#[derive(Debug)]
pub enum Error {
    Custom(String),
    MissingValue(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Custom(s) => write!(f, "env deserialization error: {s}"),
            Error::MissingValue(s) => write!(f, "environment variable '{s}' missing"),
        }
    }
}

impl std::error::Error for Error {}
