use super::{local, outcome, Response, Server};
use application::identity::password_reset::ResetDeliveryOutcome;

#[test]
fn positional_json_arrays_do_not_establish_delivery_outcomes() {
    let mut outcomes = Vec::new();
    for (status, body) in [
        (200, "[\"0e6d1d44-17c3-4d1b-b32e-d8ffcf91681e\",null]"),
        (400, "[null,\"validation_error\"]"),
    ] {
        let server = Server::new(Response::reply(status, body));
        outcomes.push((status, outcome(&local(&server))));
        assert_eq!(server.finish().len(), 1);
    }
    assert_eq!(
        outcomes,
        vec![
            (200, ResetDeliveryOutcome::Uncertain),
            (400, ResetDeliveryOutcome::Uncertain),
        ]
    );
}
