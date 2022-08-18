//! The `@font-face` rule.

use super::Location;
use crate::error::{ParserError, PrinterError};
use crate::printer::Printer;
use crate::properties::custom::CustomProperty;
use crate::properties::font::{FontFamily, FontStretch, FontStyle, FontWeight};
use crate::traits::{Parse, ToCss};
use crate::values::size::Size2D;
use crate::values::string::CowArcStr;
use crate::values::url::Url;
use cssparser::*;
use std::fmt::Write;

/// A [@font-face](https://drafts.csswg.org/css-fonts/#font-face-rule) rule.
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontFaceRule<'i> {
  /// Declarations in the `@font-face` rule.
  #[cfg_attr(feature = "serde", serde(borrow))]
  pub properties: Vec<FontFaceProperty<'i>>,
  /// The location of the rule in the source file.
  pub loc: Location,
}

/// A property within an `@font-face` rule.
///
/// See [FontFaceRule](FontFaceRule).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
pub enum FontFaceProperty<'i> {
  /// The `src` property.
  #[cfg_attr(feature = "serde", serde(borrow))]
  Source(Vec<Source<'i>>),
  /// The `font-family` property.
  FontFamily(FontFamily<'i>),
  /// The `font-style` property.
  FontStyle(FontStyle),
  /// The `font-weight` property.
  FontWeight(Size2D<FontWeight>),
  /// The `font-stretch` property.
  FontStretch(Size2D<FontStretch>),
  /// The `unicode-range` property.
  UnicodeRange(Vec<UnicodeRange>),
  /// An unknown or unsupported property.
  Custom(CustomProperty<'i>),
}

/// A value for the [src](https://drafts.csswg.org/css-fonts/#src-desc)
/// property in an `@font-face` rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
pub enum Source<'i> {
  /// A `url()` with optional format metadata.
  Url(UrlSource<'i>),
  /// The `local()` function.
  #[cfg_attr(feature = "serde", serde(borrow))]
  Local(FontFamily<'i>),
}

impl<'i> Parse<'i> for Source<'i> {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    match input.try_parse(UrlSource::parse) {
      Ok(url) => return Ok(Source::Url(url)),
      a @ Err(ParseError {
        kind: ParseErrorKind::Basic(BasicParseErrorKind::AtRuleBodyInvalid),
        ..
      }) => {
        return Err(a.err().unwrap());
      }
      _ => {}
    }

    input.expect_function_matching("local")?;
    let local = input.parse_nested_block(FontFamily::parse)?;
    Ok(Source::Local(local))
  }
}

impl<'i> ToCss for Source<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      Source::Url(url) => url.to_css(dest),
      Source::Local(local) => {
        dest.write_str("local(")?;
        local.to_css(dest)?;
        dest.write_char(')')
      }
    }
  }
}

/// A `url()` value for the [src](https://drafts.csswg.org/css-fonts/#src-desc)
/// property in an `@font-face` rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UrlSource<'i> {
  /// The URL.
  pub url: Url<'i>,
  /// Optional `format()` function.
  #[cfg_attr(feature = "serde", serde(borrow))]
  pub format: Option<FontFormat<'i>>,
  /// Optional `tech()` function.
  pub tech: Vec<FontTechnology>,
}

impl<'i> Parse<'i> for UrlSource<'i> {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let url = Url::parse(input)?;

    let mut format_location: Option<SourceLocation> = None;
    let format = if input.try_parse(|input| input.expect_function_matching("format")).is_ok() {
      format_location = Some(input.current_source_location());
      Some(input.parse_nested_block(FontFormat::parse)?)
    } else {
      None
    };

    let tech = if input.try_parse(|input| input.expect_function_matching("tech")).is_ok() {
      if let Some(location) = format_location {
        // parser error
        let tech_location = input.current_source_location();
        if tech_location.line < location.line {
          return Err(ParseError {
            kind: ParseErrorKind::Basic(BasicParseErrorKind::AtRuleBodyInvalid),
            location: input.current_source_location(),
          });
        } else if tech_location.line == location.line && tech_location.column < location.column {
          return Err(ParseError {
            kind: ParseErrorKind::Basic(BasicParseErrorKind::AtRuleBodyInvalid),
            location: input.current_source_location(),
          });
        }
      }

      input.parse_nested_block(Vec::<FontTechnology>::parse)?
    } else {
      vec![]
    };

    Ok(UrlSource { url, format, tech })
  }
}

impl<'i> ToCss for UrlSource<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    self.url.to_css(dest)?;
    if let Some(format) = &self.format {
      dest.whitespace()?;
      dest.write_str("format(")?;
      format.to_css(dest)?;
      dest.write_char(')')?;
    }

    let tech_len = self.tech.len();
    if tech_len > 0 {
      dest.whitespace()?;
      dest.write_str("tech(")?;
      for i in 0..tech_len {
        self.tech[i].to_css(dest)?;
        if tech_len - 1 != i {
          dest.write_char(',')?;
        }
      }
      dest.write_char(')')?;
    }
    Ok(())
  }
}

