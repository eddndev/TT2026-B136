use std::{fs, path::Path, process::Command, sync::OnceLock};

use der::{
    asn1::{BitString, OctetString, Uint},
    oid::AssociatedOid,
    Decode, Encode,
};
use domain::crypto::{
    CredentialTrustInspection, DocumentHasher, DocumentSigner, InternalDeclarationVerifier,
};
use infrastructure::{
    certificates::InternalRsaDeclarationVerifier, RingSha256Hasher, RsaPkcs1Signer,
};
use x509_cert::{crl::CertificateList, ext::pkix::CrlNumber, time::Time};
use zeroize::Zeroizing;

pub struct Materials {
    pub directory: tempfile::TempDir,
    pub root: Vec<u8>,
    pub crl: CertificateList,
    pub at: i64,
}
impl Materials {
    pub fn inspection(&self, number: u64, start: i64, end: i64) -> CredentialTrustInspection {
        let mut crl = self.crl.clone();
        let number = CrlNumber(Uint::new(&number.to_be_bytes()).unwrap());
        let extension = crl
            .tbs_cert_list
            .crl_extensions
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|ext| ext.extn_id == CrlNumber::OID)
            .unwrap();
        extension.extn_value = OctetString::new(number.to_der().unwrap()).unwrap();
        crl.tbs_cert_list.this_update = time(self.at + start);
        crl.tbs_cert_list.next_update = Some(time(self.at + end));
        let key =
            Zeroizing::new(fs::read(self.directory.path().join("private/ca.key.pem")).unwrap());
        let signature = RsaPkcs1Signer::new(key)
            .unwrap()
            .sign(&RingSha256Hasher.hash_bytes(&crl.tbs_cert_list.to_der().unwrap()))
            .unwrap();
        crl.signature = BitString::from_bytes(signature.as_bytes()).unwrap();
        InternalRsaDeclarationVerifier::new()
            .inspect_trust(&self.root, &crl.to_der().unwrap(), self.at)
            .unwrap()
    }
}
fn time(at: i64) -> Time {
    Time::UtcTime(
        der::asn1::UtcTime::from_unix_duration(std::time::Duration::from_secs(at as u64)).unwrap(),
    )
}
pub fn materials() -> &'static Materials {
    static MATERIALS: OnceLock<Materials> = OnceLock::new();
    MATERIALS.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        for script in ["init-ca.sh", "gen-crl.sh"] {
            let output = Command::new("bash")
                .arg(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../pki")
                        .join(script),
                )
                .env("PKI_CA_DIR", directory.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let root = fs::read(directory.path().join("ca.crt.pem")).unwrap();
        let (_, der) =
            der::pem::decode_vec(&fs::read(directory.path().join("crl/crl.pem")).unwrap()).unwrap();
        let crl = CertificateList::from_der(&der).unwrap();
        let at = crl.tbs_cert_list.this_update.to_unix_duration().as_secs() as i64 + 3600;
        Materials {
            directory,
            root,
            crl,
            at,
        }
    })
}
