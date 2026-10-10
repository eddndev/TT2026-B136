use super::{inconsistent, load, port};
use application::ApplicationError;
use postgres::GenericClient;
use uuid::Uuid;

/// Rechecks actual signatures at captured time under captured immutable trust.
pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let verifier = crate::certificates::InternalRsaDocumentSigningVerifier::new();
    let mut after: Option<Uuid> = None;
    loop {
        let ids = client
            .query(
                "SELECT binding_id FROM document_signing_bindings
            WHERE ($1::uuid IS NULL OR binding_id>$1) ORDER BY binding_id LIMIT 64",
                &[&after],
            )
            .map_err(port)?;
        for row in &ids {
            let binding: Uuid = row.try_get(0).map_err(|_| inconsistent())?;
            let receipt = load(client, binding)?.ok_or_else(inconsistent)?;
            let check = verifier
                .verify_binding(
                    &receipt.statement,
                    &receipt.check.certificate.der,
                    &receipt.check.signature,
                    &receipt.trust,
                    receipt.check.checked_at,
                )
                .map_err(|_| inconsistent())?;
            if check != receipt.check {
                return Err(inconsistent());
            }
            after = Some(binding);
        }
        if ids.len() < 64 {
            return Ok(());
        }
    }
}
