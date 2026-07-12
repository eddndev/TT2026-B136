# Transcripción de la demostración (`scripts/demo.sh`)

Ejecución real de la demostración de extremo a extremo, capturada el
12 de julio de 2026 sobre `OpenSSL 3.5.7 9 Jun 2026` con el binario
compilado `target/debug/despacho-cli`.

Recortes aplicados para legibilidad, sin alterar ninguna salida de los
comandos:

- la raíz del repositorio se abrevia como `$REPO` y el directorio
  temporal de trabajo como `$WORK`;
- se eliminaron las líneas de diagnóstico (`INFO ... command received`)
  que el binario emite por `stderr` en cada invocación;
- se eliminaron las secuencias de color de la terminal.

Los secretos que aparecen (llaves, secreto TOTP, códigos de
recuperación) se generaron dentro del directorio temporal de esta
ejecución y se destruyeron al terminar; no protegen nada.

```text
=====================================================================
== Build the binary once
=====================================================================
+ cargo build --workspace --manifest-path $REPO/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s

=====================================================================
== Reproducibility banner
=====================================================================
+ openssl version
OpenSSL 3.5.7 9 Jun 2026 (Library: OpenSSL 3.5.7 9 Jun 2026)
+ $REPO/target/debug/despacho-cli --version
despacho-cli 0.1.0

=====================================================================
== Create the internal CA, the signer, and the timestamp authority
=====================================================================
+ $REPO/target/debug/despacho-cli pki --scripts-dir $REPO/pki init-ca
certificate authority ready at $WORK/pki-ca
root certificate: $WORK/pki-ca/ca.crt.pem
issued certificate serial 1000
  certificate: $WORK/pki-ca/certs/socia-demo.crt.pem
  private key: $WORK/pki-ca/private/socia-demo.key.pem (keep secret, never commit)
+ bash $REPO/pki/issue-tsa-cert.sh
Using configuration from $REPO/pki/openssl.cnf
Check that the request matches the signature
Signature ok
The Subject's Distinguished Name is as follows
countryName           :PRINTABLE:'MX'
stateOrProvinceName   :ASN.1 12:'Ciudad de Mexico'
localityName          :ASN.1 12:'Ciudad de Mexico'
organizationName      :ASN.1 12:'Despacho Juridico Demo'
organizationalUnitName:ASN.1 12:'Autoridad de Sellado de Tiempo'
commonName            :ASN.1 12:'TSA Interna Despacho Juridico Demo'
Certificate is to be certified until Jul 12 10:19:41 2027 GMT (365 days)

Write out database with 1 new entries
Database updated

TSA certificate issued successfully.
  TSA directory: $WORK/pki-tsa
  Certificate:   $WORK/pki-tsa/tsa.crt.pem
  Private key:   $WORK/pki-tsa/private/tsa.key.pem (keep secret, never commit)
  Chain file:    $WORK/pki-tsa/tsa-chain.pem
  Serial file:   $WORK/pki-tsa/serial
  subject=C=MX, ST=Ciudad de Mexico, L=Ciudad de Mexico, O=Despacho Juridico Demo, OU=Autoridad de Sellado de Tiempo, CN=TSA Interna Despacho Juridico Demo
  serial=1001
  notBefore=Jul 12 10:14:41 2026 GMT
  notAfter=Jul 12 10:19:41 2027 GMT
+ $REPO/target/debug/despacho-cli pki --scripts-dir $REPO/pki gen-crl
wrote $WORK/pki-ca/crl/crl.pem

=====================================================================
== Hash the sample document
=====================================================================
+ $REPO/target/debug/despacho-cli crypto hash document.txt
fb5c0a6097be9246860fa82c89db3576e8a4ba7e554f3954f148f8bdfdd27ad9

=====================================================================
== Encrypt at rest, corrupt one byte, observe rejection
=====================================================================
+ $REPO/target/debug/despacho-cli vault encrypt document.txt --doc-id 00000000-0000-4000-8000-000000000001 --version 1
wrote document.txt.enc
+ $REPO/scripts/flip-byte.sh document.txt.enc 512
flipped byte at offset 512 of document.txt.enc: 0x45 -> 0xba (size 3678 bytes, unchanged)
+ (expected to fail) $REPO/target/debug/despacho-cli vault decrypt document.txt.enc --doc-id 00000000-0000-4000-8000-000000000001 --out rechazado.txt
Error: decryption of document.txt.enc failed

Caused by:
    authenticated decryption failed
OK: command was rejected, as expected
+ $REPO/scripts/flip-byte.sh document.txt.enc 512
flipped byte at offset 512 of document.txt.enc: 0xba -> 0x45 (size 3678 bytes, unchanged)
+ $REPO/target/debug/despacho-cli vault decrypt document.txt.enc --doc-id 00000000-0000-4000-8000-000000000001 --out intacto.txt
wrote intacto.txt
+ cmp document.txt intacto.txt
+ $REPO/target/debug/despacho-cli vault rotate-kek --file document.txt.enc
rewrapped the data key in document.txt.enc
+ $REPO/target/debug/despacho-cli vault decrypt document.txt.enc --doc-id 00000000-0000-4000-8000-000000000001 --out rotado.txt
wrote rotado.txt
+ cmp document.txt rotado.txt

=====================================================================
== Sign the document and obtain a trusted timestamp
=====================================================================
+ $REPO/target/debug/despacho-cli sign document.txt --cert $WORK/pki-ca/certs/socia-demo.crt.pem --key $WORK/pki-ca/private/socia-demo.key.pem
digest: fb5c0a6097be9246860fa82c89db3576e8a4ba7e554f3954f148f8bdfdd27ad9
signature: document.txt.sig
+ $REPO/target/debug/despacho-cli timestamp document.txt --mock --pki-dir $REPO/pki
digest: fb5c0a6097be9246860fa82c89db3576e8a4ba7e554f3954f148f8bdfdd27ad9
token: document.txt.tsr
generated at: 2026-07-12T10:19:41Z

=====================================================================
== Integral verification: all components valid
=====================================================================
+ $REPO/target/debug/despacho-cli verify document.txt --sig document.txt.sig --tsr document.txt.tsr --cert $WORK/pki-ca/certs/socia-demo.crt.pem --ca $WORK/pki-ca/ca.crt.pem --crl $WORK/pki-ca/crl/crl.pem
document:    document.txt
digest:      fb5c0a6097be9246860fa82c89db3576e8a4ba7e554f3954f148f8bdfdd27ad9
integrity:   passed: recomputed digest matches the digest the timestamp token attests
signature:   passed: signature verifies over the document digest with the signer certificate
certificate: passed: status at the evaluation time: valid (revocation checked against the supplied list)
timestamp:   passed: token accepted under the trust anchor; generated at 2026-07-12T10:19:41Z
verdict:     valid

=====================================================================
== Tamper with a copy of the document: the signature component fails
=====================================================================
+ $REPO/scripts/flip-byte.sh alterado.txt 100
flipped byte at offset 100 of alterado.txt: 0x61 -> 0x9e (size 3581 bytes, unchanged)
+ (expected to fail) $REPO/target/debug/despacho-cli verify alterado.txt --sig document.txt.sig --tsr document.txt.tsr --cert $WORK/pki-ca/certs/socia-demo.crt.pem --ca $WORK/pki-ca/ca.crt.pem --crl $WORK/pki-ca/crl/crl.pem
document:    alterado.txt
digest:      19914d8748bac2b9796d6ac6b1d49b263767280c0bec6087f26e54273667dbb4
integrity:   failed: recomputed digest differs from the digest the timestamp token attests
signature:   failed: signature rejected: signature does not match document and key
certificate: passed: status at the evaluation time: valid (revocation checked against the supplied list)
timestamp:   failed: token does not cover the recomputed document digest
verdict:     not valid
Error: the verification verdict is not valid
OK: command was rejected, as expected

=====================================================================
== Export the evidence package and verify it with openssl alone
=====================================================================
+ $REPO/target/debug/despacho-cli package export document.txt --sig document.txt.sig --tsr document.txt.tsr --cert $WORK/pki-ca/certs/socia-demo.crt.pem --ca $WORK/pki-ca/ca.crt.pem --crl $WORK/pki-ca/crl/crl.pem --tsa-chain $WORK/pki-tsa/tsa-chain.pem --out evidencia.zip
digest:  fb5c0a6097be9246860fa82c89db3576e8a4ba7e554f3954f148f8bdfdd27ad9
package: evidencia.zip
instructions inside the package: INSTRUCCIONES.md
+ unzip -o evidencia.zip -d evidencia
Archive:  evidencia.zip
 extracting: evidencia/document.txt
 extracting: evidencia/document.txt.sig
 extracting: evidencia/document.txt.tsr
 extracting: evidencia/certificado.pem
 extracting: evidencia/ca.pem
 extracting: evidencia/crl.pem
 extracting: evidencia/tsa-chain.pem
 extracting: evidencia/INSTRUCCIONES.md
+ head of evidencia/INSTRUCCIONES.md
# Instrucciones de verificación independiente

Este paquete de evidencia permite verificar el documento `document.txt`
sin el software que lo generó. Todos los comandos usan únicamente la
herramienta de línea de órdenes `openssl`; el paquete se generó en una
máquina con `OpenSSL 3.5.7 9 Jun 2026 (Library: OpenSSL 3.5.7 9 Jun 2026)`. Ejecute los comandos dentro del
directorio donde extrajo el paquete.

+ openssl dgst -sha256 document.txt
SHA2-256(document.txt)= fb5c0a6097be9246860fa82c89db3576e8a4ba7e554f3954f148f8bdfdd27ad9
+ openssl x509 -in certificado.pem -pubkey -noout -out firmante.pub.pem
+ openssl dgst -sha256 -verify firmante.pub.pem -signature document.txt.sig document.txt
Verified OK
+ openssl verify -crl_check -CAfile ca-y-crl.pem certificado.pem
certificado.pem: OK
+ openssl ts -verify -data document.txt -in document.txt.tsr -CAfile ca.pem
Using configuration from /etc/pki/tls/openssl.cnf
Verification: OK

=====================================================================
== Revoke the certificate: verification reports the revoked status
=====================================================================
+ $REPO/target/debug/despacho-cli pki --scripts-dir $REPO/pki revoke --serial 1000
revoked certificate serial 1000
run 'pki gen-crl' to publish an updated revocation list
+ $REPO/target/debug/despacho-cli pki --scripts-dir $REPO/pki gen-crl
wrote $WORK/pki-ca/crl/crl.pem
+ (expected to fail) $REPO/target/debug/despacho-cli verify document.txt --sig document.txt.sig --tsr document.txt.tsr --cert $WORK/pki-ca/certs/socia-demo.crt.pem --ca $WORK/pki-ca/ca.crt.pem --crl $WORK/pki-ca/crl/crl.pem
document:    document.txt
digest:      fb5c0a6097be9246860fa82c89db3576e8a4ba7e554f3954f148f8bdfdd27ad9
integrity:   passed: recomputed digest matches the digest the timestamp token attests
signature:   passed: signature verifies over the document digest with the signer certificate
certificate: failed: status at the evaluation time: revoked (serial 1000); this status speaks for the evaluation time only and does not by itself invalidate a signature made while the certificate was valid
timestamp:   passed: token accepted under the trust anchor; generated at 2026-07-12T10:19:41Z
verdict:     not valid
Error: the verification verdict is not valid
OK: command was rejected, as expected

=====================================================================
== Authentication: hashing cost and one-time passwords
=====================================================================
+ $REPO/target/debug/despacho-cli auth calibrate
mean over 5 runs: 37.8 ms (below the 500-1000 ms target band)
+ $REPO/target/debug/despacho-cli auth totp enroll --user demo@example.com --secret-out totp-secret.b32
otpauth-uri: otpauth://totp/despacho:demo%40example.com?secret=IGAPAF2O2XDMIG6QGFZU2YITPDCD44P3&issuer=despacho
secret-base32: IGAPAF2O2XDMIG6QGFZU2YITPDCD44P3
recovery codes (each works exactly once):
  GQVMH-DCWJS
  9QBY1-NXNJ3
  MY4P3-CX4HZ
  0CE8T-KBNSS
  31D0P-GDAD8
  B6B6N-DBGAJ
  ZV2T3-B35JJ
  C329Q-TTBEE
secret written to totp-secret.b32
+ despacho-cli auth totp verify --code (computed with openssl)
accepted

=====================================================================
== Audit chain: append, verify, tamper, verify again
=====================================================================
+ $REPO/target/debug/despacho-cli audit append --action demo.firma --resource document.txt.sig
appended entry 0 with chain e8b3fefc3966e47a695260f2b098dd73eff0f7da9e9b4692314df873b1e083fa
+ $REPO/target/debug/despacho-cli audit append --action demo.sello --resource document.txt.tsr
appended entry 1 with chain ffc1a151ecc7e5976d24b0bbfaaaed4c9230a8ed597938e24e6c228b2a0a5581
+ $REPO/target/debug/despacho-cli audit verify-chain
audit chain valid (2 entries)
+ $REPO/target/debug/despacho-cli audit show
0 2026-07-12T10:19:42.272903635Z eddndev demo.firma document.txt.sig e8b3fefc3966e47a695260f2b098dd73eff0f7da9e9b4692314df873b1e083fa
1 2026-07-12T10:19:42.275499975Z eddndev demo.sello document.txt.tsr ffc1a151ecc7e5976d24b0bbfaaaed4c9230a8ed597938e24e6c228b2a0a5581
+ sed -i s/demo.sello/demo.XXXXX/ $WORK/audit-log.jsonl
+ (expected to fail) $REPO/target/debug/despacho-cli audit verify-chain
Error: audit chain broken at index 1
OK: command was rejected, as expected

=====================================================================
== Demo complete
=====================================================================
All stages finished. Working files lived in $WORK and are
removed on exit.
```
