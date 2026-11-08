// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! Retrieve ATNA Audit Event [ITI-81] (feature `retrieve`): the query an Audit
//! Consumer sends, and the `searchset` Bundle and the refusal an Audit Record
//! Repository answers with.
//!
//! ITI-81 is a FHIR R4 search on `AuditEvent` (`RESTful` ATNA supplement,
//! §3.81.4.1.2). [`Query::parse`] reads its query string: at least one `date`
//! parameter (§3.81.4.1.2.1), the parameters of §3.81.4.1.2.2, and the other
//! `AuditEvent` search parameters FHIR R4 defines, each typed by the
//! search-parameter registry of `fhir-types`. Repeated parameters combine with
//! AND and the values of one parameter with OR (§3.81.4.1.2.2); a parameter
//! the repository does not support is ignored and named in
//! [`Query::ignored`] (§3.81.4.1.3). The repository keeps its storage and the
//! query over it; this module decides nothing about which records a caller
//! may read.
//!
//! [`bundle::searchset`] and [`bundle::refusal`] write the response (§3.81.4.2.2) as
//! `fhir_types::r4` resources.
//!
//! # Examples
//!
//! ```
//! use ihe_atna::retrieve::{Query, Value};
//!
//! let query = Query::parse(
//!     "date=ge2013-01-05&date=le2013-01-05\
//!      &outcome=http://hl7.org/fhir/audit-event-outcome|4,8,12&_count=20",
//! )?;
//! assert_eq!(query.dates().len(), 2);
//! let outcome = query.criteria("outcome").next().expect("the outcome criterion");
//! assert_eq!(outcome.any_of().len(), 3);
//! assert!(matches!(outcome.any_of().first(), Some(Value::Token(_))));
//! assert_eq!(query.count(), Some(20));
//! # Ok::<(), ihe_atna::retrieve::QueryError>(())
//! ```

pub mod bundle;
pub mod date;

use std::borrow::Cow;

use fhir_types::r4::search_parameters::SEARCH_PARAMETERS;

use date::{DateError, DateParam};
use fhir_search::escape::{split, unescape};
use fhir_search::token::{Token, TokenError};
use fhir_types::search::{SearchParamType, SearchParameter};

/// The resource type ITI-81 searches.
const AUDIT_EVENT: &str = "AuditEvent";

/// The parameters §3.81.4.1.2.2 adds to the FHIR R4 `AuditEvent` search, each
/// of token type and with the element it matches.
const IHE_PARAMETERS: [(&str, &str); 4] = [
    ("agent.identifier", "AuditEvent.agent.who.identifier"),
    (
        "patient.identifier",
        "AuditEvent.agent.who.identifier | AuditEvent.entity.what.identifier",
    ),
    ("entity.identifier", "AuditEvent.entity.what.identifier"),
    ("source.identifier", "AuditEvent.source.observer.identifier"),
];

/// The FHIR R4 result parameter that bounds a page (FHIR R4 Search, §3.1.1.5.3).
const COUNT: &str = "_count";

/// The offset of a page; no specification governs it: our own design, since
/// FHIR R4 leaves the form of a paging link to the server
/// (<http://hl7.org/fhir/R4/http.html#paging>).
const OFFSET: &str = "_offset";

/// A parsed ITI-81 query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    dates: Vec<DateParam>,
    criteria: Vec<Criterion>,
    count: Option<u32>,
    offset: Option<u64>,
    ignored: Vec<String>,
}

/// One parameter of the query with the values it accepts, any of which
/// matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criterion {
    parameter: Parameter,
    any_of: Vec<Value>,
}

/// Where a supported parameter is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    /// A FHIR R4 `AuditEvent` search parameter, with its registry entry.
    Fhir(&'static SearchParameter),
    /// A parameter of §3.81.4.1.2.2 that FHIR R4 does not define, of token
    /// type.
    Ihe {
        /// The name the query uses.
        code: &'static str,
        /// The elements §3.81.4.1.2.2 matches it against.
        matches: &'static str,
    },
}

/// One value of a criterion, by the type of its parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A string parameter (FHIR R4 Search, §3.1.1.4.8); `address` matches it
    /// as a substring (§3.81.4.1.2.2).
    String(String),
    /// A token parameter (FHIR R4 Search, §3.1.1.4.10), read by the family's
    /// search grammar, whose `Debug` never shows the code.
    Token(Token),
    /// A reference parameter (FHIR R4 Search, §3.1.1.4.12): an id, a
    /// `[type]/[id]` or an absolute URL, as written.
    Reference(String),
    /// A uri parameter (FHIR R4 Search, §3.1.1.4.9).
    Uri(String),
}

