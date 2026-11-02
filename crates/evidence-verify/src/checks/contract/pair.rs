//! The rules that read both parties of a full package: distinct accounts,
//! leaves and keys under one root, the hash chain from position 2 to the
//! position 1 signature and T1, and the position 1 certificate in the firm
//! CRL up to Tf. Rationale: the section "Evidence packages" of
//! `docs/adr/0086-bilateral-contract-signing.md`.

use super::super::in_entry;
use super::differing;
use super::parties::Party;
use crate::certificate::{self, Certificate};
use crate::crl;
use crate::report::Finding;

/// The parties must be distinct accounts with distinct leaves and keys, and
/// name the same root. Distinct keys do not stop someone who generates two.
pub(super) fn distinct(client: &Party<'_>, firm: &Party<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut same_key = false;
    if let (Ok(one), Ok(two)) = (&client.statement, &firm.statement) {
        if one.account == two.account {
            findings.push(Finding::failed(
                "contract.same_account",
                "both statements name the same account",
            ));
        }
        same_key |= one.certificate_sha256 == two.certificate_sha256;
        if one.root_sha256 != two.root_sha256 {
            findings.push(Finding::failed(
                "contract.root_mismatch",
                "the statements name different roots",
            ));
        }
    }
    if let (Ok(one), Ok(two)) = (&client.certificate, &firm.certificate) {
        let key = |certificate: &Certificate| {
            certificate
                .decoded
                .tbs_certificate
                .subject_public_key_info
                .clone()
        };
        same_key |= one.der == two.der || key(one) == key(two);
    }
    if same_key {
        findings.push(Finding::failed(
            "contract.same_key",
            "both parties hold the same certificate leaf or public key",
        ));
    }
    findings
}

/// The position 2 statement must carry the SHA-256 of the position 1
/// signature and of T1 exactly as the package holds them.
pub(super) fn chain_link(firm: &Party<'_>, client_signature: &[u8], t1: &[u8]) -> Option<Finding> {
    let statement = firm.statement.as_ref().ok()?;
    let fields = [
        (
            "prior_signature_sha256",
            statement.prior_signature_sha256 == certificate::sha256(client_signature),
        ),
        (
            "prior_token_sha256",
            statement.prior_token_sha256 == certificate::sha256(t1),
        ),
    ];
    let differing = differing(&fields);
    (!differing.is_empty()).then(|| {
        Finding::failed(
            "contract.chain_mismatch",
            format!("{} differs in {differing}", firm.entries.statement),
        )
    })
}

/// The position 1 certificate in the firm CRL `parte-2-crl.pem` up to the
/// end of the Tf interval: a listed revocation not after it fails; otherwise
/// the list's own outcome is reported, so a later revocation keeps the
/// historical result.
pub(super) fn counterparty(
    client: &Party<'_>,
    firm: &Party<'_>,
    root: Option<&Certificate>,
    tf: Option<(i64, i64)>,
) -> Option<Finding> {
    let (Ok(subject), Ok(list), Some(root), Some((from, to))) =
        (&client.certificate, &firm.crl, root, tf)
    else {
        return None;
    };
    crl::check_issued_by(list, root).ok()?;
    if let Some(instant) =
        crl::revocation_of(list, &subject.serial).filter(|instant| *instant <= to)
    {
        return Some(Finding::failed(
            "contract.counterparty_revoked",
            format!(
                "{} lists the position 1 certificate as revoked at {instant}, not after {to}",
                firm.entries.crl
            ),
        ));
    }
    Some(in_entry(
        "Client at Tf genTime (parte-2-crl.pem)",
        crl::status(list, root, &subject.serial, from, to),
    ))
}
