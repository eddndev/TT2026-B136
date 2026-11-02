use super::*;
use crate::contracts::{
    error::ContractWorkflowError,
    model::EventKind,
    test_support::{principal, request, snapshot, CLIENT, FIRM, OTHER, REQUESTER},
};
use domain::identity::Role;
use EventKind::*;

fn relation(n: u128, role: Role, scoped: bool) -> Relation {
    let mut value = snapshot(&[Created, Requested], vec![request(100, 0)]);
    value.case_scope = scoped;
    Relation::of(&principal(n, role), &value, value.requests.first())
}

fn not_found(result: Result<(), ApplicationError>) -> bool {
    matches!(
        result,
        Err(ApplicationError::Contract(ContractWorkflowError::NotFound))
    )
}

fn denied(result: Result<(), ApplicationError>) -> bool {
    matches!(result, Err(ApplicationError::PermissionDenied))
}

#[test]
fn staff_reach_contracts_by_role_and_scope_only() {
    assert!(relation(OTHER, Role::Owner, true).manage().is_ok());
    assert!(relation(OTHER, Role::Litigator, true).manage().is_ok());
    assert!(denied(relation(OTHER, Role::Paralegal, true).manage()));
    assert!(relation(OTHER, Role::Paralegal, true)
        .read_contract()
        .is_ok());
    assert!(not_found(
        relation(OTHER, Role::Litigator, false).read_contract()
    ));
    assert!(not_found(relation(OTHER, Role::Litigator, false).manage()));
    assert!(not_found(
        relation(OTHER, Role::Client, false).read_request()
    ));
}

#[test]
fn a_client_reaches_only_its_own_request_and_never_the_case() {
    let client = relation(CLIENT, Role::Client, false);
    assert!(client.read_request().is_ok());
    assert!(not_found(client.read_contract()));
    let case_route = snapshot(&[Created, Requested], vec![request(100, 0)]);
    assert!(not_found(
        Relation::of(&principal(CLIENT, Role::Client), &case_route, None).manage()
    ));
    assert!(client.sign(1).is_ok());
    assert!(denied(client.sign(2)));
    assert!(client.close(Rejected).is_ok());
    assert!(denied(client.close(Declined)));
    assert!(denied(client.close(Cancelled)));
}

#[test]
fn content_follows_the_request_and_the_client_signature() {
    let client = relation(CLIENT, Role::Client, false);
    let firm = relation(FIRM, Role::Litigator, true);
    let staff = relation(OTHER, Role::Owner, true);
    assert!(client
        .read_content(RequestStatus::AwaitingClient, false)
        .is_ok());
    assert!(not_found(
        client.read_content(RequestStatus::Rejected, false)
    ));
    assert!(client.read_content(RequestStatus::Declined, true).is_ok());
    assert!(firm.read_content(RequestStatus::AwaitingFirm, true).is_ok());
    assert!(not_found(firm.read_content(RequestStatus::Completed, true)));
    assert!(not_found(
        staff.read_content(RequestStatus::AwaitingClient, false)
    ));
}

#[test]
fn packages_require_export_scope_or_a_designation() {
    let client = relation(CLIENT, Role::Client, false);
    assert!(not_found(client.partial_package(false)));
    assert!(client.partial_package(true).is_ok());
    assert!(client.final_package().is_ok());
    assert!(relation(OTHER, Role::Paralegal, true)
        .partial_package(false)
        .is_ok());
    assert!(not_found(
        relation(OTHER, Role::Paralegal, false).final_package()
    ));
    assert!(relation(FIRM, Role::Litigator, false)
        .final_package()
        .is_ok());
}

#[test]
fn only_an_owner_or_the_managing_requester_cancels() {
    assert!(relation(OTHER, Role::Owner, true).close(Cancelled).is_ok());
    assert!(relation(REQUESTER, Role::Litigator, true)
        .close(Cancelled)
        .is_ok());
    assert!(not_found(
        relation(REQUESTER, Role::Litigator, false).close(Cancelled)
    ));
    assert!(denied(
        relation(OTHER, Role::Litigator, true).close(Cancelled)
    ));
    assert!(relation(FIRM, Role::Litigator, true)
        .close(Declined)
        .is_ok());
    assert!(denied(
        relation(FIRM, Role::Litigator, true).close(Rejected)
    ));
}