/// Why a query string is refused.
///
/// No variant carries a parameter value, since a value can name a patient.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueryError {
    /// The query names no `date` parameter, which §3.81.4.1.2.1 requires.
    #[error("the query names no date parameter, at least one of which ITI-81 requires")]
    NoDate,
    /// A name or value is not percent-encoded UTF-8 (RFC 3986 §2.1).
    #[error("a parameter of the query is not percent-encoded UTF-8")]
    Encoding,
    /// A supported parameter carries a modifier the library does not
    /// support, which FHIR R4 Search §3.1.1.4.4 has a server reject.
    #[error("the parameter {parameter} carries the unsupported modifier :{modifier}")]
    Modifier {
        /// The parameter.
        parameter: String,
        /// The modifier, without its colon.
        modifier: String,
    },
    /// A value breaks the `\` escaping of FHIR R4 Search §3.1.1.4.19.
    #[error("a value of {parameter} breaks the escaping rules of FHIR search")]
    Escape {
        /// The parameter.
        parameter: String,
    },
    /// A value of a parameter is empty.
    #[error("a value of {parameter} is empty")]
    Empty {
        /// The parameter.
        parameter: String,
    },
    /// A `date` value is refused.
    #[error("a date value is refused")]
    Date(#[source] DateError),
    /// `_count` is not a non-negative integer.
    #[error("_count is not a non-negative integer")]
    Count,
    /// `_offset` is not a non-negative integer.
    #[error("_offset is not a non-negative integer")]
    Offset,
    /// `_count` or `_offset` is repeated.
    #[error("{parameter} is given more than once")]
    Repeated {
        /// The parameter.
        parameter: &'static str,
    },
}

impl Query {
    /// Parses the query string of an ITI-81 request, the part after `?`.
    ///
    /// Names and values are percent-decoded (RFC 3986 §2.1), so a `+` stays a
    /// plus, as a time zone offset needs. A parameter the library does not
    /// support, `_sort` and `_include` among them, is ignored and listed in
    /// [`Query::ignored`] (§3.81.4.1.3).
    ///
    /// # Errors
    ///
    /// A [`QueryError`] when no `date` parameter is present, a value cannot
    /// be read, or a supported parameter carries a modifier.
    pub fn parse(query: &str) -> Result<Self, QueryError> {
        let mut parsed = Self::default();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            let name = decode(name)?;
            let value = decode(value)?;
            parsed.add(&name, &value)?;
        }
        if parsed.dates.is_empty() {
            return Err(QueryError::NoDate);
        }
        Ok(parsed)
    }

    /// Adds one decoded parameter.
    fn add(&mut self, name: &str, value: &str) -> Result<(), QueryError> {
        let (code, modifier) = match name.split_once(':') {
            Some((code, modifier)) => (code, Some(modifier)),
            None => (name, None),
        };
        let parameter = match code {
            COUNT => {
                refuse_modifier(code, modifier)?;
                return set_once(&mut self.count, COUNT, value, QueryError::Count);
            }
            OFFSET => {
                refuse_modifier(code, modifier)?;
                return set_once(&mut self.offset, OFFSET, value, QueryError::Offset);
            }
            _ => supported(code),
        };
        let Some(parameter) = parameter else {
            self.ignored.push(name.to_owned());
            return Ok(());
        };
        refuse_modifier(code, modifier)?;
        let values = split(value, ',');
        if values.iter().any(String::is_empty) {
            return Err(QueryError::Empty {
                parameter: code.to_owned(),
            });
        }
        if let Parameter::Fhir(definition) = parameter
            && definition.r#type == SearchParamType::Date
        {
            for value in values {
                self.dates
                    .push(DateParam::parse(&value).map_err(|error| match error {
                        DateError::Escape => QueryError::Escape {
                            parameter: code.to_owned(),
                        },
                        other => QueryError::Date(other),
                    })?);
            }
            return Ok(());
        }
        let any_of = values
            .into_iter()
            .map(|text| typed(parameter, code, &text))
            .collect::<Result<Vec<_>, _>>()?;
        self.criteria.push(Criterion { parameter, any_of });
        Ok(())
    }

    /// The `date` parameters, every one of which a record's `recorded` must
    /// meet (§3.81.4.1.2.1).
    #[must_use]
    pub fn dates(&self) -> &[DateParam] {
        &self.dates
    }

    /// Every criterion other than `date`, every one of which a record must
    /// meet.
    #[must_use]
    pub fn all_criteria(&self) -> &[Criterion] {
        &self.criteria
    }

    /// The criteria of the parameter named `code`, every one of which a
    /// record must meet.
    pub fn criteria<'a>(&'a self, code: &'a str) -> impl Iterator<Item = &'a Criterion> + 'a {
        self.criteria
            .iter()
            .filter(move |c| c.parameter.code() == code)
    }

    /// `_count`, the largest page the caller accepts; `Some(0)` asks for the
    /// total alone (FHIR R4 Search, §3.1.1.5.3).
    #[must_use]
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    /// `_offset`, the number of matches before the page.
    #[must_use]
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// The parameters the query names that the library does not support, as
    /// written, which the repository ignores (§3.81.4.1.3).
    #[must_use]
    pub fn ignored(&self) -> &[String] {
        &self.ignored
    }
}

