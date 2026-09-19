//! Connection preconditions shared by schema initialization and runtime opening.
use application::ApplicationError;
use postgres::Client;

pub(crate) fn port_error(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("postgres initialization: {error}"))
}

pub(crate) fn require_utf8(client: &mut Client) -> Result<(), ApplicationError> {
    let encoding: String = client
        .query_one("SELECT pg_catalog.current_setting('server_encoding')", &[])
        .map_err(port_error)?
        .get(0);
    if encoding != "UTF8" {
        return Err(ApplicationError::InvalidConfiguration(
            "PostgreSQL database encoding must be UTF8 for canonical document metadata".into(),
        ));
    }
    Ok(())
}
