# Instrucciones de verificación independiente

Este paquete de evidencia permite verificar el documento `{{DOCUMENTO}}`
sin el software que lo generó. Todos los comandos usan únicamente la
herramienta de línea de órdenes `openssl`; el paquete se generó en una
máquina con `{{OPENSSL_VERSION}}`. Ejecute los comandos dentro del
directorio donde extrajo el paquete.

## Contenido del paquete

- `{{DOCUMENTO}}`: el documento original.
- `{{FIRMA}}`: la firma digital separada (RSA PKCS#1 v1.5 sobre SHA-256).
- `{{SELLO}}`: el sello de tiempo RFC 3161 emitido sobre el resumen
  SHA-256 del documento.
- `certificado.pem`: el certificado del firmante.
- `ca.pem`: el certificado de la autoridad emisora, raíz de la confianza
  de este paquete.
- `crl.pem`: la lista de revocación publicada por la autoridad al
  momento de exportar.
- `tsa-chain.pem`: la cadena de certificados de la autoridad de sellado
  de tiempo (presente cuando estaba disponible al exportar).
- `INSTRUCCIONES.md`: este archivo.

## 1. Integridad del documento

Recalcule el resumen SHA-256 del documento:

    openssl dgst -sha256 {{DOCUMENTO}}

Resumen registrado al momento de exportar:

    {{DIGEST_SHA256}}

Ambos valores deben coincidir carácter por carácter. Si difieren, el
documento fue alterado después de la exportación.

## 2. Firma digital

Extraiga la clave pública del certificado del firmante y verifique la
firma separada sobre el documento:

    openssl x509 -in certificado.pem -pubkey -noout > firmante.pub.pem
    openssl dgst -sha256 -verify firmante.pub.pem -signature {{FIRMA}} {{DOCUMENTO}}

La salida esperada es `Verified OK`. Cualquier otra salida significa que
la firma no corresponde al documento y a la clave del certificado.

## 3. Certificado del firmante

Verifique que el certificado del firmante encadena a la autoridad
emisora y consulte su estado de revocación contra la lista incluida:

    cat ca.pem crl.pem > ca-y-crl.pem
    openssl verify -crl_check -CAfile ca-y-crl.pem certificado.pem

La salida esperada es `certificado.pem: OK`. Tenga presente que la
lista de revocación refleja el estado al momento de la exportación y
tiene fecha de caducidad; para conocer el estado actual solicite una
lista vigente a la autoridad emisora. Un certificado hoy expirado o
revocado no invalida por sí solo una firma efectuada mientras el
certificado era válido; esa lectura corresponde a quien valora la
evidencia.

## 4. Sello de tiempo

Verifique que el sello de tiempo cubre este documento y que fue firmado
por una autoridad de sellado que encadena a la raíz incluida:

    openssl ts -verify -data {{DOCUMENTO}} -in {{SELLO}} -CAfile ca.pem

La salida esperada comienza con `Verification: OK`. Para inspeccionar la
fecha y hora aseveradas por la autoridad de sellado:

    openssl ts -reply -in {{SELLO}} -text
