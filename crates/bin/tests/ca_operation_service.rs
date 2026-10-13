//! Native HTTP -> isolated worker -> OpenSSL acceptance on owned backends.
//! Requires CA_OPERATIONS_TEST_ADMIN_URL naming a dedicated disposable cluster,
//! with qadra/qadra_admin absent, plus the existing CA identity and Redis URLs.
#![cfg(target_os = "linux")]

#[allow(dead_code)]
#[path = "../../infrastructure/tests/ca_identity_cases/support.rs"]
mod ca_identity_support;
mod ca_operation_support;
#[allow(dead_code)]
mod ca_service_support;

use application::identity::{Principal, SessionIdentity, SessionPolicy, SessionStore};
use domain::identity::Role;
use infrastructure::RedisSessionStore;
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};
use std::fs;
use zeroize::Zeroizing;

use ca_identity_support::Fixture;
use ca_operation_support::NativeCa;
use ca_service_support::{redis_url, response_json, take_secret, Credentials, Service};

const OPERATIONS: &str = "/api/v1/ca/operations";

fn execute(service: &Service, token: &str, command: Value) -> Value {
    response_json(
        service.post_authorized(OPERATIONS, token, command),
        StatusCode::OK,
    )
}

#[test]
fn authenticated_native_ca_issues_both_profiles_and_revokes_exact_certificate_pending_publication()
{
    let redis = redis_url();
    let mut fixture = Fixture::new();
    let credentials = Credentials::enroll(&mut fixture);
    let ca = NativeCa::new(&fixture);
    let holder_key = Zeroizing::new(fs::read(&ca.holder_key).unwrap());
    let authority_key = Zeroizing::new(fs::read(ca.ca_file("private/ca.key.pem")).unwrap());
    let original_crl = fs::read(ca.ca_file("crl/crl.pem")).unwrap();
    let crl_counter = fs::read(ca.ca_file("crlnumber")).unwrap();
    let mut service =
        Service::start_with_operations(&fixture, &credentials, &redis, Some(ca.operations()));

    let request = ca.command("partner");
    assert_eq!(
        service.post(OPERATIONS, request.clone()).status(),
        StatusCode::UNAUTHORIZED
    );
    let challenge = service.login(&fixture.operator.email, &credentials.password);
    assert_eq!(
        service
            .post_authorized(OPERATIONS, &challenge, request.clone())
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let office = RedisSessionStore::connect(&redis).unwrap();
    let office_token = Zeroizing::new(
        office
            .create_session(
                &SessionIdentity {
                    principal: Principal {
                        id: fixture.operator.id,
                        email: fixture.operator.email.clone(),
                        role: Role::Owner,
                    },
                    auth_generation: fixture.operator.auth_generation,
                },
                SessionPolicy::new(300, Some(120)).unwrap(),
            )
            .unwrap()
            .access_token,
    );
    assert_eq!(
        service
            .post_authorized(OPERATIONS, &office_token, request.clone())
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(fs::read(ca.ca_file("serial")).unwrap(), b"1000\n");
    assert!(fs::read(ca.ca_file("index.txt")).unwrap().is_empty());

    let mut session = response_json(
        service.post(
            "/api/v1/auth/mfa/totp",
            json!({
                "challenge_token": &*challenge, "code": &*credentials.totp(),
            }),
        ),
        StatusCode::OK,
    );
    let token = take_secret(&mut session, "access_token");
    rejected_requests_do_not_mutate(&service, &token, &ca, &request);

    let state = execute(&service, &token, json!({"action": "status"}));
    assert_eq!(state["root_fingerprint"], ca.fingerprint);
    assert_eq!(state["revision"], 1);
    assert_eq!(state["valid_now"], true);
    let preview = execute(
        &service,
        &token,
        json!({
            "action": "preview", "csr_pem": ca.csr, "purpose": "partner",
        }),
    );
    assert_eq!(preview["root_fingerprint"], ca.fingerprint);
    assert_eq!(preview["purpose"], "partner");
    assert!(preview["subject"]
        .as_str()
        .unwrap()
        .contains("External HTTP Holder"));
    assert_eq!(preview["spki_sha256"].as_str().unwrap().len(), 64);

    let first = execute(&service, &token, request.clone());
    assert_eq!(first["operator_id"], fixture.operator.id.to_string());
    assert_eq!(first["purpose"], "partner");
    ca.assert_signed_by_root(first["certificate_pem"].as_str().unwrap());
    assert_eq!(execute(&service, &token, request.clone()), first);
    let second = execute(&service, &token, ca.command("document-signing"));
    assert_eq!(second["purpose"], "document-signing");
    assert_ne!(second["serial"], first["serial"]);
    ca.assert_signed_by_root(second["certificate_pem"].as_str().unwrap());
    assert_eq!(fs::read(ca.ca_file("serial")).unwrap(), b"1002\n");
    assert_public_inventory_and_download(&service, &token, &first, &second);
    let history = execute(
        &service,
        &token,
        json!({"action": "history", "offset": 0, "limit": 100}),
    );
    assert_eq!(history["total"], 2);
    for row in history["items"].as_array().unwrap() {
        assert_eq!(row["operator_id"], fixture.operator.id.to_string());
        assert_eq!(row["kind"], "issue");
        assert_eq!(row["reference"], "owned-http-case");
        assert_eq!(row["state"], "completed");
    }

    let revoke = json!({"action": "revoke", "operation_id": uuid::Uuid::new_v4(),
        "serial": first["serial"], "fingerprint": first["fingerprint"],
        "reason": "External holder requested cancellation", "expected_root_fingerprint": ca.fingerprint});
    let receipt = execute(&service, &token, revoke.clone());
    assert_eq!(receipt["operator_id"], fixture.operator.id.to_string());
    assert_eq!(receipt["local_status"], "revoked");
    assert_eq!(receipt["published_status"], "pending");
    assert_eq!(execute(&service, &token, revoke), receipt);
    let inventory = execute(
        &service,
        &token,
        json!({"action": "inventory", "offset": 0, "limit": 100}),
    );
    let rows = inventory["items"].as_array().unwrap();
    let revoked = rows
        .iter()
        .find(|row| row["serial"] == first["serial"])
        .unwrap();
    assert_eq!(revoked["local_status"], "revoked");
    assert_eq!(revoked["published_status"], "pending");
    assert_eq!(
        rows.iter()
            .find(|row| row["serial"] == second["serial"])
            .unwrap()["published_status"],
        "not-revoked"
    );
    let history = execute(
        &service,
        &token,
        json!({"action": "history", "offset": 0, "limit": 100}),
    );
    assert_eq!(history["total"], 3);
    assert_eq!(
        history["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "revoke")
            .count(),
        1
    );
    let state = execute(&service, &token, json!({"action": "status"}));
    assert_eq!(state["revision"], 1);
    assert_eq!(state["revoked_serials"], json!([]));
    assert_eq!(fs::read(ca.ca_file("crl/crl.pem")).unwrap(), original_crl);
    assert_eq!(fs::read(ca.ca_file("crlnumber")).unwrap(), crl_counter);
    assert!(
        fs::read(&ca.holder_key).unwrap() == *holder_key,
        "external holder key changed"
    );
    assert!(
        fs::read(ca.ca_file("private/ca.key.pem")).unwrap() == *authority_key,
        "CA key changed"
    );
    assert_eq!(fs::read_dir(ca.ca_file("private")).unwrap().count(), 1);
    assert_eq!(
        service
            .bearer(Method::POST, "/api/v1/auth/logout", &token)
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        service
            .post_authorized(OPERATIONS, &token, ca.command("partner"))
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(fs::read(ca.ca_file("serial")).unwrap(), b"1002\n");
    service.stop();
    fixture.assert_office_unchanged();
}

fn rejected_requests_do_not_mutate(service: &Service, token: &str, ca: &NativeCa, request: &Value) {
    let mut forged = request.clone();
    forged["operator_id"] = json!(uuid::Uuid::new_v4());
    assert_eq!(
        service.post_authorized(OPERATIONS, token, forged).status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut path = request.clone();
    path["csr_path"] = json!("../private/ca.key.pem");
    assert_eq!(
        service.post_authorized(OPERATIONS, token, path).status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        service
            .post_authorized(
                OPERATIONS,
                token,
                json!({"action": "shell", "command": "not admitted"})
            )
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut foreign_root = request.clone();
    foreign_root["expected_root_fingerprint"] = json!("0".repeat(64));
    assert_eq!(
        service
            .post_authorized(OPERATIONS, token, foreign_root)
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        service.bearer(Method::GET, "/api/v1/cases", token).status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(fs::read(ca.ca_file("serial")).unwrap(), b"1000\n");
    assert!(fs::read(ca.ca_file("index.txt")).unwrap().is_empty());
    assert!(!ca.ca_file("operations").exists());
}

fn assert_public_inventory_and_download(
    service: &Service,
    token: &str,
    first: &Value,
    second: &Value,
) {
    let page = execute(
        service,
        token,
        json!({"action": "inventory", "offset": 0, "limit": 100}),
    );
    assert_eq!(page["total"], 2);
    for receipt in [first, second] {
        let row = page["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["serial"] == receipt["serial"])
            .unwrap();
        assert_eq!(row["fingerprint"], receipt["fingerprint"]);
        assert_eq!(row["purpose"], receipt["purpose"]);
        assert_eq!(row["local_status"], "issued");
        assert_eq!(row["published_status"], "not-revoked");
        let certificate = execute(
            service,
            token,
            json!({"action": "certificate",
            "serial": receipt["serial"], "fingerprint": receipt["fingerprint"]}),
        );
        assert_eq!(certificate["certificate_pem"], receipt["certificate_pem"]);
    }
}
