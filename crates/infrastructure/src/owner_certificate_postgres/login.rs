use application::{
    identity::certificate_login::{CertificateLoginContext, OwnerLoginAuthority},
    ApplicationError,
};
use domain::identity::UserId;
use postgres::GenericClient;
use uuid::Uuid;

use super::{account, port, query, storage, PostgresOwnerCertificateStore};

impl OwnerLoginAuthority for PostgresOwnerCertificateStore {
    fn load(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<Option<CertificateLoginContext>, ApplicationError> {
        let mut client = self.client()?;
        // The shared writer lock precedes every authority read. READ COMMITTED
        // then observes account and trust changes committed during that wait.
        let mut tx = crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
        let context = current(&mut tx, owner, binding)?;
        tx.commit().map_err(port)?;
        Ok(context)
    }
}

fn current<C: GenericClient>(
    client: &mut C,
    owner: UserId,
    binding: Uuid,
) -> Result<Option<CertificateLoginContext>, ApplicationError> {
    if owner.as_uuid().is_nil() || binding.is_nil() {
        return Ok(None);
    }
    let account = match account::owner(client, owner) {
        Ok(account) => account,
        Err(ApplicationError::PermissionDenied) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(receipt) = query::load(client, binding, Some(owner))? else {
        return Ok(None);
    };
    if receipt.record.withdrawal().is_some() {
        return Ok(None);
    }
    let Some(trust) = crate::credential_trust_postgres::current(client).map_err(|_| storage())?
    else {
        return Ok(None);
    };
    // Historical evidence remains immutable. Login admission uses current
    // account and trust facts; its caller checks time and verifies the proof.
    Ok(Some(CertificateLoginContext {
        account,
        binding_id: binding,
        certificate: receipt.check.certificate,
        trust,
    }))
}
