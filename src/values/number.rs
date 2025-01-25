//! CSS number values.

use super::angle::impl_try_from_angle;
use super::calc::Calc;
use crate::error::{ParserError, PrinterError};
use crate::printer::Printer;
use crate::traits::private::AddInternal;
use crate::traits::{Map, Op, Parse, Sign, ToCss, Zero};
use cssparser::*;

/// A CSS [`<number>`](https://www.w3.org/TR/css-values-4/#numbers) value.
///
/// Numbers may be explicit or computed by `calc()`, but are always stored and serialized
/// as their computed value.
pub type CSSNumber = f32;

impl<'i> Parse<'i> for CSSNumber {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    match input.try_parse(Calc::parse) {
      Ok(Calc::Value(v)) => return Ok(*v),
      Ok(Calc::Number(n)) => return Ok(n),
      // Numbers are always compatible, so they will always compute to a value.
      Ok(_) => return Err(input.new_custom_error(ParserError::InvalidValue)),
      _ => {}
    }

    let number = input.expect_number()?;
    Ok(number)
  }
}

impl ToCss for CSSNumber {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    let number = *self;
    if number != 0.0 && number.abs() < 1.0 {
      let mut s = String::new();
      cssparser::ToCss::to_css(self, &mut s)?;
      if number < 0.0 {
        dest.write_char('-')?;
        dest.write_str(s.trim_start_matches("-").trim_start_matches("0"))
      } else {
        dest.write_str(s.trim_start_matches('0'))
      }
    } else {
      cssparser::ToCss::to_css(self, dest)?;
      Ok(())
    }
  }
}

impl std::convert::Into<Calc<CSSNumber>> for CSSNumber {
  fn into(self) -> Calc<CSSNumber> {
    Calc::Value(Box::new(self))
  }
}

impl std::convert::From<Calc<CSSNumber>> for CSSNumber {
  fn from(calc: Calc<CSSNumber>) -> CSSNumber {
    match calc {
      Calc::Value(v) => *v,
      Calc::Number(n) => n,
      _ => unreachable!(),
    }
  }
}

impl AddInternal for CSSNumber {
  fn add(self, other: Self) -> Self {
    self + other
  }
}

impl Op for CSSNumber {
  fn op<F: FnOnce(f32, f32) -> f32>(&self, to: &Self, op: F) -> Self {
    op(*self, *to)
  }

  fn op_to<T, F: FnOnce(f32, f32) -> T>(&self, rhs: &Self, op: F) -> T {
    op(*self, *rhs)
  }
}

impl Map for CSSNumber {
  fn map<F: FnOnce(f32) -> f32>(&self, op: F) -> Self {
    op(*self)
  }
}

impl Sign for CSSNumber {
  fn sign(&self) -> f32 {
    if *self == 0.0 {
      return if f32::is_sign_positive(*self) { 0.0 } else { -0.0 };
    }
    self.signum()
  }
}

impl Zero for CSSNumber {
  fn zero() -> Self {
    0.0
  }

  fn is_zero(&self) -> bool {
    *self == 0.0
  }
}

impl_try_from_angle!(CSSNumber);

/// A CSS [`<integer>`](https://www.w3.org/TR/css-values-4/#integers) value.
pub type CSSInteger = i32;

impl<'i> Parse<'i> for CSSInteger {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    // TODO: calc??
    let integer = input.expect_integer()?;
    Ok(integer)
  }
}

impl ToCss for CSSInteger {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    cssparser::ToCss::to_css(self, dest)?;
    Ok(())
  }
}

impl Zero for CSSInteger {
  fn zero() -> Self {
    0
  }

  fn is_zero(&self) -> bool {
    *self == 0
  }
}
