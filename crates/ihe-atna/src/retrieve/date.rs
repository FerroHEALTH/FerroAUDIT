// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The `date` parameter of ITI-81 (`RESTful` ATNA supplement, §3.81.4.1.2.1):
//! a prefix and a FHIR date value, and the interval of `recorded` instants it
//! matches.
//!
//! The value is read by the family's FHIR search grammar, `fhir-search`: any
//! of the FHIR `date`, `dateTime` and `instant` forms, to any precision from
//! the year, the time with or without its seconds and offset (FHIR R4
//! Search, §3.1.1.4.7). `recorded` is an instant, a point in time
//! (§3.1.1.4.7), so each prefix of §3.1.1.4.5 becomes an interval around the
//! range the value's precision implies.

use fhir_search::date::{DateError as GrammarError, DateValue, SearchDate};
use fhir_search::prefix::Prefix;
use jiff::Timestamp;
use jiff::tz::TimeZone;

/// One `date` parameter value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateParam {
    value: DateValue,
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
    /// No instant: the value's range runs past the last instant, and the
    /// prefix asks for what lies after it.
    Empty,
}

/// Why a `date` value is refused.
///
/// No variant carries the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DateError {
    /// The value breaks the `\` escaping of FHIR R4 Search §3.1.1.4.19.
    #[error("the date value breaks the escaping rules of FHIR search")]
    Escape,
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

impl From<GrammarError> for DateError {
    fn from(error: GrammarError) -> Self {
        match error {
            GrammarError::Escape(_) => Self::Escape,
            GrammarError::Prefix(_) => Self::Prefix,
            GrammarError::LeapSecond | GrammarError::Unrepresentable => Self::Unrepresentable,
            _ => Self::Form,
        }
    }
}

impl DateParam {
    /// Reads one percent-decoded `date` value, its prefix included, with its
    /// `\` escapes in place.
    ///
    /// # Errors
    ///
    /// A [`DateError`] for an illegal escape, an unknown or `ap` prefix, or a
    /// value outside the FHIR date forms.
    pub fn parse(text: &str) -> Result<Self, DateError> {
        let value = DateValue::parse(text)?;
        if value.prefix() == Prefix::Ap {
            return Err(DateError::Approximate);
        }
        Ok(Self { value })
    }

    /// The prefix, never `ap`.
    #[must_use]
    pub fn prefix(&self) -> Prefix {
        self.value.prefix()
    }

    /// The date, written back as the query wrote it.
    #[must_use]
    pub fn date(&self) -> &SearchDate {
        self.value.date()
    }

    /// The `recorded` instants this parameter matches.
    ///
    /// A value without an offset is read in `zone`, the repository's local
    /// time zone, which FHIR R4 Search §3.1.1.4.7 has a server assume where
    /// the value carries none. The range of the value is the year, month,
    /// day, minute or second it names, narrowed by each fractional digit.
    ///
    /// # Errors
    ///
    /// [`DateError::Unrepresentable`] when the value starts outside the
    /// instants the library can represent.
    pub fn bounds(&self, zone: &TimeZone) -> Result<Bound, DateError> {
        let instants = self.value.date().instants(zone)?;
        let from = instants.from();
        Ok(match (self.value.prefix(), instants.until()) {
            // NOTE: FHIR R4 Search §3.1.1.4.5 leaves `ap` to the server, and `parse`
            // refuses it, so no parameter holds it.
            (Prefix::Eq | Prefix::Ap, until) => Bound::Within {
                from: Some(from),
                until,
            },
            (Prefix::Ne, Some(until)) => Bound::Outside { from, until },
            (Prefix::Ne | Prefix::Lt | Prefix::Eb, _) => Bound::Within {
                from: None,
                until: Some(from),
            },
            (Prefix::Gt | Prefix::Sa, Some(until)) => Bound::Within {
                from: Some(until),
                until: None,
            },
            (Prefix::Gt | Prefix::Sa, None) => Bound::Empty,
            (Prefix::Ge, _) => Bound::Within {
                from: Some(from),
                until: None,
            },
            (Prefix::Le, until) => Bound::Within { from: None, until },
        })
    }
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
        let minute_end = at("2013-01-14T10:01:00Z");
        for text in ["gt2013-01-14T10:00Z", "sa2013-01-14T10:00Z"] {
            assert_eq!(
                bounds(text),
                Bound::Within {
                    from: Some(minute_end),
                    until: None
                },
                "{text}: a minute's range ends with the minute (§3.1.1.4.5)"
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
    fn a_time_without_seconds_spans_its_minute() {
        let param = DateParam::parse("ge2013-01-05T10:00+01:00").expect("hours and minutes");
        assert_eq!(param.date().to_string(), "2013-01-05T10:00+01:00");
        assert_eq!(
            bounds("2013-01-05T10:00+01:00"),
            Bound::Within {
                from: Some(at("2013-01-05T09:00:00Z")),
                until: Some(at("2013-01-05T09:01:00Z"))
            }
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
            ("2013-01-05T10:00:00\\x", DateError::Escape),
            ("2016-12-31T23:59:60Z", DateError::Unrepresentable),
            ("13-01-05", DateError::Form),
            ("", DateError::Form),
        ] {
            assert_eq!(DateParam::parse(text), Err(error), "{text:?}");
        }
    }

    #[test]
    fn a_time_without_an_offset_reads_in_the_repository_zone() {
        let zone = TimeZone::fixed(Offset::constant(2));
        let param = DateParam::parse("2013-01-05T10:00:00").expect("a zoneless time");
        assert_eq!(
            param.bounds(&zone),
            Ok(Bound::Within {
                from: Some(at("2013-01-05T08:00:00Z")),
                until: Some(at("2013-01-05T08:00:01Z"))
            })
        );
    }

    #[test]
    fn a_range_open_past_the_last_instant_matches_nothing_after_it() {
        assert_eq!(
            bounds("9999"),
            Bound::Within {
                from: Some(at("9999-01-01T00:00:00Z")),
                until: None
            }
        );
        assert_eq!(bounds("gt9999"), Bound::Empty);
        assert_eq!(
            bounds("ne9999"),
            Bound::Within {
                from: None,
                until: Some(at("9999-01-01T00:00:00Z"))
            }
        );
        assert_eq!(
            bounds("le9999"),
            Bound::Within {
                from: None,
                until: None
            }
        );
    }
}
