use std::str::FromStr;

use domain::case_administration::{
    CaseAdministrationValues, CaseAdministrativeStatus, CaseEditableValues, CaseRevision,
    CaseStageRevision, InitialCaseStage, PenalCaseCreation, PenalCaseProfile,
};
use domain::cases::CaseMetadata;
use domain::identity::{Permission, Role};
use domain::DomainError;

fn profile(
    fields: [&str; 4],
    offenses: &[&str],
    info: Option<&str>,
    ids: Option<&str>,
) -> Result<PenalCaseProfile, DomainError> {
    PenalCaseProfile::new(
        fields[0], fields[1], fields[2], fields[3], offenses, info, ids,
    )
}

#[test]
fn profile_is_complete_and_preserves_manual_text_order_and_unicode_forms() {
    let p = profile(
        [" NUC ", "\u{2003} Fiscal\u{ed}a ", " CJ ", " Organo "],
        &[" a,b ", "A", "\u{c1}", "A\u{301}"],
        Some(" Linea 1\r\nLinea 2 \n"),
        Some(" \u{2003}"),
    )
    .unwrap();
    assert_eq!(p.nuc(), "NUC");
    assert_eq!(p.nuc_authority(), "Fiscal\u{ed}a");
    assert_eq!(p.judicial_case_number(), "CJ");
    assert_eq!(p.judicial_authority(), "Organo");
    assert_eq!(p.offenses(), ["a,b", "A", "\u{c1}", "A\u{301}"]);
    assert_eq!(p.general_information(), Some("Linea 1\nLinea 2"));
    assert_eq!(p.complementary_identifiers(), None);
    for index in 0..4 {
        let mut fields = ["N", "F", "J", "O"];
        fields[index] = " \u{2003}";
        assert!(profile(fields, &["A"], None, None).is_err());
    }
}

#[test]
fn offenses_require_raw_bounded_cardinality_and_reject_exact_duplicates() {
    for offenses in [vec![], vec!["A"; 9], vec!["A", " A "], vec![" "]] {
        assert!(profile(["N", "F", "J", "O"], &offenses, None, None).is_err());
    }
    let error = profile(["N", "F", "J", "O"], &["Secret", "Secret"], None, None).unwrap_err();
    assert_eq!(
        error,
        DomainError::InvalidPenalCaseProfile {
            field: "offenses",
            reason: "duplicate entries are not allowed"
        }
    );
    assert!(!error.to_string().contains("Secret"));
    assert!(profile(["N", "F", "J", "O"], &["A", "a"], None, None).is_ok());
}

#[test]
fn controls_are_rejected_before_trim_except_normalized_information_lf() {
    for code in (0..=31).chain(127..=159) {
        let control = char::from_u32(code).unwrap();
        let input = format!("{control}Secret{control}");
        for index in 0..4 {
            let mut fields = ["N", "F", "J", "O"];
            fields[index] = &input;
            let error = profile(fields, &["A"], None, None).unwrap_err();
            assert!(!error.to_string().contains("Secret"));
        }
        assert!(profile(["N", "F", "J", "O"], &[&input], None, None).is_err());
        assert!(profile(["N", "F", "J", "O"], &["A"], None, Some(&input)).is_err());
        let result = profile(["N", "F", "J", "O"], &["A"], Some(&input), None);
        if control == '\n' {
            assert_eq!(result.unwrap().general_information(), Some("Secret"));
        } else {
            assert!(result.is_err());
        }
    }
    for input in ["\r", " \r ", "x\r\ny\r", "x\r\ry", "x\t\r\ny"] {
        assert!(profile(["N", "F", "J", "O"], &["A"], Some(input), None).is_err());
    }
    for input in ["\r\n \r\n", "\n", " \u{2003}", ""] {
        assert_eq!(
            profile(["N", "F", "J", "O"], &["A"], Some(input), None)
                .unwrap()
                .general_information(),
            None
        );
    }
    let p = profile(["N", "F", "J", "O"], &["A"], Some("x\r\n\r\ny\nz"), None).unwrap();
    assert_eq!(p.general_information(), Some("x\n\ny\nz"));
}

