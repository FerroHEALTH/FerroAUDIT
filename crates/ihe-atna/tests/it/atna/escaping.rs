// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! A value the audit message carries comes from a token, a request or a
//! relaying gateway, so the writer escapes every one: no control character,
//! no bidirectional control and no line separator reaches the XML or the
//! syslog frame, and a value of any length is written whole (no
//! specification governs the escape form: our own design).

use ihe_atna::atna::codes;
use ihe_atna::atna::message::{
    ActiveParticipant, AuditMessage, AuditSource, CodedValue, EventIdentification,
    ParticipantObject,
};
use ihe_atna::atna::syslog::Sender;
use ihe_atna::escape;
use secrecy::{ExposeSecret as _, SecretString};

/// A value with a C0 control, an escape sequence, a right-to-left override,
/// a line separator and a C1 control, each of which a log viewer would act
/// on.
const HOSTILE: &str = "Qz7-a\u{0}b\u{1b}[31mc\u{202e}d\u{2028}e\u{85}f";

/// How [`HOSTILE`] is written.
const ESCAPED: &str = r"Qz7-a\u{0}b\u{1b}[31mc\u{202e}d\u{2028}e\u{85}f";

fn message(value: &str) -> AuditMessage {
    AuditMessage {
        event: EventIdentification {
            event_id: CodedValue::new(value, value, value),
            action: "R",
            date_time: jiff::Timestamp::UNIX_EPOCH,
            outcome: "0",
            type_codes: vec![CodedValue::new(value, "DCM", value)],
            outcome_description: Some(value.to_owned()),
            purposes_of_use: vec![CodedValue::new(value, value, value)],
        },
        participants: vec![ActiveParticipant {
            user_id: value.to_owned(),
            alternative_user_id: Some(value.to_owned()),
            user_name: Some(value.to_owned()),
            user_is_requestor: true,
            role_id_codes: vec![CodedValue::new(value, value, value)],
            access_point: None,
        }],
        source: AuditSource {
            id: value.to_owned(),
            enterprise_site: Some(value.to_owned()),
            type_codes: Vec::new(),
        },
        objects: vec![ParticipantObject {
            id: SecretString::from(value),
            type_code: Some("4"),
            type_code_role: None,
            data_life_cycle: None,
            id_type_code: CodedValue::new(value, value, value),
            name: Some(SecretString::from(value)),
            query: None,
            details: vec![(value.to_owned(), SecretString::from(value))],
        }],
    }
}

fn xml(message: &AuditMessage) -> String {
    let bytes = message.to_xml().expect("the message is written");
    String::from_utf8(bytes.expose_secret().to_vec()).expect("UTF-8")
}

/// Whether `text` holds a character the escape writes in its place.
fn holds_unescaped(text: &str) -> bool {
    text.chars()
        .any(|c| escape::untrusted(&c.to_string()) != c.to_string())
}

#[test]
fn the_escape_writes_each_control_and_bidirectional_character_as_its_code_point() {
    assert_eq!(escape::untrusted(HOSTILE), ESCAPED);
    for (raw, written) in [
        ("\t", r"\u{9}"),
        ("\n", r"\u{a}"),
        ("\r", r"\u{d}"),
        ("\u{7f}", r"\u{7f}"),
        ("\u{9f}", r"\u{9f}"),
        ("\u{61c}", r"\u{61c}"),
        ("\u{200e}", r"\u{200e}"),
        ("\u{200f}", r"\u{200f}"),
        ("\u{202a}", r"\u{202a}"),
        ("\u{2029}", r"\u{2029}"),
        ("\u{2066}", r"\u{2066}"),
        ("\u{2069}", r"\u{2069}"),
        ("\u{feff}", r"\u{feff}"),
        ("\u{fffe}", r"\u{fffe}"),
        ("\u{ffff}", r"\u{ffff}"),
    ] {
        assert_eq!(escape::untrusted(raw), written, "{raw:?}");
    }
}