impl Criterion {
    /// Where the parameter is defined.
    #[must_use]
    pub fn parameter(&self) -> Parameter {
        self.parameter
    }

    /// The values, any one of which matches.
    #[must_use]
    pub fn any_of(&self) -> &[Value] {
        &self.any_of
    }
}

impl Parameter {
    /// The name the query uses.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Fhir(definition) => definition.code,
            Self::Ihe { code, .. } => code,
        }
    }
}

/// Returns the supported parameter named `code`.
fn supported(code: &str) -> Option<Parameter> {
    if let Some((code, matches)) = IHE_PARAMETERS.iter().find(|(name, _)| *name == code) {
        return Some(Parameter::Ihe { code, matches });
    }
    if code.starts_with('_') {
        return None;
    }
    SEARCH_PARAMETERS
        .of(AUDIT_EVENT, code)
        .filter(|definition| {
            matches!(
                definition.r#type,
                SearchParamType::Date
                    | SearchParamType::String
                    | SearchParamType::Token
                    | SearchParamType::Reference
                    | SearchParamType::Uri
            )
        })
        .map(Parameter::Fhir)
}

/// Refuses a modifier on a supported parameter.
fn refuse_modifier(code: &str, modifier: Option<&str>) -> Result<(), QueryError> {
    match modifier {
        Some(modifier) => Err(QueryError::Modifier {
            parameter: code.to_owned(),
            modifier: modifier.to_owned(),
        }),
        None => Ok(()),
    }
}

/// Sets a result parameter that may appear once.
fn set_once<T: std::str::FromStr>(
    slot: &mut Option<T>,
    parameter: &'static str,
    value: &str,
    refused: QueryError,
) -> Result<(), QueryError> {
    if slot.is_some() {
        return Err(QueryError::Repeated { parameter });
    }
    *slot = Some(value.parse().map_err(|_unparsable| refused)?);
    Ok(())
}

/// Percent-decodes `text` (RFC 3986 §2.1).
fn decode(text: &str) -> Result<Cow<'_, str>, QueryError> {
    percent_encoding::percent_decode_str(text)
        .decode_utf8()
        .map_err(|_not_utf8| QueryError::Encoding)
}

