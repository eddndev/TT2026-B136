//! The document time-stamp token, its time-stamping certificate and the
//! anchor of that certificate.
//!
//! The anchor is the root that issued the token signer certificate, found
//! among the certificates of `tsa-chain.pem`, or of `ca.pem` when a server
//! seal package has no `tsa-chain.pem`, and admitted only when its
//! fingerprint is pinned for time-stamping: those entries hold candidates,
//! never anchors, and the certificates inside the token are never anchor
//! candidates at all, so a pinned root carried only by the token does not
//! anchor it. A root the served trust returns for time-stamping without being pinned is
//! indeterminate trust, as for every other purpose, and the token is still
//! checked against it. A token without either anchor, or whose policy is
//! not admitted, is not supported; a token that has one but fails a CMS,
//! ESS, imprint, profile or validity check is failed. The token's own
//! checks (CMS, ESS, imprint, policy) form the document token component;
//! the anchor, profile, issuer link and validity of the time-stamping chain
//! belong with certificates and trust, so a chain that misses the strict
//! profile never reads as an altered document. Rationale: the
//! sections "Time-stamp tokens" and "Trust anchors" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use super::trust;
use crate::certificate::{self, Certificate, Role};
use crate::pem;
use crate::policy::{Policy, Purpose};
use crate::report::{hex, Finding, Outcome, TrustNote};
use crate::timestamp::{self, TokenFacts};

/// Most certificates admitted in a time-stamping chain entry.
const MAX_CHAIN_CERTIFICATES: usize = 8;

/// The time-stamping certificate and the anchor root that issued it, pinned
/// or, with indeterminate trust, served but not pinned.
pub(super) struct TsaChain {
    pub signer: Certificate,
    pub root: Certificate,
}

/// The result of the document token check.
pub(super) struct TokenCheck {
    /// Findings about the token itself, for the document token component.
    pub findings: Vec<Finding>,
    /// Findings about the time-stamping chain, for the certificates and
    /// trust component.
    pub chain: Vec<Finding>,
    /// The verified facts, when the token passed every check that yields
    /// them.
    pub facts: Option<TokenFacts>,
    /// The time-stamping chain, when the token chains to an anchor root.
    pub tsa: Option<TsaChain>,
    pub notes: Vec<TrustNote>,
}

/// Where anchor candidates come from; the token itself never supplies one.
pub(super) struct Candidates<'a> {
    /// The `tsa-chain.pem` entry, when the package has one; then it is the
    /// only source of candidates.
    pub chain_pem: Option<&'a [u8]>,
    /// The decoded `ca.pem`, the documented anchor of a server seal package
    /// without `tsa-chain.pem`; ignored when `chain_pem` is present, as it
    /// always is in individual signing.
    pub issuer: Option<&'a Certificate>,
}

/// Check the token `tsr` as a time-stamp over `covered`.
pub(super) fn check(
    tsr: &[u8],
    covered: &[u8; 32],
    candidates: &Candidates<'_>,
    policy: &Policy,
) -> TokenCheck {
    let mut result = TokenCheck {
        findings: Vec::new(),
        chain: Vec::new(),
        facts: None,
        tsa: None,
        notes: Vec::new(),
    };
    let pool: Vec<Certificate> = match candidates.chain_pem {
        Some(chain) => decode_chain(chain).unwrap_or_else(|finding| {
            result.chain.push(finding);
            Vec::new()
        }),
        None => candidates.issuer.into_iter().cloned().collect(),
    };
    let token = match timestamp::parse(tsr) {
        Ok(token) => token,
        Err(finding) => {
            result.findings.push(finding);
            return result;
        }
    };
    let signer = token.signer_certificate();
    let Some(root) = anchor(signer, &pool, policy, &mut result) else {
        return result;
    };
    for (certificate, role, what) in [
        (&root, Role::Root, "time-stamping root"),
        (signer, Role::TimeStamping, "time-stamping certificate"),
    ] {
        if let Err(finding) = certificate::check_profile(certificate, role) {
            result.chain.push(super::in_entry(what, finding));
        }
    }
    if let Err(finding) = certificate::check_issued_by(signer, &root) {
        result
            .chain
            .push(super::in_entry("time-stamping certificate", finding));
    }
    let admitted: Vec<&str> = policy
        .admitted_policies
        .iter()
        .map(String::as_str)
        .collect();
    match token.verify(covered, &admitted) {
        Ok(facts) => {
            result.findings.push(Finding::new(
                Outcome::Checked,
                "token.verified",
                format!(
                    "genTime {} accuracy {} s, policy {}, ESS v1 {}, ESS v2 {}",
                    facts.gen_time,
                    facts.accuracy_seconds,
                    facts.policy,
                    facts.ess_v1,
                    facts.ess_v2
                ),
            ));
            result.chain.push(validity(signer, &root, &facts));
            result.facts = Some(facts);
        }
        Err(finding) => result.findings.push(finding),
    }
    result.tsa = Some(TsaChain {
        signer: signer.clone(),
        root,
    });
    result
}

