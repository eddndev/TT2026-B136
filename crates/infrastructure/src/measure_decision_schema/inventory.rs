use application::ApplicationError;
use postgres::Client;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    crate::measure_decision_postgres::validate_inventory(client)
}
