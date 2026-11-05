// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The escape every value an audit record carries goes through before it is
//! written, in the DICOM PS3.15 message and in the BALP `AuditEvent` alike.
//!
//! A record's values come from tokens, requests and relaying gateways, and a
//! reader of the record shows them: a control character can rewrite a
//! terminal or end a log line, a bidirectional control can reorder what a
//! reviewer reads, and XML 1.0 cannot carry most C0 controls at all. Each
//! such character is written as its code point, `\u{…}` in lower-case
//! hexadecimal, and every other character is kept. A value is never cut,
//! whatever its length.
//!
//! The characters escaped are the C0 and C1 controls and DEL (Unicode
//! general category `Cc`), the bidirectional formatting characters of
//! Unicode Standard Annex #9 §2 (U+061C, U+200E, U+200F, U+202A to U+202E,
//! U+2066 to U+2069), the line and paragraph separators U+2028 and U+2029,
//! the byte order mark U+FEFF, and the noncharacters U+FFFE and U+FFFF,
//! which XML 1.0 §2.2 excludes. No specification governs the escape form:
//! our own design. The escape is one-way: a reader cannot tell an escaped
//! character from the same six or more characters sent as text.

use std::borrow::Cow;

/// `value` with every character the module lists written as its code point.
///
/// Borrows `value` when it holds none of them.
#[must_use]
pub fn untrusted(value: &str) -> Cow<'_, str> {
    if !value.chars().any(escaped) {
        return Cow::Borrowed(value);
    }
    let mut written = String::with_capacity(value.len().saturating_add(16));
    for character in value.chars() {
        if escaped(character) {
            written.extend(character.escape_unicode());
        } else {
            written.push(character);
        }
    }
    Cow::Owned(written)
}

/// Whether `character` is one the module lists.
fn escaped(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{2066}'..='\u{2069}'
                | '\u{feff}'
                | '\u{fffe}'
                | '\u{ffff}'
        )
}

#[cfg(test)]
mod tests {
    use super::untrusted;

    #[test]
    fn a_value_with_nothing_to_escape_is_borrowed() {
        assert!(matches!(untrusted("plain"), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn an_escaped_character_is_its_lower_case_code_point() {
        assert_eq!(untrusted("a\u{202E}b\u{7}"), r"a\u{202e}b\u{7}");
    }
}