/// A font format keyword in the `format()` function of the the
/// [src](https://drafts.csswg.org/css-fonts/#src-desc)
/// property of an `@font-face` rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
pub enum FontFormat<'i> {
  /// A WOFF font.
  WOFF,
  /// A WOFF v2 font.
  WOFF2,
  /// A TrueType font.
  TrueType,
  /// An OpenType font.
  OpenType,
  /// An Embedded OpenType (.eot) font.
  EmbeddedOpenType,
  /// A font collection.
  Collection,
  /// An SVG font.
  SVG,
  /// An unknown format.
  #[cfg_attr(feature = "serde", serde(borrow))]
  String(CowArcStr<'i>),
}

impl<'i> Parse<'i> for FontFormat<'i> {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let s = input.expect_ident_or_string()?;
    match_ignore_ascii_case! { &s,
      "woff" => Ok(FontFormat::WOFF),
      "woff2" => Ok(FontFormat::WOFF2),
      "truetype" => Ok(FontFormat::TrueType),
      "opentype" => Ok(FontFormat::OpenType),
      "embedded-opentype" => Ok(FontFormat::EmbeddedOpenType),
      "collection" => Ok(FontFormat::Collection),
      "svg" => Ok(FontFormat::SVG),
      _ => Ok(FontFormat::String(s.into()))
    }
  }
}

impl<'i> ToCss for FontFormat<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    use FontFormat::*;
    let s = match self {
      WOFF => "woff",
      WOFF2 => "woff2",
      TrueType => "truetype",
      OpenType => "opentype",
      EmbeddedOpenType => "embedded-opentype",
      Collection => "collection",
      SVG => "svg",
      String(s) => &s,
    };
    // Browser support for keywords rather than strings is very limited.
    // https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/src
    serialize_string(&s, dest)?;
    Ok(())
  }
}

/// A font format keyword in the `format()` function of the the
/// [src](https://drafts.csswg.org/css-fonts/#src-desc)
/// property of an `@font-face` rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
pub enum FontTechnology {
  /// A font feature tech descriptor in the `tech()`function of the
  /// [src](https://drafts.csswg.org/css-fonts/#src-desc)
  /// property of an `@font-face` rule.
  /// support FeaturesOpentype
  FeaturesOpentype,
  /// support FeaturesAat
  FeaturesAat,
  /// support FeaturesGraphite
  FeaturesGraphite,

  /// A color font tech descriptor in the `tech()`function of the
  /// [src](https://drafts.csswg.org/css-fonts/#src-desc)
  /// property of an `@font-face` rule.
  /// support ColorColrv0
  ColorColrv0,
  /// support ColorColrv1
  ColorColrv1,
  /// support ColorSvg
  ColorSvg,
  /// support ColorSbix
  ColorSbix,
  /// support ColorCbdt
  ColorCbdt,

  /// support Variations
  Variations,
  /// support Palettes
  Palettes,
  /// support Incremental
  Incremental,
}

impl<'i> Parse<'i> for FontTechnology {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let location = input.current_source_location();
    match input.next()? {
      Token::Ident(ident) => {
        match_ignore_ascii_case! { &ident,
          "variations" => Ok(FontTechnology::Variations),
          "palettes" => Ok(FontTechnology::Palettes),
          "incremental" => Ok(FontTechnology::Incremental),
          "features-opentype" => Ok(FontTechnology::FeaturesOpentype),
          "features-aat" => Ok(FontTechnology::FeaturesAat),
          "features-graphite" => Ok(FontTechnology::FeaturesGraphite),
          "color-colrv0" => Ok(FontTechnology::ColorColrv0),
          "color-colrv1" => Ok(FontTechnology::ColorColrv1),
          "color-svg" => Ok(FontTechnology::ColorSvg),
          "color-sbix" => Ok(FontTechnology::ColorSbix),
          "color-cbdt" => Ok(FontTechnology::ColorCbdt),
          _ => Err(location.new_unexpected_token_error(
            cssparser::Token::Ident(ident.clone())
          ))
        }
      }
      tok => Err(location.new_unexpected_token_error(tok.clone())),
    }
  }
}

impl ToCss for FontTechnology {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str(match self {
      FontTechnology::FeaturesOpentype => "features-opentype",
      FontTechnology::FeaturesAat => "features-aat",
      FontTechnology::FeaturesGraphite => "features-graphite",
      FontTechnology::ColorColrv0 => "color-colrv0",
      FontTechnology::ColorColrv1 => "color-colrv1",
      FontTechnology::ColorSvg => "color-svg",
      FontTechnology::ColorSbix => "color-sbix",
      FontTechnology::ColorCbdt => "color-cbdt",
      FontTechnology::Variations => "variations",
      FontTechnology::Palettes => "palettes",
      FontTechnology::Incremental => "incremental",
    })
  }
}

/// A contiguous range of Unicode code points.
///
/// Cannot be empty. Can represent a single code point when start == end.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnicodeRange {
  /// Inclusive start of the range. In [0, end].
  pub start: u32,
  /// Inclusive end of the range. In [0, 0x10FFFF].
  pub end: u32,
}

