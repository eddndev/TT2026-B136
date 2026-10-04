use application::ApplicationError;
use postgres::Client;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    crate::resource_hearing_postgres::validate_inventory(client)
}
