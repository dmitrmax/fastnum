//! # Signed Decimal

mod cmp;
mod extras;
mod format;
mod impls;
mod math;
mod normalize;
mod parse;
mod scale;

use impls::consts::consts_impl;

use core::{cmp::Ordering, fmt};

use crate::{
    decimal::{
        doc, Category, Context, Flags, ParseError, RoundingMode, Sign, Signal, UnsignedDecimal,
    },
    int::UInt,
};

/// # Decimal
///
/// Generic signed N-bits decimal number.
#[derive(Copy, Clone)]
pub struct Decimal<const N: usize> {
    /// An N-bit unsigned integer coefficient. Represent significant decimal
    /// digits.
    digits: UInt<N>,

    /// Scaling factor (or _exponent_) which determines the position of the
    /// decimal point and indicates the power of ten by which the coefficient is
    /// multiplied.
    scale: i16,

    /// Special values and signaling flags.
    flags: Flags,
}

consts_impl!();

impl<const N: usize> Decimal<N> {
    /// Creates and initializes a decimal from string.
    #[track_caller]
    #[inline]
    pub const fn from_str(s: &str) -> Result<Self, ParseError> {
        parse::from_str(s)
    }

    /// Creates and initializes an unsigned decimal from string.
    ///
    /// # Panics
    ///
    /// This function will panic if `UnsignedDecimal<N>` cannot be constructed
    /// from given string.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{UD256, udec256};
    ///
    /// assert_eq!(UD256::parse_str("1.2345"), udec256!(1.2345));
    /// ```
    #[track_caller]
    #[inline]
    pub const fn parse_str(s: &str) -> Self {
        match Self::from_str(s) {
            Ok(n) => n,
            Err(e) => {
                panic!("{}", e.description())
            }
        }
    }

    /// Returns the internal big integer, representing the significant
    /// decimal digits of a `Decimal`, including significant trailing
    /// zeros.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{dec256, u256};
    ///
    /// let a = dec256!(-123.45);
    /// assert_eq!(a.digits(), u256!(12345));
    ///
    /// let b = dec256!(-1.0);
    /// assert_eq!(b.digits(), u256!(10));
    /// ```
    #[inline]
    pub const fn digits(&self) -> UInt<N> {
        self.digits
    }

    /// Returns the count of digits in the non-scaled integer representation
    #[inline]
    pub const fn digits_count(&self) -> usize {
        if self.is_zero() {
            return 1;
        }
        self.digits.ilog10() as usize + 1
    }

    /// Returns the scale of the `Decimal`, the total number of
    /// digits to the right of the decimal point (including insignificant
    /// leading zeros).
    ///
    /// # Examples:
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// let a = dec256!(12345);  // No fractional part
    /// let b = dec256!(123.45);  // Fractional part
    /// let c = dec256!(0.0000012345);  // Completely fractional part
    /// let d = dec256!(500000000);  // No fractional part
    /// let e = dec256!(5e9);  // Negative-fractional part
    ///
    /// assert_eq!(a.fractional_digits_count(), 0);
    /// assert_eq!(b.fractional_digits_count(), 2);
    /// assert_eq!(c.fractional_digits_count(), 10);
    /// assert_eq!(d.fractional_digits_count(), 0);
    /// assert_eq!(e.fractional_digits_count(), -9);
    /// ```
    #[inline]
    pub const fn fractional_digits_count(&self) -> i16 {
        self.scale
    }

    /// Return the sign of the `Decimal` as [Sign].
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{decimal::Sign, dec256};
    ///
    /// assert_eq!(dec256!(-1.0).sign(), Sign::Minus);
    /// assert_eq!(dec256!(0.0).sign(),  Sign::Plus);
    /// assert_eq!(dec256!(+1.0).sign(),  Sign::Plus);
    /// ```
    #[inline]
    pub const fn sign(&self) -> Sign {
        if self.flags.is_negative() {
            Sign::Minus
        } else {
            Sign::Plus
        }
    }

    #[inline]
    pub const fn is_op_div_by_zero(&self) -> bool {
        self.flags.has_signal(Signal::OP_DIV_BY_ZERO)
    }

