use std::{
    thread,
    time::{Duration, Instant},
};

use application::credential_trust::CredentialTrustSnapshot;
use der::{
    asn1::{OctetString, Uint},
    oid::AssociatedOid,
    Decode, Encode,
};
use domain::{
    audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS},
    crypto::{InternalDeclarationVerifier, Sha256Digest},
};
use infrastructure::{certificates::InternalRsaDeclarationVerifier, RingSha256Hasher};
use postgres::{Client, Transaction};
use uuid::Uuid;
use x509_cert::{crl::CertificateList, ext::pkix::CrlNumber};

use crate::{declaration_fixture, support::Clock};

pub const AUDIT_LOCK: i64 = 0x4155444954;

pub fn named_url(base: &str, repeatable_read: bool) -> (String, String) {
    let name = format!("owner_certificate_wait_{}", Uuid::new_v4().simple());
    let mut url = reqwest::Url::parse(base).unwrap();
    let pairs: Vec<_> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let mut options = pairs
        .iter()
        .filter(|(k, _)| k == "options")
        .map(|(_, v)| v.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    options.push_str(" -cstatement_timeout=6000 -clock_timeout=5000");
    if repeatable_read {
        options.push_str(" -cdefault_transaction_isolation=repeatable\\ read");
    }
    url.query_pairs_mut()
        .clear()
        .extend_pairs(
            pairs
                .iter()
                .filter(|(k, _)| k != "options" && k != "application_name")
                .map(|(k, v)| (k.as_str(), v.as_str())),
        )
        .append_pair("options", &options)
        .append_pair("application_name", &name);
    (url.to_string().replace('+', "%20"), name)
}

pub fn wait_for_lock(client: &mut Client, name: &str, event: &str) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let waiting: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
            WHERE application_name=$1 AND wait_event_type='Lock' AND wait_event=$2)",
                &[&name, &event],
            )
            .unwrap()
            .get(0);
        if waiting {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("owner certificate mutation did not reach the expected database lock wait");
}

/// Publish real signed successor material inside the test's held audit lock.
/// The explicit transaction permits deterministic contention with the adapter.
pub fn publish_locked(tx: &mut Transaction<'_>, prior: &CredentialTrustSnapshot, clock: &Clock) {
    let mut crl = CertificateList::from_der(&prior.inspection.crl_der).unwrap();
    let number = prior.inspection.crl_number.checked_add(1).unwrap();
    let ext = crl
        .tbs_cert_list
        .crl_extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|ext| ext.extn_id == CrlNumber::OID)
        .unwrap();
    ext.extn_value = OctetString::new(
        CrlNumber(Uint::new(&number.to_be_bytes()).unwrap())
            .to_der()
            .unwrap(),
    )
    .unwrap();
    let material = InternalRsaDeclarationVerifier::new()
        .inspect_trust(
            &prior.inspection.root_der,
            &declaration_fixture::signed_crl(crl),
            clock.at().unix_timestamp(),
        )
        .unwrap();
    let revision = i64::from(prior.revision.get()) + 1;
    let by: String = tx.query_one("SELECT SESSION_USER", &[]).unwrap().get(0);
    tx.execute(
        "INSERT INTO participant_credential_trust_revisions(
        deployment_id,revision,crl_der,crl_digest,crl_number,crl_this_update,crl_next_update,
        valid_from,valid_until,published_at_seconds,published_at_nanoseconds,published_by)
        VALUES($1,$2,$3,$4,$5::text::numeric,$6,$7,$8,$9,$10,$11,$12)",
        &[
            &prior.deployment_id,
            &revision,
            &material.crl_der,
            &&material.crl_digest.as_bytes()[..],
            &number.to_string(),
            &material.crl_this_update,
            &material.crl_next_update,
            &material.valid_from,
            &material.valid_until,
            &clock.at().unix_timestamp(),
            &(clock.at().nanosecond() as i32),
            &by,
        ],
    )
    .unwrap();
    let last = tx
        .query_opt(
            "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
            &[],
        )
        .unwrap();
    let (sequence, previous) = last.map_or((0, GENESIS_PREVIOUS), |row| {
        let sequence: i64 = row.get(0);
        let bytes: Vec<u8> = row.get(1);
        (
            (sequence + 1) as u64,
            Sha256Digest::from_array(bytes.try_into().unwrap()),
        )
    });
    let actor = format!("database-admin:{by}");
    let resource = format!(
        "credential-trust:{}:revision:{revision}:root:{}:crl:{}",
        prior.deployment_id,
        material.root_fingerprint.to_hex(),
        material.crl_digest.to_hex()
    );
    let event = AuditEvent::new(
        sequence,
        clock.at(),
        &actor,
        "participant.credential_trust_published",
        &resource,
    );
    let chain = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&(sequence as i64), &event.timestamp_rfc3339().unwrap(), &actor, &event.action, &resource,
          &&chain.as_bytes()[..]]).unwrap();
}
