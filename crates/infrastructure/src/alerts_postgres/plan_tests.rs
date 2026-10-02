use super::*;
use crate::alerts_postgres::{invalidate, schedule_rows};
use application::{deadlines::DeadlineId, hearings::HearingId};
use domain::{cases::CaseId, crypto::Sha256Digest};

fn fixture(hearing: bool) -> (subject::Verified, OffsetDateTime) {
    let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    let case_id = CaseId::from_uuid(Uuid::from_u128(1));
    let subject = if hearing {
        AlertSubject::Hearing {
            case_id,
            id: HearingId::from_uuid(Uuid::from_u128(2)),
        }
    } else {
        AlertSubject::Deadline {
            case_id,
            id: DeadlineId::from_uuid(Uuid::from_u128(2)),
        }
    };
    (
        subject::Verified {
            subject,
            origin: AlertOrigin {
                revision: 1,
                evidence_digest: Sha256Digest::from_array([3; 32]),
            },
            activity_at: Some(now + Duration::hours(6)),
            attention_pending: !hearing,
            review: false,
            ended: None,
            responsible: None,
            subject_title: "Activity".into(),
            case_title: "Case".into(),
            case_reference: "REF-1".into(),
        },
        now,
    )
}

fn moved(hearing: bool) -> (subject::Verified, Value, OffsetDateTime) {
    let (mut current, now) = fixture(hearing);
    let previous = state(&invalidate::initial(), &current, now).unwrap();
    current.activity_at = Some(now + Duration::hours(12));
    current.origin.revision = 2;
    let next = state(&previous, &current, now).unwrap();
    (current, next, now)
}

fn validate_plans(current: &subject::Verified, proposals: Vec<Plan>) {
    assert!(
        !proposals.is_empty(),
        "rescheduling must preserve upcoming alerts"
    );
    for plan in proposals {
        let scheduled = schedule_rows::Scheduled {
            id: Uuid::new_v4(),
            subject: current.subject,
            recipient: UserId::from_uuid(Uuid::from_u128(4)),
            origin: current.origin,
            subject_title: current.subject_title.clone(),
            case_title: current.case_title.clone(),
            case_reference: current.case_reference.clone(),
            plan,
            status: "planned".into(),
        };
        let preview = schedule_rows::record(&scheduled, scheduled.plan.trigger, false);
        assert!(
            preview
                .validate(preview.recipient_id, preview.created_at)
                .is_ok(),
            "persisted schedule must satisfy startup notification validation: {:?}",
            preview.kind
        );
    }
}

#[test]
fn hearing_reschedule_does_not_create_a_deadline_change_episode() {
    let (_, next, _) = moved(true);
    assert!(next["changed_episode"].is_null());
    assert!(next["review_episode"].is_null());
    assert!(next["overdue_episode"].is_null());
}

#[test]
fn hearing_reschedule_plans_pass_startup_notification_validation() {
    let (current, next, now) = moved(true);
    let proposals = plans(&current, &next, &AlertPreferenceValues::default(), now).unwrap();
    validate_plans(&current, proposals);
}

#[test]
fn deadline_reschedule_retains_change_notice_and_valid_upcoming_plans() {
    let (current, next, now) = moved(false);
    assert!(!next["changed_episode"].is_null());
    let proposals = plans(&current, &next, &AlertPreferenceValues::default(), now).unwrap();
    assert!(proposals.iter().any(|plan| {
        matches!(plan.kind, AlertKind::DueChangedSoon {
            previous_due_at,
            current_due_at,
        } if previous_due_at == now + Duration::hours(6)
            && current_due_at == now + Duration::hours(12))
    }));
    validate_plans(&current, proposals);
}
