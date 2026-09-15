mod case_administration_support;
use application::cases::{
    case_administration_digest, CaseAdministrationValues, CaseAdministrativeStatus,
    CaseEditableValues, PenalCaseProfile,
};
use case_administration_support::Fixture;
use domain::cases::CaseMetadata;
use infrastructure::RingSha256Hasher;

#[test]
fn maximum_multibyte_profile_has_identical_sql_rust_canonical_bytes_and_digest() {
    let Some(mut f) = Fixture::new() else { return };
    let symbol = "\u{10000}";
    let offenses: Vec<String> = (0..8)
        .map(|n| {
            format!(
                "{}{}",
                symbol.repeat(119),
                char::from_u32(0x10001 + n).unwrap()
            )
        })
        .collect();
    let refs: Vec<&str> = offenses.iter().map(String::as_str).collect();
    let profile = PenalCaseProfile::new(
        &symbol.repeat(100),
        &symbol.repeat(200),
        &symbol.repeat(100),
        &symbol.repeat(200),
        &refs,
        Some(&symbol.repeat(1000)),
        Some(&symbol.repeat(300)),
    )
    .unwrap();
    let values = CaseAdministrationValues::new(
        CaseEditableValues::new(
            CaseMetadata::new(&symbol.repeat(200), &symbol.repeat(100)).unwrap(),
            Some(profile),
        ),
        CaseAdministrativeStatus::Closed,
    );
    let p = values.profile().unwrap();
    let row=f.admin.query_one("SELECT case_administration_bytes($1,$2,$3,$4,$5,$6,$7,$8,$9,$10),sha256(case_administration_bytes($1,$2,$3,$4,$5,$6,$7,$8,$9,$10))", &[&values.status().as_str(),&values.metadata().title(),&values.metadata().reference(),&p.nuc(),&p.nuc_authority(),&p.judicial_case_number(),&p.judicial_authority(),&p.offenses(),&p.general_information(),&p.complementary_identifiers()]).unwrap();
    let bytes: Vec<u8> = row.get(0);
    assert_eq!(bytes.len(), 12717);
    assert_eq!(bytes, values.canonical_bytes());
    let digest: Vec<u8> = row.get(1);
    assert_eq!(
        digest,
        case_administration_digest(&RingSha256Hasher, &values).as_bytes()
    );
}
#[test]
fn persisted_canonical_checks_reject_controls_partial_profiles_and_noncanonical_offense_arrays() {
    let Some(mut f) = Fixture::new() else { return };
    for expression in [
        "case_administration_text_valid(E'\\rbad',200,FALSE)",
        "case_administration_text_valid(E'bad\\t',200,FALSE)",
        "case_administration_text_valid(E'Line1\\r\\nLine2',1000,TRUE)",
        "case_administration_text_valid(E'\\nLine',1000,TRUE)",
        "case_administration_is_canonical('active','Title','REF',NULL,NULL,NULL,NULL,ARRAY['Only'],NULL,NULL)",
        "case_administration_is_canonical('active','Title','REF','NUC','Office','F','Court',ARRAY['Same','Same'],NULL,NULL)",
        "case_administration_is_canonical('active','Title','REF','NUC','Office','F','Court',ARRAY[NULL]::text[],NULL,NULL)",
        "case_administration_is_canonical('active','Title','REF','NUC','Office','F','Court','[0:0]={Reported}'::text[],NULL,NULL)",
        "case_administration_is_canonical('active','Title','REF','NUC','Office','F','Court',ARRAY[['Reported']],NULL,NULL)",
        "case_administration_is_canonical('Active','Title','REF',NULL,NULL,NULL,NULL,NULL,NULL,NULL)",
    ] {let valid:bool=f.admin.query_one(&format!("SELECT {expression}"),&[]).unwrap().get(0);assert!(!valid,"accepted {expression}");}
    let valid: bool = f
        .admin
        .query_one(
            "SELECT case_administration_text_valid(E'Line1\\nLine2',1000,TRUE)",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(valid);
}
