//! The fixed entry names of both contract package profiles, shared by the
//! manifest, the profile recognition and the checks. Rationale: the section
//! "Evidence packages" of `docs/adr/0086-bilateral-contract-signing.md`.

use super::ContractRole;

/// Package entry holding the contracted PDF.
pub const DOCUMENT_ENTRY: &str = "contrato.pdf";
/// Package entry holding the time-stamp response Td over the PDF digest.
pub const DOCUMENT_TOKEN_ENTRY: &str = "contrato.pdf.tsr";
/// Package entry holding the 240 request bytes.
pub const REQUEST_ENTRY: &str = "contrato-solicitud.bin";
/// Package entry holding the root of both parties and of the authority.
pub const ROOT_ENTRY: &str = "ca.pem";
/// Package entry holding the anchor candidates of Td and Tf.
pub const TSA_CHAIN_ENTRY: &str = "tsa-chain.pem";
/// Package entry holding the time-stamp response T1 over the position 1
/// signature digest.
pub const STATEMENT_TOKEN_ENTRY: &str = "parte-1-declaracion.sig.tsr";
/// Package entry holding the anchor candidates of T1.
pub const STATEMENT_TSA_CHAIN_ENTRY: &str = "parte-1-tsa-chain.pem";

/// The eight entries of one party, `parte-N-` for position N.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyEntries {
    pub statement: &'static str,
    pub statement_signature: &'static str,
    pub certificate: &'static str,
    pub crl: &'static str,
    pub binding_statement: &'static str,
    pub binding_signature: &'static str,
    pub binding_root: &'static str,
    pub binding_crl: &'static str,
}

impl PartyEntries {
    /// Each member name of a signer's `entries`, its role and the entry.
    pub fn members(&self) -> [(&'static str, ContractRole, &'static str); 8] {
        [
            ("statement", ContractRole::Statement, self.statement),
            (
                "statement_signature",
                ContractRole::StatementSignature,
                self.statement_signature,
            ),
            (
                "signer_certificate",
                ContractRole::SignerCertificate,
                self.certificate,
            ),
            ("issuer_crl", ContractRole::IssuerCrl, self.crl),
            (
                "binding_statement",
                ContractRole::BindingStatement,
                self.binding_statement,
            ),
            (
                "binding_signature",
                ContractRole::BindingSignature,
                self.binding_signature,
            ),
            ("binding_root", ContractRole::BindingRoot, self.binding_root),
            ("binding_crl", ContractRole::BindingCrl, self.binding_crl),
        ]
    }
}

/// The entries of position 1 and position 2, in that order.
pub const PARTY_ENTRIES: [PartyEntries; 2] = [
    PartyEntries {
        statement: "parte-1-declaracion.bin",
        statement_signature: "parte-1-declaracion.sig",
        certificate: "parte-1-certificado.pem",
        crl: "parte-1-crl.pem",
        binding_statement: "parte-1-vinculo.bin",
        binding_signature: "parte-1-vinculo.sig",
        binding_root: "parte-1-vinculo-raiz.der",
        binding_crl: "parte-1-vinculo-crl.der",
    },
    PartyEntries {
        statement: "parte-2-declaracion.bin",
        statement_signature: "parte-2-declaracion.sig",
        certificate: "parte-2-certificado.pem",
        crl: "parte-2-crl.pem",
        binding_statement: "parte-2-vinculo.bin",
        binding_signature: "parte-2-vinculo.sig",
        binding_root: "parte-2-vinculo-raiz.der",
        binding_crl: "parte-2-vinculo-crl.der",
    },
];
