// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The `date` parameter of ITI-81 (`RESTful` ATNA supplement, §3.81.4.1.2.1):
//! a prefix and a FHIR date value, and the interval of `recorded` instants it
//! matches.
//!
//! The value is any of the FHIR `date`, `dateTime` and `instant` forms, to
//! any precision from the year, and the time may leave out its seconds (FHIR
//! R4 Search, §3.1.1.4.7). It is held as a `fhir_types::r4` `dateTime`,
//! checked against that type's lexical form. `recorded` is an instant, a
//! point in time (§3.1.1.4.7), so each prefix of §3.1.1.4.5 becomes an
//! interval around the range the value's precision implies.

use fhir_types::r4::primitives::DateTime;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp, ToSpan};

/// A comparison prefix (FHIR R4 Search, §3.1.1.4.5); `eq` when none is
/// written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    /// `eq`: within the range of the value.
    Eq,
    /// `ne`: outside the range of the value.
    Ne,
    /// `gt`: after the range of the value.
    Gt,
    /// `lt`: before the range of the value.
    Lt,
    /// `ge`: within the range of the value or after it.
    Ge,
    /// `le`: within the range of the value or before it.
    Le,
    /// `sa`: starts after the range of the value.
    Sa,
    /// `eb`: ends before the range of the value.
    Eb,
}

/// One `date` parameter value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateParam {
    prefix: Prefix,
    value: DateTime,
}

/// The `recorded` instants a [`DateParam`] matches.
///
/// Every `from` is inclusive and every `until` exclusive; `None` is
/// unbounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// `from <= recorded < until`.
    Within {
        /// The first instant that matches.
        from: Option<Timestamp>,
        /// The first instant after the match.
        until: Option<Timestamp>,
    },
    /// `recorded < from` or `recorded >= until`.
    Outside {
        /// The first instant that does not match.
        from: Timestamp,
        /// The first instant after `from` that matches again.
        until: Timestamp,
    },
}

/// Why a `date` value is refused.
///
/// No variant carries the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DateError {
    /// The value starts with letters that are no prefix of §3.1.1.4.5.
    #[error("the date value starts with an unknown prefix")]
    Prefix,
    /// The `ap` prefix, whose approximation FHIR leaves to the server; the
    /// library does not choose one.
    #[error("the ap prefix is not supported")]
    Approximate,
    /// The value is not a FHIR date, dateTime or instant.
    #[error("the date value is not a FHIR date, dateTime or instant")]
    Form,
    /// The value has the FHIR form but names no instant the library can
    /// represent, such as a leap second.
    #[error("the date value names no representable instant")]
    Unrepresentable,
}

impl DateParam {
    /// Reads one decoded `date` value, its prefix included.
    ///
    /// # Errors
    ///
    /// A [`DateError`] for an unknown or `ap` prefix, or a value outside the
    /// FHIR date forms.
    pub fn parse(text: &str) -> Result<Self, DateError> {
        let (prefix, rest) = prefix(text)?;
        let value = complete_seconds(rest);
        DateTime::check(&value).map_err(|_outside| DateError::Form)?;
        Ok(Self {
            prefix,
            value: DateTime::from(value),
        })
    }

    /// The prefix.
    #[must_use]
    pub fn prefix(&self) -> Prefix {
        self.prefix
    }

    /// The value, as a FHIR R4 `dateTime`, with zero seconds written where
    /// the query left them out.
    #[must_use]
    pub fn value(&self) -> &DateTime {
        &self.value
    }

    /// The `recorded` instants this parameter matches.
    ///
    /// A value without a time is read in `zone`, the repository's local time
    /// zone, which FHIR R4 Search §3.1.1.4.7 has a server assume where the
    /// value carries none. The range of the value is the year, month, day or
    /// second it names, narrowed by each fractional digit.
    ///
    /// # Errors
    ///
    /// [`DateError::Unrepresentable`] when the value or the end of its range
    /// is no instant the library can represent.
    pub fn bounds(&self, zone: &TimeZone) -> Result<Bound, DateError> {
        let text = self.value.value.as_deref().ok_or(DateError::Form)?;
        let (from, until) = range(text, zone)?;
        Ok(match self.prefix {
            Prefix::Eq => Bound::Within {
                from: Some(from),
                until: Some(until),
            },
            Prefix::Ne => Bound::Outside { from, until },
            Prefix::Gt | Prefix::Sa => Bound::Within {
                from: Some(until),
                until: None,
            },
            Prefix::Lt | Prefix::Eb => Bound::Within {
                from: None,
                until: Some(from),
            },
            Prefix::Ge => Bound::Within {
                from: Some(from),
                until: None,
            },
            Prefix::Le => Bound::Within {
                from: None,
                until: Some(until),
            },
        })
    }
}

