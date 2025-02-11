use core::error::Error;

use diesel::{
    data_types::PgNumeric,
    deserialize,
    deserialize::FromSql,
    pg::{Pg, PgValue},
    serialize,
    serialize::{Output, ToSql},
    sql_types::Numeric,
};

use crate::decimal::{
    errors::parse::pretty_error_msg, extras::utils::db::postgres::NBase, Decimal, ParseError,
    UnsignedDecimal,
};

type D<const N: usize> = Decimal<N>;
type UD<const N: usize> = UnsignedDecimal<N>;

impl<const N: usize> TryFrom<PgNumeric> for UD<N> {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(numeric: PgNumeric) -> deserialize::Result<Self> {
        let nbase: NBase = numeric.into();
        let dec: D<N> = nbase
            .try_into()
            .map_err(|e| pretty_error_msg(Self::type_name().as_str(), e))?;

        if dec.is_negative() {
            return Err(pretty_error_msg(Self::type_name().as_str(), ParseError::Signed).into());
        }

        Ok(UD::new(dec))
    }
}

impl<const N: usize> TryFrom<UD<N>> for PgNumeric {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(dec: UD<N>) -> deserialize::Result<Self> {
        let nbase: NBase = dec
            .0
            .try_into()
            .map_err(|e| pretty_error_msg(D::<N>::type_name().as_str(), e))?;

        Ok(nbase.into())
    }
}

impl<const N: usize> ToSql<Numeric, Pg> for UD<N> {
    #[inline]
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let numeric = PgNumeric::try_from(*self)?;
        ToSql::<Numeric, Pg>::to_sql(&numeric, &mut out.reborrow())
    }
}

impl<const N: usize> FromSql<Numeric, Pg> for UD<N> {
    #[inline]
    fn from_sql(numeric: PgValue<'_>) -> deserialize::Result<Self> {
        PgNumeric::from_sql(numeric)?.try_into()
    }
}
