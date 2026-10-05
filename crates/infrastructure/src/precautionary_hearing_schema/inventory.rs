use application::ApplicationError;
use postgres::Client;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    crate::precautionary_hearing_postgres::validate_inventory(client)
}