    #[inline]
    pub const fn is_op_invalid(&self) -> bool {
        self.flags.has_signal(Signal::OP_INVALID)
    }

    #[must_use]
    #[inline]
    pub const fn is_op_subnormal(&self) -> bool {
        self.flags.has_signal(Signal::OP_SUBNORMAL)
    }

    #[inline]
    pub const fn is_op_inexact(&self) -> bool {
        self.flags.has_signal(Signal::OP_INEXACT)
    }

    #[inline]
    pub const fn is_op_rounded(&self) -> bool {
        self.flags.has_signal(Signal::OP_ROUNDED)
    }

    #[inline]
    pub const fn is_op_clamped(&self) -> bool {
        self.flags.has_signal(Signal::OP_CLAMPED)
    }

    #[inline]
    pub const fn is_op_ok(&self) -> bool {
        !self.flags.has_signals()
    }

    /// Returns the decimal category of the number. If only one property
    /// is going to be tested, it is generally faster to use the specific
    /// predicate instead.
    ///
    /// ```
    /// use fastnum::{dec256, D256, decimal::Category};
    ///
    /// let num = dec256!(12.4);
    /// let inf = D256::INFINITY;
    ///
    /// assert_eq!(num.classify(), Category::Normal);
    /// assert_eq!(inf.classify(), Category::Infinite);
    /// ```
    #[inline]
    pub const fn classify(&self) -> Category {
        if self.flags.is_nan() {
            Category::Nan
        } else if self.flags.is_infinity() {
            Category::Infinite
        } else if self.digits.is_zero() {
            Category::Zero
        } else if self.is_subnormal() {
            Category::Subnormal
        } else {
            Category::Normal
        }
    }

    /// Returns `true` if the number is neither zero, infinite,
    /// [subnormal], or NaN.
    ///
    /// ```
    /// let min = f64::MIN_POSITIVE; // 2.2250738585072014e-308f64
    /// let max = f64::MAX;
    /// let lower_than_min = 1.0e-308_f64;
    /// let zero = 0.0f64;
    ///
    /// assert!(min.is_normal());
    /// assert!(max.is_normal());
    ///
    /// assert!(!zero.is_normal());
    /// assert!(!f64::NAN.is_normal());
    /// assert!(!f64::INFINITY.is_normal());
    /// // Values between `0` and `min` are Subnormal.
    /// assert!(!lower_than_min.is_normal());
    /// ```
    /// [subnormal]: https://en.wikipedia.org/wiki/Denormal_number

    #[inline]
    pub const fn is_normal(&self) -> bool {
        !self.is_subnormal()
    }

    /// Returns `true` if the number is [subnormal].
    ///
    /// ```
    /// let min = f64::MIN_POSITIVE; // 2.2250738585072014e-308_f64
    /// let max = f64::MAX;
    /// let lower_than_min = 1.0e-308_f64;
    /// let zero = 0.0_f64;
    ///
    /// assert!(!min.is_subnormal());
    /// assert!(!max.is_subnormal());
    ///
    /// assert!(!zero.is_subnormal());
    /// assert!(!f64::NAN.is_subnormal());
    /// assert!(!f64::INFINITY.is_subnormal());
    /// // Values between `0` and `min` are Subnormal.
    /// assert!(lower_than_min.is_subnormal());
    /// ```
    /// [subnormal]: https://en.wikipedia.org/wiki/Denormal_number
    #[must_use]
    #[inline]
    pub const fn is_subnormal(&self) -> bool {
        self.is_op_subnormal()
    }

    /// Returns `true` if this number is neither infinite nor `NaN`.
    ///
    /// ```
    /// use fastnum::{D256, dec256};
    ///
    /// let d = dec256!(7.0);
    /// let inf = D256::INFINITY;
    /// let neg_inf = D256::NEG_INFINITY;
    /// let nan = D256::NAN;
    ///
    /// assert!(d.is_finite());
    ///
    /// assert!(!nan.is_finite());
    /// assert!(!inf.is_finite());
    /// assert!(!neg_inf.is_finite());
    /// ```
    #[inline]
    pub const fn is_finite(&self) -> bool {
        !self.flags.is_special()
    }

