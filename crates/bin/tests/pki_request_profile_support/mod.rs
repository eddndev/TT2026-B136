use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn scripts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

pub fn pki(ca: &Path, directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .arg("pki")
        .arg("--scripts-dir")
        .arg(directory)
        .args(args)
        .env("PKI_CA_DIR", ca)
        .output()
        .unwrap()
}

pub fn success(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

pub fn openssl(args: &[&str]) -> Vec<u8> {
    success(Command::new("openssl").args(args).output().unwrap())
}

pub fn request(directory: &Path) -> PathBuf {
    fs::create_dir(directory).unwrap();
    let key = directory.join("holder.key.pem");
    let csr = directory.join("holder.csr.pem");
    openssl(&[
        "genpkey",
        "-quiet",
        "-algorithm",
        "RSA",
        "-pkeyopt",
        "rsa_keygen_bits:3072",
        "-pkeyopt",
        "rsa_keygen_pubexp:65537",
        "-out",
        key.to_str().unwrap(),
    ]);
    openssl(&[
        "req",
        "-new",
        "-sha256",
        "-key",
        key.to_str().unwrap(),
        "-subj",
        "/C=MX/O=Despacho Juridico Demo/CN=External Profile Holder",
        "-addext",
        "basicConstraints=critical,CA:TRUE",
        "-addext",
        "keyUsage=critical,keyCertSign,cRLSign",
        "-addext",
        "extendedKeyUsage=serverAuth",
        "-addext",
        "subjectAltName=DNS:untrusted.example",
        "-out",
        csr.to_str().unwrap(),
    ]);
    csr
}

pub fn issue(ca: &Path, directory: &Path, csr: &Path, name: &str, purpose: Option<&str>) -> Output {
    let mut args = vec![
        "issue-request",
        "--csr",
        csr.to_str().unwrap(),
        "--name",
        name,
        "--json",
    ];
    if let Some(purpose) = purpose {
        args.extend(["--purpose", purpose]);
    }
    pki(ca, directory, &args)
}

pub fn issued_path(output: Output) -> PathBuf {
    let body: serde_json::Value = serde_json::from_slice(&success(output)).unwrap();
    let object = body.as_object().unwrap();
    assert_eq!(object.len(), 2, "only the public result is returned");
    assert!(object.get("private_key").is_none());
    assert!(body["serial"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    PathBuf::from(body["certificate"].as_str().unwrap())
}

pub fn state(directory: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(directory, directory, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

pub fn public_key(program: &str, path: &Path) -> Vec<u8> {
    openssl(&[program, "-in", path.to_str().unwrap(), "-pubkey", "-noout"])
}

pub fn replacement_script(directory: &Path, certificate: &Path) {
    fs::create_dir(directory).unwrap();
    fs::copy(certificate, directory.join("replacement.pem")).unwrap();
    fs::write(
        directory.join("issue-request.sh"),
        "#!/usr/bin/env bash\nset -euo pipefail\n\
         cp \"$(dirname \"$0\")/replacement.pem\" \"$PKI_CA_DIR/certs/$1.crt.pem\"\n",
    )
    .unwrap();
}
