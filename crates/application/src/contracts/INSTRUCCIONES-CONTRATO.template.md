# Verificación independiente de un contrato bilateral

Este paquete ({{PERFIL}}) conserva el PDF contratado, la solicitud de firma
`contrato-solicitud.bin` y la declaración firmada de cada parte. Fue exportado
por Qadra. Ejecute los comandos dentro del directorio extraído; solo requieren
OpenSSL y herramientas estándar.

- Solicitud: `{{SOLICITUD}}`
- SHA-256 de `contrato-solicitud.bin`: `{{SOLICITUD_DIGEST}}`
- SHA-256 de `contrato.pdf`: `{{PDF_DIGEST}}`
- Vencimiento firmado (`expires_at`, segundos Unix): `{{VENCE}}`

Ninguna parte firma los bytes del PDF directamente. Cada parte firma una
declaración binaria de 303 bytes (`parte-N-declaracion.bin`) que contiene el
SHA-256 de la solicitud y el SHA-256 del PDF. Por eso la verificación tiene
dos pasos: comprobar la firma de la declaración y comparar la huella del PDF
que la declaración contiene con el PDF entregado.

La CA, la autoridad de sellado de tiempo y el servidor pertenecen al despacho.
La confianza en `ca.pem` debe establecerse por un canal independiente, por
ejemplo con la hoja de huellas firmada a mano; incluirla en el ZIP no la
convierte en una autoridad confiable.

## 1. Firma de la declaración y de la vinculación del Cliente

Extraiga la clave pública del certificado de la parte y verifique la firma de
su declaración y la de su vinculación (`parte-1-vinculo.bin`, el registro
`DOCBIND1` de 142 bytes con el que la cuenta vinculó ese certificado antes de
firmar):

```
openssl x509 -in parte-1-certificado.pem -pubkey -noout > clave-publica-1.pem
openssl dgst -sha256 -verify clave-publica-1.pem \
  -signature parte-1-declaracion.sig parte-1-declaracion.bin
openssl dgst -sha256 -verify clave-publica-1.pem \
  -signature parte-1-vinculo.sig parte-1-vinculo.bin
```

Ambas verificaciones deben devolver `Verified OK`.

## 2. Certificado del Cliente

Compruebe que el certificado fue emitido por `ca.pem` y que no figura en la
CRL incluida, y que la raíz de la vinculación es la misma `ca.pem`:

```
openssl verify -CAfile ca.pem -crl_check -CRLfile parte-1-crl.pem parte-1-certificado.pem
openssl x509 -in ca.pem -outform DER | openssl dgst -sha256
openssl dgst -sha256 parte-1-vinculo-raiz.der
```

La primera orden debe devolver `parte-1-certificado.pem: OK`; las dos huellas
deben coincidir. `openssl verify` evalúa la vigencia en la fecha actual y puede
rechazar una CRL o un certificado hoy caducados; para evaluarla en el instante
de la firma añada `-attime` con ese instante en segundos Unix.

## 3. Huella del PDF dentro de la declaración

Los 32 bytes desde el desplazamiento 206 de la declaración son el SHA-256 del
PDF; deben coincidir con la huella calculada:

```
dd if=parte-1-declaracion.bin bs=1 skip=206 count=32 | xxd -p -c 32
sha256sum contrato.pdf
```

Los 32 bytes desde el desplazamiento 174 son el SHA-256 de la solicitud:

```
dd if=parte-1-declaracion.bin bs=1 skip=174 count=32 | xxd -p -c 32
sha256sum contrato-solicitud.bin
```

## 4. Sello de tiempo de la firma del Cliente (T1)

```
openssl dgst -sha256 -binary parte-1-declaracion.sig > firma-1.sha256
openssl ts -verify -in parte-1-declaracion.sig.tsr -digest \
  "$(xxd -p -c 32 firma-1.sha256)" -CAfile ca.pem -untrusted parte-1-tsa-chain.pem
```

{{COMPLETO}}
## Lectura de la declaración antes de firmar

La declaración de 303 bytes empieza con `CTRTSIG1`. Antes de firmarla por la
ruta externa, compruebe con `xxd parte-N-declaracion.bin` la solicitud
(desplazamiento 158), la huella de la solicitud (174), la del PDF (206) y la
posición (238: 1 Cliente, 2 despacho). En la posición 2, los desplazamientos
239 y 271 contienen el SHA-256 de `parte-1-declaracion.sig` y de
`parte-1-declaracion.sig.tsr`: la firma del despacho declara qué firma y qué
sello del Cliente vio.

Los archivos JSON de recibo o acuse son registros de la aplicación; ningún
sello los cubre y no deciden ningún resultado. El rechazo, la cancelación y el
vencimiento tampoco aparecen en el paquete.