/// The certificates of `tsa-chain.pem`. More than
/// [`MAX_CHAIN_CERTIFICATES`] is a limit of this verifier, not a defect of
/// the evidence, so it is not supported; any other PEM defect is failed.
fn decode_chain(bytes: &[u8]) -> Result<Vec<Certificate>, Finding> {
    let blocks = pem::decode_all(bytes, "CERTIFICATE", MAX_CHAIN_CERTIFICATES).map_err(
        |error| match error {
            pem::PemError::TooManyBlocks => Finding::not_supported(
                error.code(),
                format!("tsa-chain.pem holds more than {MAX_CHAIN_CERTIFICATES} certificates"),
            ),
            _ => Finding::failed(error.code(), "tsa-chain.pem"),
        },
    )?;
    blocks
        .iter()
        .map(|der| super::der_certificate("tsa-chain.pem", der))
        .collect()
}

/// The root among `pool` that names and identifies the issuer of `signer`:
/// a pinned one, otherwise one the served trust returns for time-stamping
/// with an indeterminate trust finding, recording why there is none.
fn anchor(
    signer: &Certificate,
    pool: &[Certificate],
    policy: &Policy,
    result: &mut TokenCheck,
) -> Option<Certificate> {
    let issuers: Vec<&Certificate> = pool
        .iter()
        .filter(|candidate| {
            candidate.der != signer.der
                && candidate.subject == signer.issuer
                && candidate.subject_key_id.is_some()
                && candidate.subject_key_id == signer.authority_key_id
        })
        .collect();
    let pinned = issuers.iter().find_map(|candidate| {
        let fingerprint = certificate::sha256(&candidate.der);
        policy
            .pinned_for(&fingerprint, Purpose::TimeStamping)
            .map(|pin| (*candidate, pin.label.clone()))
    });
    if let Some((root, label)) = pinned {
        result.chain.push(Finding::new(
            Outcome::Checked,
            "token.anchor_pinned",
            format!("{label} for time-stamping"),
        ));
        return Some(root.clone());
    }
    let mut seen: Vec<[u8; 32]> = Vec::new();
    for candidate in &issuers {
        let fingerprint = certificate::sha256(&candidate.der);
        if !seen.contains(&fingerprint) {
            seen.push(fingerprint);
            result.notes.push(TrustNote {
                code: "trust.token_anchor_not_pinned",
                purpose: Some(Purpose::TimeStamping),
                fingerprint: hex(&fingerprint),
                detail: format!(
                    "{} issued the token signer but is not pinned",
                    candidate.subject
                ),
            });
        }
    }
    let served = issuers.iter().find(|candidate| {
        trust::served_for(
            policy,
            &certificate::sha256(&candidate.der),
            Purpose::TimeStamping,
        )
    });
    if let Some(root) = served {
        result.chain.push(trust::admit_root(
            root,
            Purpose::TimeStamping,
            policy,
            "time-stamping root",
        ));
        return Some((*root).clone());
    }
    result.findings.push(Finding::not_supported(
        "token.anchor_not_pinned",
        format!("no root pinned for time-stamping issued {}", signer.subject),
    ));
    None
}

/// The time-stamping certificate and its root must be valid over the whole
/// interval `genTime` plus or minus the accuracy.
fn validity(signer: &Certificate, root: &Certificate, facts: &TokenFacts) -> Finding {
    let (from, to) = super::token_interval(facts);
    let outside = [signer, root]
        .iter()
        .any(|certificate| !certificate::valid_over(certificate, from, to));
    if outside {
        Finding::failed(
            "cert.outside_validity",
            format!("time-stamping chain at token genTime {from}..={to}"),
        )
    } else {
        Finding::new(
            Outcome::Checked,
            "cert.validity",
            format!("time-stamping chain at token genTime {from}..={to}"),
        )
    }
}