#[test]
fn scalar_limits_apply_after_normalization_and_bound_the_full_encoding() {
    let fields = [
        "\u{10000}".repeat(100),
        "\u{10000}".repeat(200),
        "\u{10000}".repeat(100),
        "\u{10000}".repeat(200),
    ];
    let mut offenses = Vec::new();
    for i in 0..8 {
        offenses.push(char::from_u32(0x10000 + i).unwrap().to_string().repeat(120));
    }
    let refs: Vec<_> = offenses.iter().map(String::as_str).collect();
    let info = "\u{10000}".repeat(1000);
    let ids = "\u{10000}".repeat(300);
    let fields_ref = fields.each_ref().map(String::as_str);
    let p = profile(fields_ref, &refs, Some(&info), Some(&ids)).unwrap();
    let metadata = CaseMetadata::new(&"\u{10000}".repeat(200), &"\u{10000}".repeat(100)).unwrap();
    let values = PenalCaseCreation::new(metadata, p).into_values();
    assert_eq!(values.canonical_bytes().len(), 12_717);
    for index in 0..4 {
        let mut longer = fields.clone();
        longer[index].push('x');
        assert!(profile(longer.each_ref().map(String::as_str), &["A"], None, None).is_err());
    }
    assert!(profile(fields_ref, &[&("A".repeat(121))], None, None).is_err());
    assert!(profile(fields_ref, &["A"], Some(&(info + "x")), None).is_err());
    assert!(profile(fields_ref, &["A"], None, Some(&(ids + "x"))).is_err());
    let lines = "x\r\n".repeat(500);
    assert_eq!(
        profile(fields_ref, &["A"], Some(&lines), None)
            .unwrap()
            .general_information()
            .unwrap()
            .chars()
            .count(),
        999
    );
}

#[test]
fn editable_values_and_status_do_not_invent_a_profile_or_stage() {
    let metadata = CaseMetadata::new("Title", "Ref").unwrap();
    let basic = CaseAdministrationValues::basic(metadata.clone());
    assert_eq!(basic.metadata(), &metadata);
    assert_eq!(basic.profile(), None);
    assert_eq!(basic.status(), CaseAdministrativeStatus::Active);
    let p = profile(["N", "F", "J", "O"], &["A"], None, None).unwrap();
    let creation = PenalCaseCreation::new(metadata.clone(), p.clone());
    assert_eq!(creation.metadata(), &metadata);
    assert_eq!(creation.profile(), &p);
    let active = creation.into_values();
    let editable = CaseEditableValues::new(metadata, Some(p));
    assert_eq!(active.editable(), &editable);
    let closed = active.with_status(CaseAdministrativeStatus::Closed);
    assert_eq!(closed.editable(), active.editable());
    assert_eq!(closed.status(), CaseAdministrativeStatus::Closed);
    assert_eq!(closed.with_status(CaseAdministrativeStatus::Active), active);
}

#[test]
fn revisions_are_positive_and_the_stage_counter_is_independent() {
    assert_eq!(CaseRevision::new(0), Err(DomainError::InvalidCaseRevision));
    assert_eq!(CaseRevision::FIRST.get(), 1);
    assert_eq!(
        CaseRevision::FIRST.next(),
        Some(CaseRevision::new(2).unwrap())
    );
    assert_eq!(CaseRevision::new(u32::MAX).unwrap().next(), None);
    assert_eq!(
        CaseStageRevision::new(0),
        Err(DomainError::InvalidCaseStageRevision)
    );
    assert_eq!(CaseStageRevision::FIRST.get(), 1);
    assert_eq!(CaseStageRevision::new(u32::MAX).unwrap().get(), u32::MAX);
    assert_eq!(InitialCaseStage::Investigation.as_str(), "investigation");
}

#[test]
fn status_parsing_is_exact_and_does_not_retain_rejected_input() {
    for (name, status) in [
        ("active", CaseAdministrativeStatus::Active),
        ("closed", CaseAdministrativeStatus::Closed),
    ] {
        assert_eq!(status.as_str(), name);
        assert_eq!(CaseAdministrativeStatus::from_str(name), Ok(status));
    }
    for invalid in ["", "Active", " closed", "closed ", "Secret"] {
        let error = CaseAdministrativeStatus::from_str(invalid).unwrap_err();
        assert_eq!(error, DomainError::InvalidCaseAdministrativeStatus);
        assert!(!error.to_string().contains("Secret"));
    }
}

#[test]
fn staff_administration_permissions_do_not_expand_basic_client_visibility() {
    for (role, read, manage) in [
        (Role::Owner, true, true),
        (Role::Litigator, true, true),
        (Role::Paralegal, true, false),
        (Role::Client, false, false),
    ] {
        assert_eq!(role.allows(Permission::ReadCaseAdministration), read);
        assert_eq!(role.allows(Permission::ManageCaseAdministration), manage);
        assert!(domain::cases::can_read_case(role, true));
    }
}
