// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The regulatory metadata of an access (Xt-EHR D5.2 §7.2.7): declared once
//! by the deployment, read from its configuration, and overridden by what a
//! recording system knows of one access.

use ehds_logging::regulatory::{Condition, Lawfulness, LegalBasis, Regulatory};
use serde_json::json;

/// The deployment's declaration: health care under a public task, one
/// controller and one processor.
fn deployment() -> Regulatory {
    serde_json::from_value(json!({
        "legal_basis": {"lawfulness": "public-task", "condition": "health-or-social-care"},
        "controllers": ["Qz7-controller-86"],
        "processors": ["Qz7-processor-87"],
    }))
    .expect("the declaration reads")
}

#[test]
fn a_deployment_declares_its_legal_basis_controllers_and_processors() {
    let declared = deployment();
    assert_eq!(
        declared.legal_basis,
        Some(LegalBasis {
            lawfulness: Lawfulness::PublicTask,
            condition: Some(Condition::HealthOrSocialCare),
        })
    );
    assert_eq!(declared.controllers, ["Qz7-controller-86"]);
    assert_eq!(declared.processors, ["Qz7-processor-87"]);
}

#[test]
fn an_unknown_member_or_point_is_refused() {
    for refused in [
        json!({"legal_basis": {"lawfulness": "public-task"}, "controller": "Qz7"}),
        json!({"legal_basis": {"lawfulness": "article-6-1-g"}}),
        json!({"legal_basis": {"lawfulness": "consent", "condition": "health"}}),
    ] {
        assert!(
            serde_json::from_value::<Regulatory>(refused.clone()).is_err(),
            "{refused} is read"
        );
    }
}

/// A legal basis stored beside one access, such as an explicit consent,
/// takes the place of the deployment's; the controllers and processors the
/// access does not name stay the deployment's.
#[test]
fn an_access_overrides_what_it_states_and_keeps_the_rest() {
    let access = Regulatory {
        legal_basis: Some(LegalBasis {
            lawfulness: Lawfulness::Consent,
            condition: Some(Condition::ExplicitConsent),
        }),
        ..Regulatory::default()
    };
    let recorded = deployment().overridden_by(&access);
    assert_eq!(recorded.legal_basis, access.legal_basis);
    assert_eq!(recorded.controllers, ["Qz7-controller-86"]);
    assert_eq!(recorded.processors, ["Qz7-processor-87"]);

    let processors = Regulatory {
        processors: vec!["Qz7-processor-88".to_owned()],
        ..Regulatory::default()
    };
    let recorded = deployment().overridden_by(&processors);
    assert_eq!(recorded.legal_basis, deployment().legal_basis);
    assert_eq!(recorded.processors, ["Qz7-processor-88"]);
}

/// Each point is written by its article, paragraph and point letter.
#[test]
fn each_point_is_written_by_its_article_and_letter() {
    assert_eq!(Lawfulness::Consent.code(), "gdpr-art-6-1-a");
    assert_eq!(Lawfulness::LegitimateInterests.code(), "gdpr-art-6-1-f");
    assert_eq!(Condition::ExplicitConsent.code(), "gdpr-art-9-2-a");
    assert_eq!(Condition::HealthOrSocialCare.code(), "gdpr-art-9-2-h");
    assert_eq!(Condition::ArchivingOrResearch.code(), "gdpr-art-9-2-j");
}