/// Splits the prefix off `text`.
fn prefix(text: &str) -> Result<(Prefix, &str), DateError> {
    let letters = text.chars().take_while(char::is_ascii_alphabetic).count();
    if letters == 0 {
        return Ok((Prefix::Eq, text));
    }
    let (word, rest) = text.split_at_checked(letters).ok_or(DateError::Prefix)?;
    let prefix = match word {
        "eq" => Prefix::Eq,
        "ne" => Prefix::Ne,
        "gt" => Prefix::Gt,
        "lt" => Prefix::Lt,
        "ge" => Prefix::Ge,
        "le" => Prefix::Le,
        "sa" => Prefix::Sa,
        "eb" => Prefix::Eb,
        "ap" => return Err(DateError::Approximate),
        _ => return Err(DateError::Prefix),
    };
    Ok((prefix, rest))
}

/// Writes `:00` seconds into a time of hours and minutes, which a search
/// value may carry and the `dateTime` form does not (FHIR R4 Search,
/// §3.1.1.4.7).
fn complete_seconds(text: &str) -> String {
    let Some((date, time)) = text.split_once('T') else {
        return text.to_owned();
    };
    let zone_at = time.find(['Z', '+', '-']).unwrap_or(time.len());
    let (clock, zone) = time.split_at(zone_at);
    if clock.len() == 5 && clock.as_bytes().get(2) == Some(&b':') {
        format!("{date}T{clock}:00{zone}")
    } else {
        text.to_owned()
    }
}

/// The range `[from, until)` the precision of `text`, a checked `dateTime`,
/// implies.
fn range(text: &str, zone: &TimeZone) -> Result<(Timestamp, Timestamp), DateError> {
    if text.contains('T') {
        let from: Timestamp = text.parse().map_err(|_leap| DateError::Unrepresentable)?;
        let width = fraction_width(text);
        let until = from
            .checked_add(width)
            .map_err(|_overflow| DateError::Unrepresentable)?;
        return Ok((from, until));
    }
    let mut parts = text.split('-');
    let year = number(parts.next())?;
    let (start, step): (Date, Span) = match (parts.next(), parts.next()) {
        (None, _) => (civil(year, 1, 1)?, 1.year()),
        (Some(month), None) => (civil(year, number(Some(month))?, 1)?, 1.month()),
        (Some(month), Some(day)) => (
            civil(year, number(Some(month))?, number(Some(day))?)?,
            1.day(),
        ),
    };
    let end = start
        .checked_add(step)
        .map_err(|_overflow| DateError::Unrepresentable)?;
    Ok((instant(start, zone)?, instant(end, zone)?))
}

/// The width of the range of a time with `text`'s fractional digits: one
/// second without any, a tenth for one, down to the nanosecond.
fn fraction_width(text: &str) -> Span {
    let digits = text.split_once('.').map_or(0, |(_, rest)| {
        rest.chars().take_while(char::is_ascii_digit).count()
    });
    let nanos: i64 = match digits {
        0 => return 1.second(),
        1 => 100_000_000,
        2 => 10_000_000,
        3 => 1_000_000,
        4 => 100_000,
        5 => 10_000,
        6 => 1_000,
        7 => 100,
        8 => 10,
        _ => 1,
    };
    nanos.nanoseconds()
}

/// Reads one numeric part of a checked date.
fn number<T: std::str::FromStr>(part: Option<&str>) -> Result<T, DateError> {
    part.and_then(|text| text.parse().ok())
        .ok_or(DateError::Form)
}

/// The civil date `year-month-day`.
fn civil(year: i16, month: i8, day: i8) -> Result<Date, DateError> {
    Date::new(year, month, day).map_err(|_invalid| DateError::Form)
}

