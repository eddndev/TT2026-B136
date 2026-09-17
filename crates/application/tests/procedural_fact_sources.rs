mod procedural_fact_sources_support;
use application::procedural_facts::*;
use domain::{participants::DirectoryStatus, typed_participants::*};
use procedural_fact_sources_support::*;
use uuid::Uuid;

#[test]
fn empty_sources_have_an_exact_prefix_and_three_zero_counts() {
    assert_eq!(
        fact_sources_bytes(&empty()).unwrap(),
        b"PFSRC1\0\0\0\0\0\0\0\0\0\0\0\0\0"
    );
}
#[test]
fn exact_revisions_and_absent_agreement_differ_from_nil_agreement() {
    let sources = full();
    let bytes = fact_sources_bytes(&sources).unwrap();
    let mut changed = sources.clone();
    changed.resolved.hearing_results.remove(0);
    changed.views.hearing_results.remove(0);
    assert_ne!(bytes, fact_sources_bytes(&changed).unwrap());
    assert_eq!(sources, full());
}
#[test]
fn counts_reject_missing_extra_and_duplicate_views() {
    let valid = full();
    for kind in 0..6 {
        let mut value = valid.clone();
        match kind {
            0 => value.views.resolution = None,
            1 => value.resolved.resolution = None,
            2 => {
                value.views.participants.pop();
            }
            3 => value
                .views
                .participants
                .push(value.views.participants[0].clone()),
            4 => {
                value.views.hearing_results.pop();
            }
            _ => value
                .views
                .hearing_results
                .push(value.views.hearing_results[0].clone()),
        }
        inconsistent(&value);
    }
}
#[test]
fn cardinalities_are_bounded_before_encoding() {
    let mut participants = empty();
    for id in 0..5 {
        add_participant(&mut participants, id, 1);
    }
    inconsistent(&participants);
    let mut results = empty();
    for id in 0..3 {
        add_hearing(&mut results, id, None);
    }
    inconsistent(&results);
    let mut documents = empty();
    for id in 0..3 {
        add_support(&mut documents, id, 1);
    }
    inconsistent(&documents);
}
#[test]
fn every_list_rejects_reverse_order_and_duplicate_exact_keys() {
    for kind in 0..3 {
        for duplicate in [false, true] {
            let mut value = full();
            match kind {
                0 => {
                    if duplicate {
                        value.resolved.participants[1] = value.resolved.participants[0];
                        value.views.participants[1] = value.views.participants[0].clone();
                    } else {
                        value.resolved.participants.reverse();
                        value.views.participants.reverse();
                    }
                }
                1 => {
                    if duplicate {
                        value.resolved.hearing_results[1] = value.resolved.hearing_results[0];
                        value.views.hearing_results[1] = value.views.hearing_results[0].clone();
                    } else {
                        value.resolved.hearing_results.reverse();
                        value.views.hearing_results.reverse();
                    }
                }
                _ => {
                    if duplicate {
                        value.direct_supports[1] = value.direct_supports[0].clone();
                    } else {
                        value.direct_supports.reverse();
                    }
                }
            }
            inconsistent(&value);
        }
    }
}
#[test]
fn participant_view_requires_exact_scope_reference_status_and_subject() {
    for field in 0..6 {
        let mut value = full();
        let view = &mut value.views.participants[0];
        match field {
            0 => view.case_id = domain::cases::CaseId::from_uuid(Uuid::from_u128(99)),
            1 => view.id = domain::participants::ParticipantId::from_uuid(Uuid::from_u128(99)),
            2 => view.revision = domain::participants::ParticipantRevision::new(99).unwrap(),
            3 => view.directory_status = DirectoryStatus::Archived,
            4 => {
                view.subject = Some(SubjectRevisionRef {
                    id: CaseSubjectId::from_uuid(Uuid::nil()),
                    revision: SubjectRevision::initial(),
                    values_digest: digest(1),
                })
            }
            _ => view.kind = Some(ParticipantKind::Defendant),
        }
        inconsistent(&value);
    }
}
#[test]
fn participant_text_is_bounded_and_already_normalized() {
    for text in ["", " name", "name ", "line\nbreak", &"x".repeat(201)] {
        let mut value = full();
        value.views.participants[0].display_name = text.into();
        inconsistent(&value);
    }
    for text in ["", " role", "role\t", &"x".repeat(81)] {
        let mut value = full();
        value.views.participants[0].procedural_role = text.into();
        inconsistent(&value);
    }
    for text in ["", " org", "org\r", &"x".repeat(201)] {
        let mut value = full();
        value.views.participants[0].organization = Some(text.into());
        inconsistent(&value);
    }
}
#[test]
fn typed_kind_and_subject_are_paired_and_role_matches_kind() {
    let mut value = full();
    let subject = SubjectRevisionRef {
        id: CaseSubjectId::from_uuid(Uuid::nil()),
        revision: SubjectRevision::initial(),
        values_digest: digest(1),
    };
    value.resolved.participants[0].subject = Some(subject);
    value.views.participants[0].subject = Some(subject);
    inconsistent(&value);
    value.views.participants[0].kind = Some(ParticipantKind::Defendant);
    inconsistent(&value);
    value.views.participants[0].procedural_role = "defendant".into();
    assert!(fact_sources_bytes(&value).is_ok());
}
#[test]
fn resolution_and_hearing_views_cannot_select_another_reference() {
    let mut value = full();
    value.views.resolution.as_mut().unwrap().reference.revision = FactRevision::new(2).unwrap();
    inconsistent(&value);
    let mut value = full();
    value.views.hearing_results[0].reference = value.views.hearing_results[1].reference;
    inconsistent(&value);
    let mut value = full();
    value.views.hearing_results[0].agreement = value.views.hearing_results[1].agreement.clone();
    inconsistent(&value);
    let mut value = full();
    value.views.hearing_results[1].agreement = None;
    inconsistent(&value);
}
#[test]
fn document_names_use_the_safe_archive_name_policy() {
    for name in ["", "../file.pdf", "a/b.pdf", "a b.pdf", &"x".repeat(129)] {
        let mut value = full();
        value.direct_supports[0].name = name.into();
        inconsistent(&value);
    }
    let mut value = full();
    value.direct_supports[0].name = "x".repeat(128);
    assert!(fact_sources_bytes(&value).is_ok());
}

