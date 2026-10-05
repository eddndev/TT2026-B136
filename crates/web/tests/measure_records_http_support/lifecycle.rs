use super::*;

#[tokio::test]
async fn exact_reads_preserve_genuine_m1_c1_m2_c1_families_and_provenance() {
    for (row, family) in mixed(7).into_iter().zip(["m1", "c1", "m2", "c1"]) {
        let selected = row.reference;
        let proof = row.record_history.clone();
        let record = row.record.clone();
        let (status, body) = request(exact_port(row), &exact_path(selected)).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["reference"], reference_json(selected));
        assert_eq!(body["family"], family);
        assert_eq!(body["record"]["family"], family);
        assert_eq!(
            body["record"]["capture"]["capture_digest"],
            selected.digest().to_hex()
        );
        assert_eq!(body["validity"], "valid");
        assert_eq!(body["record_root"]["kind"], "judicial");
        assert_eq!(
            body["record_history"]["records"]["judicial"]["groups"]
                .as_array()
                .unwrap()
                .len(),
            proof.records.judicial.groups.len()
        );
        assert_eq!(
            body["record_history"]["records"]["administrative"]
                .as_array()
                .unwrap()
                .len(),
            proof.records.administrative.len()
        );
        assert_eq!(
            body["record_history"]["decisions"]
                .as_array()
                .unwrap()
                .len(),
            proof.decisions.len()
        );
        match record {
            OwnedMeasureRecord::Judicial(judicial) => {
                assert_eq!(body["last_judicial"]["reference"], reference_json(selected));
                assert_eq!(
                    body["judicial_origin"]["operation_id"],
                    judicial.judicial_origin().operation_id.to_string()
                );
            }
            OwnedMeasureRecord::Administrative { capture, .. } => {
                assert_eq!(
                    body["last_judicial"]["reference"],
                    reference_json(capture.result.last_judicial.reference)
                );
                assert_eq!(
                    body["judicial_origin"]["operation_id"],
                    capture.result.judicial_origin.operation_id.to_string()
                );
                assert_eq!(
                    body["record"]["capture"]["support"]["digest"],
                    capture.support.digest.to_hex()
                );
            }
        }
    }
}

#[tokio::test]
async fn replacement_reads_keep_both_siblings_and_the_real_administrative_root() {
    let fixture = crate::replacement_support::ReplacementFixture::initial();
    let capture = fixture.capture();
    let history = crate::replacement_support::append(&fixture, &capture);
    let link = capture.replacement_link.as_ref().unwrap();
    for member in &capture.records {
        let reference = record_reference(member);
        let is_new = member.result.id == fixture.replacement_id();
        let row = MeasureRecordDetail {
            case_id: fixture.case_id,
            reference,
            record: OwnedMeasureRecord::Administrative {
                owner: MeasureAdministrativeRef {
                    operation_id: fixture.command.operation_id,
                    capture_digest: capture.capture_digest,
                },
                capture: Box::new(member.clone()),
            },
            record_history: history.clone(),
        };
        let (status, body) = request(exact_port(row), &exact_path(reference)).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["family"], "c1");
        assert_eq!(
            body["validity"],
            if is_new { "valid" } else { "entered_in_error" }
        );
        assert_eq!(body["last_action"], "impose");
        assert_eq!(
            body["last_judicial"]["reference"],
            reference_json(fixture.command.target)
        );
        assert_eq!(
            body["record"]["capture"]["result"]["previous"],
            reference_json(fixture.command.target)
        );
        let owner = &body["record_history"]["records"]["administrative"][0]["capture"];
        assert_eq!(owner["family"], "a1");
        assert_eq!(owner["records"].as_array().unwrap().len(), 2);
        assert_eq!(
            owner["replacement_link"],
            json!({"entered_in_error":reference_json(link.entered_in_error),"replacement":reference_json(link.replacement)})
        );
        assert_eq!(
            body["record_root"]["kind"],
            if is_new { "administrative" } else { "judicial" }
        );
        if is_new {
            assert_eq!(
                body["record_root"]["operation_id"],
                fixture.command.operation_id.to_string()
            );
            assert_eq!(
                body["record_root"]["measure_id"],
                fixture.replacement_id().to_string()
            );
            assert_eq!(
                body["record"]["capture"]["result"]["values"]["subject"]["id"],
                fixture.subject.id.to_string()
            );
        }
    }
}

#[tokio::test]
async fn current_marked_head_is_not_replaced_by_its_older_valid_revision() {
    let original = initial(9);
    let marked = administrative(&original, 9, true);
    let selected = marked.reference;
    let (status, body) = request(current_port(marked.clone()), &current_path(selected.id())).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["reference"], reference_json(selected));
    assert_eq!(body["validity"], "entered_in_error");
    let old = original.reference;
    let (status, body) = request(exact_port(original), &exact_path(old)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["reference"], reference_json(old));
    assert_eq!(body["validity"], "valid");
    let (status, body) = request(exact_port(marked), &exact_path(selected)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["validity"], "entered_in_error");
}
