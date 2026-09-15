use domain::case_administration::{
    CaseAdministrationValues, CaseAdministrativeStatus, CaseEditableValues, PenalCaseProfile,
};
use domain::cases::CaseMetadata;

#[test]
fn cadm1_vectors_match_independent_utf8_length_and_optional_encodings() {
    {
        let profile = None;
        let editable =
            CaseEditableValues::new(CaseMetadata::new("Expediente", "REF-001").unwrap(), profile);
        let values = CaseAdministrationValues::new(editable, CaseAdministrativeStatus::Active);
        let bytes = values.canonical_bytes();
        assert_eq!(bytes.len(), 32);
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "4341444d31000000000a457870656469656e7465000000075245462d30303100"
        );
    }
    {
        let profile = Some(
            PenalCaseProfile::new(
                "NUC-1",
                "Fiscalia",
                "CJ-1",
                "Juzgado",
                &["Delito A", "Delito B"],
                Some("Linea 1\nLinea 2"),
                None,
            )
            .unwrap(),
        );
        let editable = CaseEditableValues::new(
            CaseMetadata::new("Expediente penal", "REF-002").unwrap(),
            profile,
        );
        let values = CaseAdministrationValues::new(editable, CaseAdministrativeStatus::Closed);
        let bytes = values.canonical_bytes();
        assert_eq!(bytes.len(), 127);
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "4341444d310100000010457870656469656e74652070656e616c000000075245462d30303201000000054e55432d310000000846697363616c696100000004434a2d31000000074a757a6761646f000000020000000844656c69746f20410000000844656c69746f2042010000000f4c696e656120310a4c696e6561203200");
    }
    {
        let profile = Some(
            PenalCaseProfile::new(
                "NUC-2",
                "Fiscal\u{ed}a",
                "CJ-2",
                "\u{d3}rgano",
                &["a,b", "A"],
                Some("L\u{ed}nea uno\nL\u{ed}nea dos \u{1f600}"),
                Some("EX-1"),
            )
            .unwrap(),
        );
        let editable = CaseEditableValues::new(
            CaseMetadata::new("Jos\u{e9} \u{10000}", "ref").unwrap(),
            profile,
        );
        let values = CaseAdministrationValues::new(editable, CaseAdministrativeStatus::Active);
        let bytes = values.canonical_bytes();
        assert_eq!(bytes.len(), 125);
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "4341444d31000000000a4a6f73c3a920f09080800000000372656601000000054e55432d320000000946697363616cc3ad6100000004434a2d3200000007c3937267616e6f0000000200000003612c620000000141010000001a4cc3ad6e656120756e6f0a4cc3ad6e656120646f7320f09f9880010000000445582d31");
    }
}