#[test]
fn the_escape_keeps_every_other_character() {
    let plain = "Qz7-Ünïcødé ש ✓ <&\"'> \\u{202e}";
    assert_eq!(escape::untrusted(plain), plain);
    assert!(matches!(
        escape::untrusted(plain),
        std::borrow::Cow::Borrowed(_)
    ));
}

/// Every attribute and text value of the message is escaped, and the
/// message stays well-formed XML.
#[test]
fn a_hostile_value_is_escaped_in_every_attribute_and_text_of_the_message() {
    let text = xml(&message(HOSTILE));
    assert!(!holds_unescaped(&text), "{text:?}");
    let attributes = [
        "UserID",
        "AlternativeUserID",
        "UserName",
        "AuditSourceID",
        "AuditEnterpriseSiteID",
        "ParticipantObjectID",
        "csd-code",
        "codeSystemName",
        "originalText",
        "type",
    ];
    for attribute in attributes {
        assert!(
            text.contains(&format!(r#"{attribute}="{ESCAPED}""#)),
            "{attribute}: {text}"
        );
    }
    for element in ["EventOutcomeDescription", "ParticipantObjectName"] {
        assert!(
            text.contains(&format!("<{element}>{ESCAPED}</{element}>")),
            "{element}: {text}"
        );
    }
    let mut reader = quick_xml::Reader::from_str(&text);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("the message is no XML: {error}"),
        }
    }
}

/// DICOM PS3.15 A.5.1.1: a `ValuePair` value is base64 "to protect against
/// text contents that contain XML fragments", captured "without
/// modification by the audit encoding process", so the base64 is the
/// escape and the value is kept as given; its `type` is escaped.
#[test]
fn a_detail_value_is_the_base64_of_the_value_as_given() {
    use base64::Engine as _;
    let text = xml(&message(HOSTILE));
    let encoded = base64::engine::general_purpose::STANDARD.encode(HOSTILE);
    assert!(text.contains(&format!(r#"type="{ESCAPED}""#)), "{text}");
    assert!(text.contains(&format!(r#"value="{encoded}""#)), "{text}");
}

#[test]
fn a_value_of_ten_thousand_characters_is_written_whole_and_escaped() {
    let long: String = "Qz7\u{202e}".repeat(2_500);
    assert_eq!(long.chars().count(), 10_000);
    let text = xml(&message(&long));
    assert!(!holds_unescaped(&text));
    assert!(
        text.contains(&format!(r#"UserID="{}""#, r"Qz7\u{202e}".repeat(2_500))),
        "the long value is cut"
    );
}

/// The syslog frame carries the message as written, so it holds no raw
/// control character either: its header fields are printable US-ASCII
/// (RFC 5424 §6) and its MSG is the escaped XML.
#[test]
fn a_syslog_frame_of_a_hostile_message_holds_no_control_character() {
    let sender = Sender::new("gateway.example.org", "ihe-atna", "4242").expect("a sender");
    let frame = sender.frame(
        jiff::Timestamp::UNIX_EPOCH,
        &message(HOSTILE).to_xml().expect("the message is written"),
    );
    let text = std::str::from_utf8(frame.expose_secret()).expect("UTF-8");
    assert!(!holds_unescaped(text), "{text:?}");
    assert!(text.contains(ESCAPED), "{text}");
}

/// The code a participant object's id type is written with stays the one
/// the table gives when it holds nothing to escape.
#[test]
fn a_value_with_nothing_to_escape_is_written_as_given() {
    let mut message = message("Qz7-plain-81");
    message.objects[0].id_type_code = CodedValue::of(codes::OBJECT_ID_PATIENT_NUMBER);
    let text = xml(&message);
    assert!(
        text.contains(
            r#"<ParticipantObjectIDTypeCode csd-code="2" codeSystemName="RFC-3881" originalText="Patient Number"/>"#
        ),
        "{text}"
    );
}
