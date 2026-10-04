use application::ApplicationError;
use postgres::GenericClient;
use uuid::Uuid;

use super::{inconsistent, port, query};

/// Reverify historical public evidence at its captured time, never against latest trust.
pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let verifier = crate::certificates::InternalRsaOwnerBindingVerifier::new();
    let mut after: Option<Uuid> = None;
    loop {
        let ids = client
            .query(
                "SELECT binding_id FROM owner_certificate_registrations
            WHERE ($1::uuid IS NULL OR binding_id>$1) ORDER BY binding_id LIMIT 64",
                &[&after],
            )
            .map_err(port)?;
        for row in &ids {
            let binding: Uuid = row.try_get(0).map_err(|_| inconsistent())?;
            let receipt = query::load(client, binding, None)?.ok_or_else(inconsistent)?;
            let checked = verifier
                .verify_registration(
                    receipt.record.registration(),
                    &receipt.check.certificate.der,
                    &receipt.check.signature,
                    &receipt.trust,
                    receipt.check.checked_at,
                )
                .map_err(|_| inconsistent())?;
            if checked != receipt.check {
                return Err(inconsistent());
            }
            after = Some(binding);
        }
        if ids.len() < 64 {
            return Ok(());
        }
    }
}
