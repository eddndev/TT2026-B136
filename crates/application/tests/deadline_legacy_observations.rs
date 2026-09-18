#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;

use application::{
    deadline_inputs::{DeadlineCalendarRef, DeadlineSourceDetail},
    deadline_observations::{build_deadline_observations, build_legacy_deadline_observations},
    deadline_profiles::DeadlineProfileDefinition,
    deadline_reevaluation::{decode_observations, encode_observations, ObservationRole},
    deadlines::*,
    procedural_facts::*,
};
use deadline_observation_support as observations;
use deadline_support::{evaluation::inputs, *};
use domain::{
    crypto::Sha256Digest,
    deadline_triggers::{TriggerField, TriggerRequirement},
    judicial_calendars::JudicialCalendarClassification,
};

fn observed(
    value: &DeadlineDetail,
) -> Result<application::deadline_reevaluation::Observations, application::ApplicationError> {
    build_legacy_deadline_observations(inputs::hasher().as_ref(), value)
}

#[test]
fn legacy_resolution_uses_only_the_captured_profile_and_heads() {
    let value = deadline_tracked_support::legacy();
    let before = deadline_capture_bytes(inputs::hasher().as_ref(), &value).unwrap();
    let expected = build_deadline_observations(
        inputs::hasher().as_ref(),
        value.case_id,
        &value.calculation.profile,
        &value.calculation.material,
        None,
    )
    .unwrap();
    assert_eq!(observed(&value).unwrap(), expected);
    assert_eq!(
        deadline_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        before
    );
    assert_eq!(value.receipt.version, DeadlineReceiptVersion::Legacy);
    assert!(value.tracking.is_none());
}

#[test]
fn legacy_notification_has_no_invented_parent_head_and_preserves_exact_parent_revisions() {
    let value = notification(false);
    let before = value.clone();
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    assert!(build_deadline_observations(
        inputs::hasher().as_ref(),
        value.case_id,
        &value.calculation.profile,
        &value.calculation.material,
        None
    )
    .is_err());
    let observed = observed(&value).unwrap();
    assert_eq!(
        observed.entries.iter().map(|v| v.role).collect::<Vec<_>>(),
        [
            ObservationRole::Profile,
            ObservationRole::Source,
            ObservationRole::Calendar
        ]
    );
    assert_eq!(observed.entries[0].revision, 1);
    assert_eq!(observed.entries[1].revision, 2);
    assert_eq!(observed.entries[1].parent_resolution.unwrap().revision, 2);
    assert_eq!(observed.entries[2].revision, 2);
    assert_eq!(
        decode_observations(&encode_observations(&observed).unwrap()).unwrap(),
        observed
    );
    let selected = observations::fact(&value.calculation.material.source);
    assert_eq!(
        selected
            .sources
            .resolved
            .resolution
            .unwrap()
            .reference
            .revision
            .get(),
        1
    );
    assert_eq!(value, before);
}

#[test]
fn historical_withdrawn_heads_are_preserved_without_querying_new_heads() {
    let value = notification(true);
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    let observed = observed(&value).unwrap();
    assert_eq!(observed.entries[1].revision, 3);
    assert_eq!(observed.entries[2].revision, 3);
    assert!(observed
        .entries
        .iter()
        .all(|entry| entry.role != ObservationRole::NotificationParent));
    let source = observations::fact(&value.calculation.material.source_head);
    assert_eq!(
        observed.entries[1].submission_digest,
        source.snapshot.metadata().receipt.submission_digest
    );
    assert_eq!(
        observed.entries[1].evidence_digest,
        observations::vectors::digest(1, &observations::vectors::fact(source))
    );
}

#[test]
fn legacy_builder_validates_the_entire_deadline_receipt_not_only_source_receipts() {
    for kind in 0..5 {
        let mut value = notification(false);
        match kind {
            0 => value.receipt.review_digest = Sha256Digest::from_array([0; 32]),
            1 => value.receipt.capture_digest = Sha256Digest::from_array([0; 32]),
            2 => value.receipt.submission_digest = Sha256Digest::from_array([0; 32]),
            3 => value.calculation.profile.recorded_by.email = "Changed historical email".into(),
            4 => {
                inputs::metadata_mut(observations::fact_mut(
                    &mut value.calculation.material.source_head,
                ))
                .recorded_by
                .email = "Changed historical email".into()
            }
            _ => unreachable!(),
        }
        assert!(observed(&value).is_err(), "{kind}");
    }
}

#[test]
fn a_rebuilt_outer_commitment_does_not_hide_corrupt_notification_evidence() {
    for selected in [false, true] {
        let mut value = notification(false);
        let material = &mut value.calculation.material;
        let source = observations::fact_mut(if selected {
            &mut material.source
        } else {
            &mut material.source_head
        });
        inputs::metadata_mut(source).receipt.sources_digest = Sha256Digest::from_array([0; 32]);
        deadline_tracked_support::resign(&mut value);
        assert!(observed(&value).is_err());
    }
}

#[test]
fn tracked_records_cannot_be_reinterpreted_as_legacy_observations() {
    let value = deadline_tracked_support::accepted();
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    assert!(observed(&value).is_err());
    let mut missing_capture = value.clone();
    missing_capture.tracking = None;
    assert!(observed(&missing_capture).is_err());
    let mut false_legacy = value;
    false_legacy.receipt.version = DeadlineReceiptVersion::Legacy;
    assert!(observed(&false_legacy).is_err());
}

#[test]
fn a_legacy_unknown_source_produces_only_a_verified_profile_observation() {
    let (mut command, mut preparation) = fixture();
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.selection.source =
        FactDeclaration::Unknown(FactText::new("No source captured").unwrap());
    let material = &mut preparation.resolved.as_mut().unwrap().material;
    material.source = None;
    material.source_head = None;
    let value = detail(&prepare(command, preparation).unwrap());
    let observed = observed(&value).unwrap();
    assert_eq!(observed.entries.len(), 1);
    assert_eq!(observed.entries[0].role, ObservationRole::Profile);
}

fn notification(withdrawn_heads: bool) -> DeadlineDetail {
    let (mut command, mut preparation) = fixture();
    let resolved = preparation.resolved.as_mut().unwrap();
    let mut profile = observations::profile_input(&resolved.profile);
    profile.trigger = TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt);
    resolved.profile.definition = DeadlineProfileDefinition::new(profile).unwrap();
    observations::resign_profile(&mut resolved.profile);
    resolved.profile_head = resolved.profile.clone();
    let (mut material, _) = observations::notification_material();
    observations::calendar_pair(&mut material);
    if withdrawn_heads {
        material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::notification(
            3,
            true,
            2,
            "2026-01-07",
        ))));
        material.calendar_head = Some(inputs::calendar(
            3,
            true,
            JudicialCalendarClassification::Countable,
        ));
    }
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.selection = inputs::request(material.source.as_ref().unwrap()).trigger;
    definition.input.calendar = material
        .calendar
        .as_ref()
        .map(|calendar| DeadlineCalendarRef {
            id: calendar.id,
            revision: calendar.revision,
        });
    preparation.administration = material.administration.clone();
    resolved.material = material;
    detail(&prepare(command, preparation).unwrap())
}