    /// Returns `true` if this value is positive infinity or negative infinity
    /// and false otherwise.
    ///
    /// ```
    /// use fastnum::{D256, dec256};
    ///
    /// let d = dec256!(7.0);
    /// let inf = D256::INFINITY;
    /// let neg_inf = D256::NEG_INFINITY;
    /// let nan = D256::NAN;
    ///
    /// assert!(!d.is_infinite());
    /// assert!(!nan.is_infinite());
    ///
    /// assert!(inf.is_infinite());
    /// assert!(neg_inf.is_infinite());
    /// ```
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        self.flags.is_infinity()
    }

    /// Returns `true` if this value is `NaN` and false otherwise.
    ///
    /// ```
    /// use fastnum::{D256, dec256};
    ///
    /// let nan = D256::NAN;
    /// let d = dec256!(7.0);
    ///
    /// assert!(nan.is_nan());
    /// assert!(!d.is_nan());
    /// ```
    #[inline]
    pub const fn is_nan(&self) -> bool {
        self.flags.is_nan()
    }

    #[inline]
    pub const fn is_sign_positive(&self) -> bool {
        !self.flags.is_negative()
    }

    #[inline]
    pub const fn is_sign_negative(&self) -> bool {
        self.flags.is_negative()
    }

    /// Return if the referenced decimal is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{dec256};
    ///
    /// let a = dec256!(0);
    /// assert!(a.is_zero());
    ///
    /// let b = dec256!(0.0);
    /// assert!(b.is_zero());
    ///
    /// let c = dec256!(-0.00);
    /// assert!(c.is_zero());
    ///
    /// let d = dec256!(-0.1);
    /// assert!(!d.is_zero());
    /// ```
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.digits.is_zero()
    }

    /// Return if the referenced unsigned decimal is strictly [Self::ONE].
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{udec256};
    ///
    /// let a = udec256!(1);
    /// assert!(a.is_one());
    ///
    /// let b = udec256!(10e-1);
    /// assert!(!b.is_one());
    /// ```
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.digits.is_one() && self.scale == 0 && self.flags.is_empty()
    }

    /// Returns true if the decimal is positive and false if the decimal is zero
    /// or negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// // Positive
    /// assert!(dec256!(+1.0).is_positive());
    /// assert!(dec256!(1.0).is_positive());
    /// assert!(dec256!(+0).is_positive());
    /// assert!(dec256!(0).is_positive());
    ///
    /// // Not positive
    /// assert!(!dec256!(-0).is_positive());
    /// assert!(!dec256!(-1.0).is_positive());
    /// ```
    #[inline]
    pub const fn is_positive(&self) -> bool {
        !self.flags.is_negative()
    }

    /// Returns true if the decimal is strictly negative and false otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// // Negative
    /// assert!(dec256!(-0).is_negative());
    /// assert!(dec256!(-1.0).is_negative());
    ///
    /// // Not negative
    /// assert!(!dec256!(0).is_negative());
    /// assert!(!dec256!(+0).is_negative());
    /// assert!(!dec256!(1.0).is_negative());
    /// assert!(!dec256!(+1.0).is_negative());
    /// ```
    #[inline]
    pub const fn is_negative(&self) -> bool {
        self.flags.is_negative()
    }

    /// Invert sign of given decimal.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(+1.0).neg(), dec256!(-1.0));
    /// assert_eq!(dec256!(1.0).neg(), dec256!(-1.0));
    /// assert_eq!(dec256!(-1.0).neg(), dec256!(1.0));
    /// ```
    #[inline]
    pub const fn neg(mut self) -> Self {
        self.flags = self.flags.neg();
        self
    }

    /// Get the absolute value of the decimal (non-negative sign).
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(1.0).abs(), dec256!(1.0));
    /// assert_eq!(dec256!(-1.0).abs(), dec256!(1.0));
    /// ```
    #[inline]
    pub const fn abs(mut self) -> Self {
        self.flags = self.flags.abs();
        self
    }

    /// Get the absolute value of the decimal (non-negative sign) as
    /// [UnsignedDecimal].
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{dec256, udec256};
    ///
    /// assert_eq!(dec256!(1.0).unsigned_abs(), udec256!(1.0));
    /// assert_eq!(dec256!(-1.0).unsigned_abs(), udec256!(1.0));
    /// ```
    #[inline]
    pub const fn unsigned_abs(self) -> UnsignedDecimal<N> {
        UnsignedDecimal::new(self.abs())
    }

    /// Initialize decimal with `1 * 10`<sup>exp</sup> value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{UD256, udec256};
    ///
    /// assert_eq!(UD256::from_scale(0), udec256!(1));
    /// assert_eq!(UD256::from_scale(-0), udec256!(1));
    /// assert_eq!(UD256::from_scale(-3), udec256!(0.001));
    /// assert_eq!(UD256::from_scale(3), udec256!(1000));
    /// ```
    #[inline]
    pub const fn from_scale(exp: i16) -> Self {
        Self::new(UInt::ONE, -exp, Flags::default())
    }

    /// Returns a number that represents the sign of `self`.
    ///
    /// - `1.0` if the number is positive, `+0.0` or
    ///   [`INFINITY`](Self::INFINITY)
    /// - `-1.0` if the number is negative, `-0.0` or
    ///   [`NEG_INFINITY`](Self::NEG_INFINITY)
    /// - [`NAN`](Self::NAN) if the number is [`NAN`](Self::NAN)
    ///
    /// ```
    /// use fastnum::{D256, dec256};
    ///
    /// let d = dec256!(3.5);
    ///
    /// assert_eq!(d.signum(), dec256!(1.0));
    /// assert_eq!(D256::NEG_INFINITY.signum(), dec256!(-1.0));
    ///
    /// assert!(D256::NAN.signum().is_nan());
    /// ```
    #[inline]
    pub const fn signum(&self) -> Self {
        if self.is_nan() {
            Self::NAN
        } else if self.is_negative() {
            Self::ONE.neg()
        } else {
            Self::ONE
        }
    }

    /// __Normalize__ this decimal shift all significant trailing zeros into
    /// the exponent.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{dec256, u256, decimal::Context};
    ///
    /// let a = dec256!(-1234500);
    /// assert_eq!(a.digits(), u256!(1234500));
    /// assert_eq!(a.fractional_digits_count(), 0);
    ///
    /// let b = a.normalized(Context::default());
    /// assert_eq!(b.digits(), u256!(12345));
    /// assert_eq!(b.fractional_digits_count(), -2);
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn normalized(self, ctx: Context) -> Self {
        normalize::normalize(self).unwrap_signals(ctx)
    }

    /// Tests for `self` and `other` values to be equal, and is used by `==`
    /// operator.
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn eq(&self, other: &Self) -> bool {
        cmp::eq(self, other)
    }

    /// Tests for `self` and `other` values to be equal, and is used by `==`
    /// operator.
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn ne(&self, other: &Self) -> bool {
        cmp::ne(self, other)
    }

    /// Compares and returns the maximum of two signed decimal values.
    ///
    /// Returns the second argument if the comparison determines them to be
    /// equal.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{dec256};
    ///
    /// assert_eq!(dec256!(1).max(dec256!(2)), dec256!(2));
    /// assert_eq!(dec256!(2).max(dec256!(2)), dec256!(2));
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        match self.cmp(&other) {
            Ordering::Less | Ordering::Equal => other,
            _ => self,
        }
    }

    /// Compares and returns the minimum of two signed decimal values.
    ///
    /// Returns the first argument if the comparison determines them to be
    /// equal.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(1).min(dec256!(2)), dec256!(1));
    /// assert_eq!(dec256!(2).min(dec256!(2)), dec256!(2));
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        match self.cmp(&other) {
            Ordering::Less | Ordering::Equal => self,
            _ => other,
        }
    }

    /// Restrict a signed decimal value to a certain interval.
    ///
    /// Returns `max` if `self` is greater than `max`, and `min` if `self` is
    /// less than `min`. Otherwise, this returns `self`.
    ///
    /// # Panics
    ///
    /// Panics if `min > max`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(-3).clamp(dec256!(-2), dec256!(1)), dec256!(-2));
    /// assert_eq!(dec256!(0).clamp(dec256!(-2), dec256!(1)), dec256!(0));
    /// assert_eq!(dec256!(2).clamp(dec256!(-2), dec256!(1)), dec256!(1));
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        assert!(min.le(&max));
        if let Ordering::Less = self.cmp(&min) {
            min
        } else if let Ordering::Greater = self.cmp(&max) {
            max
        } else {
            self
        }
    }

    /// Tests signed decimal `self` less than `other` and is used by the `<`
    /// operator.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(1.0).lt(&dec256!(1.0)), false);
    /// assert_eq!(dec256!(1.0).lt(&dec256!(2.0)), true);
    /// assert_eq!(dec256!(2.0).lt(&dec256!(1.0)), false);
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn lt(&self, other: &Self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self.cmp(other) {
            Ordering::Less => true,
            _ => false,
        }
    }

    /// Tests signed decimal `self` less than or equal to `other` and is used by
    /// the `<=` operator.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(1.0).le(&dec256!(1.0)), true);
    /// assert_eq!(dec256!(1.0).le(&dec256!(2.0)), true);
    /// assert_eq!(dec256!(2.0).le(&dec256!(1.0)), false);
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn le(&self, other: &Self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self.cmp(other) {
            Ordering::Less | Ordering::Equal => true,
            _ => false,
        }
    }

    /// Tests signed decimal `self` greater than `other` and is used by the `>`
    /// operator.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(1.0).gt(&dec256!(1.0)), false);
    /// assert_eq!(dec256!(1.0).gt(&dec256!(2.0)), false);
    /// assert_eq!(dec256!(2.0).gt(&dec256!(1.0)), true);
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn gt(&self, other: &Self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self.cmp(other) {
            Ordering::Greater => true,
            _ => false,
        }
    }

    /// Tests signed decimal `self` greater than or equal to `other` and is used
    /// by the `>=` operator.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// assert_eq!(dec256!(1.0).ge(&dec256!(1.0)), true);
    /// assert_eq!(dec256!(1.0).ge(&dec256!(2.0)), false);
    /// assert_eq!(dec256!(2.0).ge(&dec256!(1.0)), true);
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn ge(&self, other: &Self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self.cmp(other) {
            Ordering::Greater | Ordering::Equal => true,
            _ => false,
        }
    }

    /// This method returns an [`Ordering`] between `self` and `other`.
    ///
    /// By convention, `self.cmp(&other)` returns the ordering matching the
    /// expression `self <operator> other` if true.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    /// use std::cmp::Ordering;
    ///
    /// assert_eq!(dec256!(5).cmp(&dec256!(10)), Ordering::Less);
    /// assert_eq!(dec256!(10).cmp(&dec256!(5)), Ordering::Greater);
    /// assert_eq!(dec256!(5).cmp(&dec256!(5)), Ordering::Equal);
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn cmp(&self, other: &Self) -> Ordering {
        cmp::cmp(self, other)
    }

    /// Calculates `self` + `rhs`.
    ///
    /// Is internally used by the `+` operator.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fastnum::{dec256, D256, decimal::{Context, RoundingMode}};
    ///
    /// let a = D256::ONE;
    /// let b = D256::TWO;
    ///
    /// let c = a + b;
    /// assert_eq!(c, dec256!(3));
    /// ```
    ///
    /// ```should_panic
    /// use fastnum::{dec256, D256};
    ///
    /// let a = D256::MAX;
    /// let b = D256::MAX;
    ///
    /// let c = a + b;
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn add(self, rhs: Self, ctx: Context) -> Self {
        math::add::add(self, rhs, ctx).unwrap_signals(ctx)
    }

    /// Calculates `self` - `rhs`.
    ///
    /// Returns the result of subtraction and [emergency
    /// flags](crate#arithmetic-result). Is internally used by the `-`
    /// operator.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fastnum::{dec256, D256, decimal::Context};
    ///
    /// let a = D256::ONE;
    /// let b = D256::TWO;
    ///
    /// let c = a.sub(b, Context::default());
    /// assert_eq!(c, dec256!(-1));
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn sub(self, rhs: Self, ctx: Context) -> Self {
        math::sub::sub(self, rhs, ctx).unwrap_signals(ctx)
    }

    /// Calculates `self` × `rhs`.
    ///
    /// Returns the result of multiplication and [emergency
    /// flags](crate#arithmetic-result). Is internally used by the `*`
    /// operator.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fastnum::{dec256, D256, decimal::Context};
    ///
    /// let a = D256::FIVE;
    /// let b = D256::TWO;
    ///
    /// let c = a.mul(b, Context::default());
    /// assert_eq!(c, dec256!(10));
    /// ```
    ///
    /// ```should_panic
    /// use fastnum::{dec256, D256};
    ///
    /// let a = D256::MAX;
    /// let b = D256::MAX;
    ///
    /// let c = a * b;
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn mul(self, rhs: Self, ctx: Context) -> Self {
        math::mul::mul(self, rhs, ctx).unwrap_signals(ctx)
    }

    /// Calculates `self` ÷ `rhs`.
    ///
    /// Returns the result of division and [emergency
    /// flags](crate#arithmetic-result). Is internally used by the `/`
    /// operator.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fastnum::{dec256, D256, decimal::Context};
    ///
    /// let a = D256::FIVE;
    /// let b = D256::TWO;
    ///
    /// let c = a.neg().div(b, Context::default());
    /// assert_eq!(c, dec256!(-2.5));
    /// ```
    ///
    /// ```should_panic
    /// use fastnum::{dec256, D256};
    ///
    /// let a = D256::ONE;
    /// let b = D256::ZERO;
    ///
    /// let c = a / b;
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn div(self, rhs: Self, ctx: Context) -> Self {
        math::div::div(self, rhs, ctx).unwrap_signals(ctx)
    }

    /// Calculates `self` % `rhs`.
    ///
    /// Returns the result of division reminder and [emergency
    /// flags](crate#arithmetic-result). Is internally used by the `%`
    /// operator.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fastnum::{dec256, D256, decimal::Context};
    ///
    /// let a = D256::FIVE;
    /// let b = D256::TWO;
    ///
    /// let c = a.rem(b.neg(), Context::default());
    /// assert_eq!(c, dec256!(1));
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn rem(self, rhs: Self, ctx: Context) -> Self {
        math::rem::rem(self, rhs, ctx).unwrap_signals(ctx)
    }

    /// Return given decimal number rounded to 'digits' precision after the
    /// decimal point, using given [RoundingMode] unwrapped with default
    /// rounding and overflow policy.
    ///
    /// # Panics:
    ///
    /// This method will panic if round operation (up-scale or down-scale)
    /// performs with some emergency flags and specified
    /// [crate::decimal::Context] enjoin to panic when the
    /// corresponding flag occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{dec256, decimal::RoundingMode};
    ///
    /// let n = dec256!(129.41675);
    ///
    /// assert_eq!(n.round(2, RoundingMode::Up),  dec256!(129.42));
    /// assert_eq!(n.round(-1, RoundingMode::Down),  dec256!(120));
    /// assert_eq!(n.round(4, RoundingMode::HalfEven),  dec256!(129.4168));
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn round(self, digits: i16, rounding_mode: RoundingMode) -> Self {
        self.with_scale(digits, Context::default().with_rounding_mode(rounding_mode))
    }

    /// Returns the result of rounding given decimal number
    /// to 'digits' precision after the decimal point using given
    /// [RoundingMode].
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::{dec256, decimal::{Context, RoundingMode}};
    ///
    /// let n = dec256!(129.41675);
    ///
    /// assert_eq!(n.with_scale(2, Context::default().with_rounding_mode(RoundingMode::Up)), dec256!(129.42));
    /// assert_eq!(n.with_scale(-1, Context::default().with_rounding_mode(RoundingMode::Down)), dec256!(120));
    /// assert_eq!(n.with_scale(4, Context::default().with_rounding_mode(RoundingMode::HalfEven)), dec256!(129.4168));
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn with_scale(self, new_scale: i16, ctx: Context) -> Self {
        scale::with_scale(self, new_scale, ctx).unwrap_signals(ctx)
        // match (rounding_mode, self.sign) {
        //     (RoundingMode::Floor, Sign::Minus) => signify_result(
        //         round::with_scale(self.value, new_scale, RoundingMode::Up),
        //         self.sign,
        //     ),
        //     (RoundingMode::Ceiling, Sign::Minus) => signify_result(
        //         round::with_scale(self.value, new_scale, RoundingMode::Down),
        //         self.sign,
        //     ),
        //     (_, _) => signify_result(
        //         round::with_scale(self.value, new_scale, rounding_mode),
        //         self.sign,
        //     ),
        // }
    }

    #[must_use = doc::must_use_op!()]
    #[inline]
    pub const fn ok(self) -> Option<Self> {
        if self.flags.is_special() || self.flags.has_signals() {
            None
        } else {
            Some(self)
        }
    }

    /// Create string of this decimal in scientific notation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// let n = dec256!(-12345678);
    /// assert_eq!(&n.to_scientific_notation(), "-1.2345678e7");
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub fn to_scientific_notation(&self) -> String {
        let mut output = String::new();
        self.write_scientific_notation(&mut output)
            .expect("Could not write to string");
        output
    }

    /// Create string of this decimal in engineering notation.
    ///
    /// Engineering notation is scientific notation with the exponent
    /// coerced to a multiple of three
    ///
    /// # Examples
    ///
    /// ```
    /// use fastnum::dec256;
    ///
    /// let n = dec256!(-12345678);
    /// assert_eq!(&n.to_engineering_notation(), "-12.345678e6");
    /// ```
    #[must_use = doc::must_use_op!()]
    #[inline]
    pub fn to_engineering_notation(&self) -> String {
        let mut output = String::new();
        self.write_engineering_notation(&mut output)
            .expect("Could not write to string");
        output
    }
}

