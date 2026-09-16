use application::ApplicationError;
use domain::crypto::{DocumentHasher, Sha256Digest};
use postgres::GenericClient;
use serde_json::Value;
use uuid::Uuid;

use super::{inconsistent, port};

mod credentials;
mod relations;
mod reviews;

pub(crate) use credentials::validate_binding as validate_credential_binding;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    relations::validate(client)?;
    values(client, true)?;
    values(client, false)?;
    reviews::validate(client)?;
    credentials::validate(client)?;
    Ok(())
}

fn values<C: GenericClient>(client: &mut C, subject: bool) -> Result<(), ApplicationError> {
    let (table, id_column) = if subject {
        ("case_subject_revisions", "subject_id")
    } else {
        ("case_participant_typed_revisions", "participant_id")
    };
    let mut id = Uuid::nil();
    let mut revision = 0_i64;
    loop {
        let rows=client.query(&format!("SELECT {id_column},revision,values_canonical,values_digest,values_view,changed_at,changed_by_email
            FROM {table} WHERE ({id_column},revision)>($1,$2) ORDER BY {id_column},revision LIMIT 64"),&[&id,&revision]).map_err(port)?;
        for row in &rows {
            let canonical: Vec<u8> = row.try_get(2).map_err(|_| inconsistent())?;
            let expected: Vec<u8> = row.try_get(3).map_err(|_| inconsistent())?;
            let view: Value = row.try_get(4).map_err(|_| inconsistent())?;
            digest(&canonical, &expected)?;
            if subject {
                crate::typed_participant_codec::subject(&canonical, &view)?;
            } else {
                crate::typed_participant_codec::participant(&canonical, &view)?;
            }
            crate::postgres_participant_schema::validate_provenance(row.get(5), row.get(6))?;
            id = row.try_get(0).map_err(|_| inconsistent())?;
            revision = row.try_get(1).map_err(|_| inconsistent())?;
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}

fn digest(bytes: &[u8], expected: &[u8]) -> Result<Sha256Digest, ApplicationError> {
    let value = Sha256Digest::from_bytes(expected).map_err(|_| inconsistent())?;
    if crate::RingSha256Hasher.hash_bytes(bytes) != value {
        return Err(inconsistent());
    }
    Ok(value)
}