/// The first instant of `date` in `zone`.
fn instant(date: Date, zone: &TimeZone) -> Result<Timestamp, DateError> {
    date.to_zoned(zone.clone())
        .map(|zoned| zoned.timestamp())
        .map_err(|_outside| DateError::Unrepresentable)
}

#[cfg(test)]
mod tests {
    use jiff::tz::{Offset, TimeZone};

    use super::*;

    fn at(text: &str) -> Timestamp {
        text.parse().expect("a test instant")
    }

    fn bounds(text: &str) -> Bound {
        DateParam::parse(text)
            .and_then(|param| param.bounds(&TimeZone::UTC))
            .expect("a supported value")
    }

    #[test]
    fn a_day_is_its_whole_day() {
        assert_eq!(
            bounds("2013-01-05"),
            Bound::Within {
                from: Some(at("2013-01-05T00:00:00Z")),
                until: Some(at("2013-01-06T00:00:00Z"))
            }
        );
        assert_eq!(
            bounds("le2013-01-05"),
            Bound::Within {
                from: None,
                until: Some(at("2013-01-06T00:00:00Z"))
            }
        );
        assert_eq!(
            bounds("ge2013-01-05"),
            Bound::Within {
                from: Some(at("2013-01-05T00:00:00Z")),
                until: None
            }
        );
    }

    #[test]
    fn a_year_and_a_month_are_their_intervals() {
        assert_eq!(
            bounds("2000"),
            Bound::Within {
                from: Some(at("2000-01-01T00:00:00Z")),
                until: Some(at("2001-01-01T00:00:00Z"))
            }
        );
        assert_eq!(
            bounds("2000-02"),
            Bound::Within {
                from: Some(at("2000-02-01T00:00:00Z")),
                until: Some(at("2000-03-01T00:00:00Z"))
            }
        );
    }

    #[test]
    fn each_prefix_takes_its_side_of_the_range() {
        let from = at("2013-01-14T10:00:00Z");
        let until = at("2013-01-14T10:00:01Z");
        assert_eq!(
            bounds("ne2013-01-14T10:00:00Z"),
            Bound::Outside { from, until }
        );
        for text in ["gt2013-01-14T10:00Z", "sa2013-01-14T10:00Z"] {
            assert_eq!(
                bounds(text),
                Bound::Within {
                    from: Some(until),
                    until: None
                },
                "{text}"
            );
        }
        for text in ["lt2013-01-14T10:00Z", "eb2013-01-14T10:00Z"] {
            assert_eq!(
                bounds(text),
                Bound::Within {
                    from: None,
                    until: Some(from)
                },
                "{text}"
            );
        }
    }

    #[test]
    fn a_time_without_seconds_reads_as_zero_seconds() {
        let param = DateParam::parse("ge2013-01-05T10:00+01:00").expect("hours and minutes");
        assert_eq!(
            param.value().value.as_deref(),
            Some("2013-01-05T10:00:00+01:00")
        );
        assert_eq!(
            param.bounds(&TimeZone::UTC),
            Ok(Bound::Within {
                from: Some(at("2013-01-05T09:00:00Z")),
                until: None
            })
        );
    }

    #[test]
    fn fractional_digits_narrow_the_range() {
        assert_eq!(
            bounds("2013-01-05T10:00:00.5Z"),
            Bound::Within {
                from: Some(at("2013-01-05T10:00:00.5Z")),
                until: Some(at("2013-01-05T10:00:00.6Z"))
            }
        );
    }

    #[test]
    fn a_date_without_a_zone_reads_in_the_repository_zone() {
        let zone = TimeZone::fixed(Offset::constant(2));
        let param = DateParam::parse("2013-01-05").expect("a day");
        assert_eq!(
            param.bounds(&zone),
            Ok(Bound::Within {
                from: Some(at("2013-01-04T22:00:00Z")),
                until: Some(at("2013-01-05T22:00:00Z"))
            })
        );
    }

    #[test]
    fn values_outside_the_forms_are_refused() {
        for (text, error) in [
            ("ap2013", DateError::Approximate),
            ("xx2013", DateError::Prefix),
            ("2013-13", DateError::Form),
            ("2013-01-05T10", DateError::Form),
            ("2013-01-05T10:00:00", DateError::Form),
            ("13-01-05", DateError::Form),
            ("", DateError::Form),
        ] {
            assert_eq!(DateParam::parse(text), Err(error), "{text:?}");
        }
    }
}