/// Types one value of `parameter`, as [`split`] gave it.
fn typed(parameter: Parameter, code: &str, text: &str) -> Result<Value, QueryError> {
    let kind = match parameter {
        Parameter::Fhir(definition) => definition.r#type,
        Parameter::Ihe { .. } => SearchParamType::Token,
    };
    let escape = || QueryError::Escape {
        parameter: code.to_owned(),
    };
    if kind == SearchParamType::Token {
        return Token::parse(text)
            .map(Value::Token)
            .map_err(|error| match error {
                TokenError::Empty => QueryError::Empty {
                    parameter: code.to_owned(),
                },
                _ => escape(),
            });
    }
    let plain = unescape(text).map_err(|_illegal| escape())?;
    Ok(match kind {
        SearchParamType::Reference => Value::Reference(plain),
        SearchParamType::Uri => Value::Uri(plain),
        _ => Value::String(plain),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_example_queries_of_the_supplement_parse() {
        for query in [
            "date=ge2013-01-05&date=le2013-01-05",
            "date=ge2013-01-01&date=le2013-01-02&address=192.168.0.1",
            "date=ge2013-01-01&date=le2013-01-02&agent.identifier=admin",
            "date=ge2013-01-01&date=le2013-01-02&patient.identifier=urn:oid:1.2.3.4|5678",
            "date=ge2013-01-01&date=le2013-01-02&type=http://dicom.nema.org/resources/ontology/DCM|110106",
            "date=ge2013-01-01&date=le2013-01-02&subtype=urn:ihe:event-type-code|ITI-43",
            "date=ge2013-01-01&date=le2013-01-02&outcome=http://hl7.org/fhir/audit-event-outcome|4,8,12",
            "date=ge2013-01-01&date=le2013-01-02&source=1234",
        ] {
            let parsed = Query::parse(query);
            assert!(parsed.is_ok(), "{query}: {parsed:?}");
        }
    }

    #[test]
    fn a_query_without_a_date_is_refused() {
        assert_eq!(Query::parse("address=10.0.0.1"), Err(QueryError::NoDate));
        assert_eq!(Query::parse(""), Err(QueryError::NoDate));
    }

    #[test]
    fn a_token_reads_each_of_its_four_forms() {
        let query = Query::parse("date=2013&type=a|b,|c,d|,e").expect("parses");
        let values = query
            .criteria("type")
            .next()
            .map(Criterion::any_of)
            .expect("type");
        assert_eq!(
            values,
            [
                Value::Token(Token::SystemCode {
                    system: "a".into(),
                    code: "b".into()
                }),
                Value::Token(Token::NoSystem("c".into())),
                Value::Token(Token::System("d".into())),
                Value::Token(Token::Code("e".into())),
            ]
        );
    }

    #[test]
    fn escapes_keep_separators_literal() {
        let query = Query::parse(r"date=2013&agent.identifier=a\,b\|c,d\\e").expect("parses");
        let values = query
            .criteria("agent.identifier")
            .next()
            .map(Criterion::any_of)
            .expect("agent.identifier");
        assert_eq!(
            values,
            [
                Value::Token(Token::Code("a,b|c".into())),
                Value::Token(Token::Code(r"d\e".into())),
            ]
        );
        assert!(matches!(
            Query::parse(r"date=2013&address=a\b"),
            Err(QueryError::Escape { .. })
        ));
        assert!(matches!(
            Query::parse("date=2013&type=a|b|c"),
            Err(QueryError::Escape { .. })
        ));
    }

    #[test]
    fn percent_decoding_keeps_a_plus() {
        let query = Query::parse("date=ge2013-01-05T10%3A00%3A00+01:00").expect("parses");
        assert_eq!(
            query
                .dates()
                .first()
                .map(|d| d.date().to_string())
                .as_deref(),
            Some("2013-01-05T10:00:00+01:00")
        );
        assert_eq!(
            Query::parse("date=2013&address=%FF"),
            Err(QueryError::Encoding)
        );
    }

    #[test]
    fn repeated_parameters_and_and_values_or() {
        let query =
            Query::parse("date=ge2013&date=le2014&entity-type=1,2&entity-type=3").expect("parses");
        assert_eq!(query.dates().len(), 2);
        let lengths: Vec<usize> = query
            .criteria("entity-type")
            .map(|c| c.any_of().len())
            .collect();
        assert_eq!(lengths, [2, 1]);
    }

    #[test]
    fn unsupported_parameters_are_ignored_and_named() {
        let query = Query::parse("date=2013&_sort=-date&_include=AuditEvent:agent&entity-id=x")
            .expect("parses");
        assert_eq!(
            query.ignored(),
            ["_sort", "_include", "entity-id"].map(String::from)
        );
        assert_eq!(query.all_criteria(), []);
    }

    #[test]
    fn a_modifier_on_a_supported_parameter_is_refused() {
        assert_eq!(
            Query::parse("date=2013&address:exact=x"),
            Err(QueryError::Modifier {
                parameter: "address".into(),
                modifier: "exact".into()
            })
        );
        assert!(Query::parse("date=2013&unknown:exact=x").is_ok());
    }

    #[test]
    fn count_and_offset_are_read_once() {
        let query = Query::parse("date=2013&_count=0&_offset=40").expect("parses");
        assert_eq!((query.count(), query.offset()), (Some(0), Some(40)));
        assert_eq!(Query::parse("date=2013&_count=-1"), Err(QueryError::Count));
        assert_eq!(
            Query::parse("date=2013&_count=1&_count=2"),
            Err(QueryError::Repeated {
                parameter: "_count"
            })
        );
    }

    #[test]
    fn each_parameter_takes_its_registry_type() {
        let query = Query::parse(
            "date=2013&address=10.0.0.1&agent=Practitioner/1&policy=http://p.example&action=R",
        )
        .expect("parses");
        let first = |code| {
            query
                .criteria(code)
                .next()
                .and_then(|c| c.any_of().first())
                .cloned()
        };
        assert_eq!(first("address"), Some(Value::String("10.0.0.1".into())));
        assert_eq!(
            first("agent"),
            Some(Value::Reference("Practitioner/1".into()))
        );
        assert_eq!(first("policy"), Some(Value::Uri("http://p.example".into())));
        assert!(matches!(first("action"), Some(Value::Token(_))));
    }

    #[test]
    fn an_empty_value_is_refused() {
        assert!(matches!(
            Query::parse("date=2013&type=a,,b"),
            Err(QueryError::Empty { .. })
        ));
        assert!(matches!(
            Query::parse("date=2013&type=|"),
            Err(QueryError::Empty { .. })
        ));
    }
}