#[test]
fn source_digest_uses_exact_bytes_and_never_hashes_invalid_shape() {
    use domain::{
        crypto::{DocumentHasher, Sha256Digest},
        DomainError,
    };
    use std::{cell::Cell, io::Read};
    struct Probe {
        expected: Vec<u8>,
        calls: Cell<usize>,
    }
    impl DocumentHasher for Probe {
        fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
            assert_eq!(bytes, self.expected);
            self.calls.set(self.calls.get() + 1);
            digest(42)
        }
        fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
            panic!("source encoding must hash its canonical byte slice")
        }
    }
    let mut sources = full();
    let probe = Probe {
        expected: fact_sources_bytes(&sources).unwrap(),
        calls: Cell::new(0),
    };
    assert_eq!(fact_sources_digest(&probe, &sources).unwrap(), digest(42));
    assert_eq!(probe.calls.get(), 1);
    sources.views.resolution = None;
    assert!(fact_sources_digest(&probe, &sources).is_err());
    assert_eq!(probe.calls.get(), 1);
}

#[test]
fn every_readable_view_and_captured_status_changes_the_bytes() {
    use domain::{
        hearing_results::*, judicial_calendars::CivilDate, procedural_time::DeclaredProceduralTime,
    };
    let original = full_distinct_results();
    let canonical = fact_sources_bytes(&original).unwrap();
    for field in 0..15 {
        let mut changed = original.clone();
        match field {
            0 => {
                changed.views.resolution.as_mut().unwrap().class =
                    FactDeclaration::Known(ResolutionClass::Judgment)
            }
            1 => {
                changed.views.resolution.as_mut().unwrap().issuer =
                    FactDeclaration::Known(FactLabel::new("Court").unwrap())
            }
            2 => {
                changed.views.resolution.as_mut().unwrap().issued_at =
                    DeclaredProceduralTime::minute(
                        "2026-09-16".parse::<CivilDate>().unwrap(),
                        12,
                        34,
                        None,
                    )
                    .unwrap()
            }
            3 => {
                changed.views.resolution.as_mut().unwrap().summary =
                    FactText::new("Changed summary").unwrap()
            }
            4 => changed.views.participants[0].display_name = "Other name".into(),
            5 => changed.views.participants[0].procedural_role = "Other role".into(),
            6 => changed.views.participants[0].organization = Some("Organization".into()),
            7 => changed.views.hearing_results[0].occurrence = HearingResultOccurrence::NotStarted,
            8 => {
                changed.views.hearing_results[0].event_time =
                    DeclaredHearingResultTime::instant(time::OffsetDateTime::UNIX_EPOCH).unwrap()
            }
            9 => {
                changed.views.hearing_results[0].summary =
                    HearingResultText::new("Other summary").unwrap()
            }
            10 => {
                let id = changed.views.hearing_results[1]
                    .agreement
                    .as_ref()
                    .unwrap()
                    .id();
                changed.views.hearing_results[1].agreement = Some(HearingResultAgreement::new(
                    id,
                    HearingResultText::new("Other agreement").unwrap(),
                ));
            }
            11 => changed.resolved.resolution.as_mut().unwrap().status = FactStatus::Withdrawn,
            12 => {
                changed.resolved.participants[0].status = DirectoryStatus::Archived;
                changed.views.participants[0].directory_status = DirectoryStatus::Archived;
            }
            13 => changed.resolved.hearing_results[0].status = HearingResultStatus::Withdrawn,
            _ => {
                changed.direct_supports[0].format =
                    application::documents::StageDocumentFormat::Docx
            }
        }
        assert_ne!(
            fact_sources_bytes(&changed).unwrap(),
            canonical,
            "field {field}"
        );
    }
}