impl<'i> Parse<'i> for UnicodeRange {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let range = cssparser::UnicodeRange::parse(input)?;
    Ok(UnicodeRange {
      start: range.start,
      end: range.end,
    })
  }
}

impl ToCss for UnicodeRange {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    // Attempt to optimize the range to use question mark syntax.
    if self.start != self.end {
      // Find the first hex digit that differs between the start and end values.
      let mut shift = 24;
      let mut mask = 0xf << shift;
      while shift > 0 {
        let c1 = self.start & mask;
        let c2 = self.end & mask;
        if c1 != c2 {
          break;
        }

        mask = mask >> 4;
        shift -= 4;
      }

      // Get the remainder of the value. This must be 0x0 to 0xf for the rest
      // of the value to use the question mark syntax.
      shift += 4;
      let remainder_mask = (1 << shift) - 1;
      let start_remainder = self.start & remainder_mask;
      let end_remainder = self.end & remainder_mask;

      if start_remainder == 0 && end_remainder == remainder_mask {
        let start = (self.start & !remainder_mask) >> shift;
        if start != 0 {
          write!(dest, "U+{:X}", start)?;
        } else {
          dest.write_str("U+")?;
        }

        while shift > 0 {
          dest.write_char('?')?;
          shift -= 4;
        }

        return Ok(());
      }
    }

    write!(dest, "U+{:X}", self.start)?;
    if self.end != self.start {
      write!(dest, "-{:X}", self.end)?;
    }
    Ok(())
  }
}

pub(crate) struct FontFaceDeclarationParser;

/// Parse a declaration within {} block: `color: blue`
impl<'i> cssparser::DeclarationParser<'i> for FontFaceDeclarationParser {
  type Declaration = FontFaceProperty<'i>;
  type Error = ParserError<'i>;

  fn parse_value<'t>(
    &mut self,
    name: CowRcStr<'i>,
    input: &mut cssparser::Parser<'i, 't>,
  ) -> Result<Self::Declaration, cssparser::ParseError<'i, Self::Error>> {
    macro_rules! property {
      ($property: ident, $type: ty) => {
        if let Ok(c) = <$type>::parse(input) {
          return Ok(FontFaceProperty::$property(c));
        }
      };
    }

    let state = input.state();
    match_ignore_ascii_case! { &name,
      "src" => {
        if let Ok(sources) = input.parse_comma_separated(Source::parse) {
          return Ok(FontFaceProperty::Source(sources))
        }
      },
      "font-family" => property!(FontFamily, FontFamily),
      "font-weight" => property!(FontWeight, Size2D<FontWeight>),
      "font-style" => property!(FontStyle, FontStyle),
      "font-stretch" => property!(FontStretch, Size2D<FontStretch>),
      "unicode-range" => property!(UnicodeRange, Vec<UnicodeRange>),
      _ => {}
    }

    input.reset(&state);
    return Ok(FontFaceProperty::Custom(CustomProperty::parse(
      name.into(),
      input,
      &Default::default(),
    )?));
  }
}

/// Default methods reject all at rules.
impl<'i> AtRuleParser<'i> for FontFaceDeclarationParser {
  type Prelude = ();
  type AtRule = FontFaceProperty<'i>;
  type Error = ParserError<'i>;
}

impl<'i> ToCss for FontFaceRule<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.add_mapping(self.loc);
    dest.write_str("@font-face")?;
    dest.whitespace()?;
    dest.write_char('{')?;
    dest.indent();
    let len = self.properties.len();
    for (i, prop) in self.properties.iter().enumerate() {
      dest.newline()?;
      prop.to_css(dest)?;
      if i != len - 1 || !dest.minify {
        dest.write_char(';')?;
      }
    }
    dest.dedent();
    dest.newline()?;
    dest.write_char('}')
  }
}

impl<'i> ToCss for FontFaceProperty<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    use FontFaceProperty::*;
    macro_rules! property {
      ($prop: literal, $value: expr) => {{
        dest.write_str($prop)?;
        dest.delim(':', false)?;
        $value.to_css(dest)
      }};
      ($prop: literal, $value: expr, $multi: expr) => {{
        dest.write_str($prop)?;
        dest.delim(':', false)?;
        let len = $value.len();
        for (idx, val) in $value.iter().enumerate() {
          val.to_css(dest)?;
          if idx < len - 1 {
            dest.delim(',', false)?;
          }
        }
        Ok(())
      }};
    }

    match self {
      Source(value) => property!("src", value, true),
      FontFamily(value) => property!("font-family", value),
      FontStyle(value) => property!("font-style", value),
      FontWeight(value) => property!("font-weight", value),
      FontStretch(value) => property!("font-stretch", value),
      UnicodeRange(value) => property!("unicode-range", value),
      Custom(custom) => {
        dest.write_str(custom.name.as_ref())?;
        dest.delim(':', false)?;
        custom.value.to_css(dest, true)
      }
    }
  }
}
