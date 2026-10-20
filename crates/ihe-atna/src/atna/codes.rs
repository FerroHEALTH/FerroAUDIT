// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The code tables an audit message is written with.
//!
//! The DICOM codes are those DICOM PS3.15 2026c Annex A.5.1 and the audit
//! message tables of Annex A.5.3 write as `EV(code, DCM, "text")`; the
//! participant object id types are the default set of RFC 3881 §5.5.4, which
//! the IHE audit tables write with the code system name `RFC-3881` (ITI TF-2
//! §3.20.7). Each coded value is a `(code, codeSystemName, originalText)`
//! triple, which [`CodedValue::of`](super::message::CodedValue::of) reads.
//! The `AuditSourceTypeCode` and `ParticipantObjectDataLifeCycle` codes are
//! the bare values the PS3.15 A.5.1 schema lists.

/// A `(code, codeSystemName, originalText)` triple.
pub type Triple = (&'static str, &'static str, &'static str);

/// The code system name of the DICOM controlled terminology.
pub const DCM: &str = "DCM";

/// The code system name of the RFC 3881 participant object id types.
pub const RFC_3881: &str = "RFC-3881";

// NOTE: DICOM PS3.15 A.5.3: the EventID of each audit message.
/// `EV(110100, DCM, "Application Activity")`.
pub const EVENT_APPLICATION_ACTIVITY: Triple = ("110100", DCM, "Application Activity");
/// `EV(110101, DCM, "Audit Log Used")`.
pub const EVENT_AUDIT_LOG_USED: Triple = ("110101", DCM, "Audit Log Used");
/// `EV(110102, DCM, "Begin Transferring DICOM Instances")`.
pub const EVENT_BEGIN_TRANSFERRING_DICOM_INSTANCES: Triple = ("110102", DCM, "Begin Transferring DICOM Instances");
/// `EV(110103, DCM, "DICOM Instances Accessed")`.
pub const EVENT_DICOM_INSTANCES_ACCESSED: Triple = ("110103", DCM, "DICOM Instances Accessed");
/// `EV(110104, DCM, "DICOM Instances Transferred")`.
pub const EVENT_DICOM_INSTANCES_TRANSFERRED: Triple = ("110104", DCM, "DICOM Instances Transferred");
/// `EV(110105, DCM, "DICOM Study Deleted")`.
pub const EVENT_DICOM_STUDY_DELETED: Triple = ("110105", DCM, "DICOM Study Deleted");
/// `EV(110106, DCM, "Export")`.
pub const EVENT_EXPORT: Triple = ("110106", DCM, "Export");
/// `EV(110107, DCM, "Import")`.
pub const EVENT_IMPORT: Triple = ("110107", DCM, "Import");
/// `EV(110108, DCM, "Network Entry")`.
pub const EVENT_NETWORK_ENTRY: Triple = ("110108", DCM, "Network Entry");
/// `EV(110109, DCM, "Order Record")`.
pub const EVENT_ORDER_RECORD: Triple = ("110109", DCM, "Order Record");
/// `EV(110110, DCM, "Patient Record")`.
pub const EVENT_PATIENT_RECORD: Triple = ("110110", DCM, "Patient Record");
/// `EV(110111, DCM, "Procedure Record")`.
pub const EVENT_PROCEDURE_RECORD: Triple = ("110111", DCM, "Procedure Record");
/// `EV(110112, DCM, "Query")`.
pub const EVENT_QUERY: Triple = ("110112", DCM, "Query");
/// `EV(110113, DCM, "Security Alert")`.
pub const EVENT_SECURITY_ALERT: Triple = ("110113", DCM, "Security Alert");
/// `EV(110114, DCM, "User Authentication")`.
pub const EVENT_USER_AUTHENTICATION: Triple = ("110114", DCM, "User Authentication");

// NOTE: DICOM PS3.15 A.5.3: the EventTypeCode values the audit messages fix.
/// `EV(110120, DCM, "Application Start")`.
pub const TYPE_APPLICATION_START: Triple = ("110120", DCM, "Application Start");
/// `EV(110121, DCM, "Application Stop")`.
pub const TYPE_APPLICATION_STOP: Triple = ("110121", DCM, "Application Stop");
/// `EV(110122, DCM, "Login")`.
pub const TYPE_LOGIN: Triple = ("110122", DCM, "Login");
/// `EV(110123, DCM, "Logout")`.
pub const TYPE_LOGOUT: Triple = ("110123", DCM, "Logout");
/// `EV(110124, DCM, "Attach")`.
pub const TYPE_ATTACH: Triple = ("110124", DCM, "Attach");
/// `EV(110125, DCM, "Detach")`.
pub const TYPE_DETACH: Triple = ("110125", DCM, "Detach");

// NOTE: DICOM PS3.15 A.5.3: the RoleIDCode values of the active participants.
/// `EV(110150, DCM, "Application")`.
pub const ROLE_APPLICATION: Triple = ("110150", DCM, "Application");
/// `EV(110151, DCM, "Application Launcher")`.
pub const ROLE_APPLICATION_LAUNCHER: Triple = ("110151", DCM, "Application Launcher");
/// `EV(110152, DCM, "Destination Role ID")`.
pub const ROLE_DESTINATION_ROLE_ID: Triple = ("110152", DCM, "Destination Role ID");
/// `EV(110153, DCM, "Source Role ID")`.
pub const ROLE_SOURCE_ROLE_ID: Triple = ("110153", DCM, "Source Role ID");
/// `EV(110154, DCM, "Destination Media")`.
pub const ROLE_DESTINATION_MEDIA: Triple = ("110154", DCM, "Destination Media");
/// `EV(110155, DCM, "Source Media")`.
pub const ROLE_SOURCE_MEDIA: Triple = ("110155", DCM, "Source Media");

// NOTE: RFC 3881 §5.5.4: the default ParticipantObjectIDTypeCode set.
/// `EV(1, RFC-3881, "Medical Record Number")`.
pub const OBJECT_ID_MEDICAL_RECORD_NUMBER: Triple = ("1", RFC_3881, "Medical Record Number");
/// `EV(2, RFC-3881, "Patient Number")`.
pub const OBJECT_ID_PATIENT_NUMBER: Triple = ("2", RFC_3881, "Patient Number");
/// `EV(3, RFC-3881, "Encounter Number")`.
pub const OBJECT_ID_ENCOUNTER_NUMBER: Triple = ("3", RFC_3881, "Encounter Number");
/// `EV(4, RFC-3881, "Enrollee Number")`.
pub const OBJECT_ID_ENROLLEE_NUMBER: Triple = ("4", RFC_3881, "Enrollee Number");
/// `EV(5, RFC-3881, "Social Security Number")`.
pub const OBJECT_ID_SOCIAL_SECURITY_NUMBER: Triple = ("5", RFC_3881, "Social Security Number");
/// `EV(6, RFC-3881, "Account Number")`.
pub const OBJECT_ID_ACCOUNT_NUMBER: Triple = ("6", RFC_3881, "Account Number");
/// `EV(7, RFC-3881, "Guarantor Number")`.
pub const OBJECT_ID_GUARANTOR_NUMBER: Triple = ("7", RFC_3881, "Guarantor Number");
/// `EV(8, RFC-3881, "Report Name")`.
pub const OBJECT_ID_REPORT_NAME: Triple = ("8", RFC_3881, "Report Name");
/// `EV(9, RFC-3881, "Report Number")`.
pub const OBJECT_ID_REPORT_NUMBER: Triple = ("9", RFC_3881, "Report Number");
/// `EV(10, RFC-3881, "Search Criteria")`.
pub const OBJECT_ID_SEARCH_CRITERIA: Triple = ("10", RFC_3881, "Search Criteria");
/// `EV(11, RFC-3881, "User Identifier")`.
pub const OBJECT_ID_USER_IDENTIFIER: Triple = ("11", RFC_3881, "User Identifier");
/// `EV(12, RFC-3881, "URI")`.
pub const OBJECT_ID_URI: Triple = ("12", RFC_3881, "URI");

// NOTE: DICOM PS3.15 A.5.1: the AuditSourceTypeCode values, each a bare csd-code.
/// `AuditSourceTypeCode` `1`: End-user display device, diagnostic device.
pub const SOURCE_USER_DEVICE: &str = "1";
/// `AuditSourceTypeCode` `2`: Data acquisition device or instrument.
pub const SOURCE_DATA_INTERFACE: &str = "2";
/// `AuditSourceTypeCode` `3`: Web Server process or thread.
pub const SOURCE_WEB_SERVER: &str = "3";
/// `AuditSourceTypeCode` `4`: Application Server process or thread.
pub const SOURCE_APPLICATION_SERVER: &str = "4";
/// `AuditSourceTypeCode` `5`: Database Server process or thread.
pub const SOURCE_DATABASE_SERVER: &str = "5";
/// `AuditSourceTypeCode` `6`: Security server, e.g., a domain controller.
pub const SOURCE_SECURITY_SERVER: &str = "6";
/// `AuditSourceTypeCode` `7`: ISO level 1-3 network component.
pub const SOURCE_NETWORK_COMPONENT: &str = "7";
/// `AuditSourceTypeCode` `8`: ISO level 4-6 operating software.
pub const SOURCE_OPERATING_SOFTWARE: &str = "8";
/// `AuditSourceTypeCode` `9`: other.
pub const SOURCE_OTHER: &str = "9";

// NOTE: DICOM PS3.15 A.5.1: the ParticipantObjectDataLifeCycle values.
/// `ParticipantObjectDataLifeCycle` `1`: Origination, Creation.
pub const LIFE_CYCLE_ORIGINATION: &str = "1";
/// `ParticipantObjectDataLifeCycle` `2`: Import/ Copy.
pub const LIFE_CYCLE_IMPORT: &str = "2";
/// `ParticipantObjectDataLifeCycle` `3`: Amendment.
pub const LIFE_CYCLE_AMENDMENT: &str = "3";
/// `ParticipantObjectDataLifeCycle` `4`: Verification.
pub const LIFE_CYCLE_VERIFICATION: &str = "4";
/// `ParticipantObjectDataLifeCycle` `5`: Translation.
pub const LIFE_CYCLE_TRANSLATION: &str = "5";
/// `ParticipantObjectDataLifeCycle` `6`: Access/Use.
pub const LIFE_CYCLE_ACCESS: &str = "6";
/// `ParticipantObjectDataLifeCycle` `7`: De-identification.
pub const LIFE_CYCLE_DE_IDENTIFICATION: &str = "7";
/// `ParticipantObjectDataLifeCycle` `8`: Aggregation, summarization, derivation.
pub const LIFE_CYCLE_AGGREGATION: &str = "8";
/// `ParticipantObjectDataLifeCycle` `9`: Report.
pub const LIFE_CYCLE_REPORT: &str = "9";
/// `ParticipantObjectDataLifeCycle` `10`: Export.
pub const LIFE_CYCLE_EXPORT: &str = "10";
/// `ParticipantObjectDataLifeCycle` `11`: Disclosure.
pub const LIFE_CYCLE_DISCLOSURE: &str = "11";
/// `ParticipantObjectDataLifeCycle` `12`: Receipt of Disclosure.
pub const LIFE_CYCLE_RECEIPT_OF_DISCLOSURE: &str = "12";
/// `ParticipantObjectDataLifeCycle` `13`: Archiving.
pub const LIFE_CYCLE_ARCHIVING: &str = "13";
/// `ParticipantObjectDataLifeCycle` `14`: Logical deletion.
pub const LIFE_CYCLE_LOGICAL_DELETION: &str = "14";
/// `ParticipantObjectDataLifeCycle` `15`: Permanent erasure, physical destruction.
pub const LIFE_CYCLE_PERMANENT_ERASURE: &str = "15";
