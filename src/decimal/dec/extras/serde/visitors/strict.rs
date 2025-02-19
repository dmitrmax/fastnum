use core::fmt;
use serde::de;

type D<const N: usize> = crate::decimal::Decimal<N>;

pub struct Visitor<const N: usize>;

impl<const N: usize> Visitor<N> {
    pub const fn default() -> Self {
        Self
    }
}

impl<'de, const N: usize> de::Visitor<'de> for Visitor<N> {
    type Value = D<N>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "formatted decimal string in strict mode")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        D::<N>::from_str(value).map_err(|err| E::custom(format!("{}", err)))
    }
}
