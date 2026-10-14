//! Restricted public-material worker used by the CA executor.
use std::io::Read;

use domain::crypto::certificate::CertificateRequestProfile;
use infrastructure::certificates::{
    inspect_ca_certificate, inspect_ca_inventory_certificate, inspect_ca_request,
    CaInventoryPurpose,
};
use serde::Deserialize;
use serde_json::json;
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    kind: Kind,
    material_pem: String,
    root_pem: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    Request,
    Certificate,
    InventoryCertificate,
}

pub fn run() -> anyhow::Result<()> {
    let mut input = Zeroizing::new(Vec::new());
    std::io::stdin().take(65537).read_to_end(&mut input)?;
    if input.len() > 65536 {
        anyhow::bail!("public inspection input is too large");
    }
    let request: Request = serde_json::from_slice(&input)
        .map_err(|_| anyhow::anyhow!("public inspection input is invalid"))?;
    let root = request.root_pem.as_bytes();
    let material = request.material_pem.as_bytes();
    let result = match request.kind {
        Kind::Request => {
            let found = inspect_ca_request(material, root)?;
            json!({"csr_sha256":found.csr_sha256.to_hex(),
                "spki_sha256":found.spki_sha256.to_hex(),
                "root_fingerprint":found.root_fingerprint.to_hex(), "subject":found.subject,
                "issuer_policy_matches":found.issuer_policy_matches})
        }
        Kind::Certificate => {
            let found = inspect_ca_certificate(material, root)
                .map_err(|_| anyhow::anyhow!("public certificate inspection failed"))?;
            let purpose = match found.purpose {
                CertificateRequestProfile::Partner => "partner",
                CertificateRequestProfile::DocumentSigning => "document-signing",
            };
            json!({"serial":found.serial,"fingerprint":found.fingerprint.to_hex(),
                "spki_sha256":found.spki_sha256.to_hex(),
                "root_fingerprint":found.root_fingerprint.to_hex(),"purpose":purpose,
                "subject":found.subject,"not_before":found.not_before,"not_after":found.not_after})
        }
        Kind::InventoryCertificate => {
            let found = inspect_ca_inventory_certificate(material, root)
                .map_err(|_| anyhow::anyhow!("public inventory certificate inspection failed"))?;
            let purpose = match found.purpose {
                CaInventoryPurpose::Partner => "partner",
                CaInventoryPurpose::DocumentSigning => "document-signing",
                CaInventoryPurpose::TimeStamping => "time-stamping",
            };
            json!({"serial":found.serial,"fingerprint":found.fingerprint.to_hex(),
                "spki_sha256":found.spki_sha256.to_hex(),
                "root_fingerprint":found.root_fingerprint.to_hex(),"purpose":purpose,
                "subject":found.subject,"not_before":found.not_before,"not_after":found.not_after})
        }
    };
    println!("{result}");
    Ok(())
}