#[doc(hidden)]
impl<const N: usize> Decimal<N> {
    #[inline]
    pub(crate) const fn new(digits: UInt<N>, scale: i16, flags: Flags) -> Self {
        Self {
            digits,
            scale,
            flags,
        }
    }

    #[inline]
    pub(crate) fn type_name() -> String {
        format!("D{}", N * 64)
    }

    #[inline]
    pub(crate) const fn flags(&self) -> Flags {
        self.flags
    }

    #[inline]
    pub(crate) const fn with_flags(mut self, flags: Flags) -> Self {
        self.flags = self.flags.combine(flags);
        self
    }

    #[inline]
    pub(crate) const fn with_signals_from(mut self, other: &Self) -> Self {
        self.flags = self.flags.with_signals_from(other.flags);
        self
    }

    #[inline]
    pub(crate) const fn with_signals_from_and(mut self, other: &Self, signal: Signal) -> Self {
        self.flags = self.flags.with_signals_from_and(other.flags, signal);
        self
    }

    #[inline]
    pub(crate) const fn raise_signal(mut self, signal: Signal) -> Self {
        self.flags = self.flags.raise_signal(signal);
        self
    }

    #[inline]
    pub(crate) const fn unwrap_signals(self, ctx: Context) -> Self {
        #[cfg(debug_assertions)]
        ctx.trap_signals(self.flags.signals());
        self
    }

    /// Write unsigned decimal in scientific notation to writer `w`.
    #[inline]
    pub(crate) fn write_scientific_notation<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
        if self.is_nan() {
            return w.write_str("NaN");
        }

        if self.is_sign_negative() {
            w.write_str("-")?;
        }

        if self.is_infinite() {
            return w.write_str("Inf");
        }

        if self.is_zero() {
            return w.write_str("0e0");
        }

        let digits = self.digits.to_str_radix(10);
        let scale = self.scale;
        format::write_scientific_notation(digits, scale, w)
    }

    /// Write unsigned decimal in engineering notation to writer `w`.
    pub(crate) fn write_engineering_notation<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
        if self.is_nan() {
            return w.write_str("NaN");
        }

        if self.is_sign_negative() {
            w.write_str("-")?;
        }

        if self.is_infinite() {
            return w.write_str("Inf");
        }

        if self.is_zero() {
            return w.write_str("0e0");
        }

        let digits = self.digits.to_str_radix(10);
        let scale = self.scale;
        format::write_engineering_notation(digits, scale, w)
    }
}