#[test]
fn source_digests_scope_and_document_names_are_bound() {
    let original = full_distinct_results();
    let canonical = fact_sources_bytes(&original).unwrap();
    for field in 0..9 {
        let mut changed = original.clone();
        match field {
            0 => changed.resolved.resolution.as_mut().unwrap().values_digest = digest(99),
            1 => {
                changed
                    .resolved
                    .resolution
                    .as_mut()
                    .unwrap()
                    .submission_digest = digest(99)
            }
            2 => changed.resolved.participants[0].values_digest = digest(99),
            3 => changed.resolved.hearing_results[0].values_digest = digest(99),
            4 => changed.resolved.hearing_results[0].submission_digest = digest(99),
            5 => changed.direct_supports[0].digest = digest(99),
            6 => changed.direct_supports[0].name = "other.pdf".into(),
            7 => {
                changed.resolved.resolution.as_mut().unwrap().case_id =
                    domain::cases::CaseId::from_uuid(Uuid::nil())
            }
            _ => {
                changed.resolved.hearing_results[0].case_id =
                    domain::cases::CaseId::from_uuid(Uuid::nil())
            }
        }
        assert_ne!(
            fact_sources_bytes(&changed).unwrap(),
            canonical,
            "field {field}"
        );
    }
}

#[test]
fn unicode_scalar_limits_encode_utf8_byte_lengths_without_normalization() {
    let mut sources = full();
    let value = "\u{1f600}".repeat(200);
    sources.views.participants[0].display_name = value.clone();
    let encoded = fact_sources_bytes(&sources).unwrap();
    let mut expected = 800u32.to_be_bytes().to_vec();
    expected.extend_from_slice(value.as_bytes());
    assert!(encoded
        .windows(expected.len())
        .any(|window| window == expected));
    sources.views.participants[0].display_name.push('x');
    inconsistent(&sources);
    let mut first = full();
    let mut second = first.clone();
    first.views.participants[0].display_name = "\u{e9}".into();
    second.views.participants[0].display_name = "e\u{301}".into();
    assert_ne!(
        fact_sources_bytes(&first).unwrap(),
        fact_sources_bytes(&second).unwrap()
    );
}

#[test]
fn different_agreements_of_one_result_require_one_consistent_result_snapshot() {
    use domain::hearing_results::*;
    for field in 0..7 {
        let mut sources = full();
        match field {
            0 => {
                sources.resolved.hearing_results[1].case_id =
                    domain::cases::CaseId::from_uuid(Uuid::nil())
            }
            1 => sources.resolved.hearing_results[1].values_digest = digest(99),
            2 => sources.resolved.hearing_results[1].submission_digest = digest(99),
            3 => sources.resolved.hearing_results[1].status = HearingResultStatus::Withdrawn,
            4 => sources.views.hearing_results[1].occurrence = HearingResultOccurrence::NotStarted,
            5 => {
                sources.views.hearing_results[1].event_time =
                    DeclaredHearingResultTime::instant(time::OffsetDateTime::UNIX_EPOCH).unwrap()
            }
            _ => {
                sources.views.hearing_results[1].summary =
                    HearingResultText::new("Different summary").unwrap()
            }
        }
        inconsistent(&sources);
    }
    assert!(fact_sources_bytes(&full()).is_ok());
}
