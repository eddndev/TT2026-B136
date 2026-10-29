# Verificación independiente de firma documental individual

El paquete conserva el PDF original, su firma separada, el token RFC 3161,
el certificado del titular y la confianza capturada. Añade las pruebas públicas
de vinculación de cuenta y consentimiento. Fue exportado con
`{{OPENSSL_VERSION}}`. Ejecute los comandos dentro del directorio extraído.

Los certificados y CRL incluidos permiten reproducir la verificación bajo la
CA interna declarada. La confianza en esa CA debe establecerse por un canal independiente:
incluir una raíz en el ZIP no la convierte en una autoridad confiable para quien
recibe el paquete. La identidad de cuenta proviene de los bytes canónicos
firmados y del registro auditado del sistema; no se deduce del `subject`, correo
o nombre del certificado y no acredita por sí sola identidad civil.

## Archivos

- `{{DOCUMENTO}}`, `{{FIRMA}}` y `{{SELLO}}`: PDF, firma RSA PKCS#1 v1.5/SHA-256
  sobre su contenido y token RFC 3161 sobre el digest del PDF.
- `certificado.pem`, `ca.pem` y `crl.pem`: certificado y confianza capturados
  al aceptar la firma. `tsa-chain.pem` contiene la cadena TSA cuando estaba disponible.
- `certificate-binding.bin` y `certificate-binding.sig`: declaración canónica
  de registro y prueba de posesión externa.
- `binding-root.der` y `binding-crl.der`: confianza capturada al registrar el vínculo.
- `document-consent.bin` y `document-consent.sig`: consentimiento canónico
  para la cuenta, operación y versión exactas, con la huella de la firma del PDF.
- `individual-signing.json`: copia pública de las declaraciones, firmas,
  confianza de registro y tiempos capturados; formato 1.
- `INSTRUCCIONES.md`: esta guía. El JSON y esta guía facilitan la lectura;
  los bytes firmados y su verificación gobiernan la interpretación.

## 1. Contenido y firma del PDF

    openssl dgst -sha256 {{DOCUMENTO}}
    openssl x509 -in certificado.pem -pubkey -noout > firmante.pub.pem
    openssl dgst -sha256 -verify firmante.pub.pem -signature {{FIRMA}} {{DOCUMENTO}}

El SHA-256 esperado del PDF es `{{DIGEST_SHA256}}`. La firma debe devolver
`Verified OK`. Calcule también la huella de sus bytes:

    openssl dgst -sha256 {{FIRMA}}

Debe ser `{{PDF_FIRMA_DIGEST}}`, el valor ligado por el consentimiento.

## 2. Vinculación y consentimiento

Verifique ambas firmas separadas sobre los archivos binarios exactos:

    openssl dgst -sha256 -verify firmante.pub.pem -signature certificate-binding.sig certificate-binding.bin
    openssl dgst -sha256 -verify firmante.pub.pem -signature document-consent.sig document-consent.bin
    openssl dgst -sha256 certificate-binding.bin
    openssl dgst -sha256 document-consent.bin

Ambas verificaciones deben devolver `Verified OK`. Sus SHA-256 son,
respectivamente, `{{REGISTRO_DIGEST}}` y `{{CONSENT_DIGEST}}`.

Las declaraciones usan enteros sin signo en orden de bytes de red y UUID de
16 bytes. Los rangos siguientes empiezan en cero y excluyen su límite superior.

| Bytes | Campo común |
| --- | --- |
| 0..8 | `DOCBIND1` en el registro; `DOCSIGN1` en el consentimiento |
| 8..10 | Versión 1 y propósito 1, un byte cada uno |
| 10..26 | UUID de despliegue |
| 26..58 | SHA-256 del DER de la raíz |
| 58..62 | Revisión de confianza, u32 |
| 62..78 | UUID de cuenta |
| 78..86 y 86..94 | Revisión de cuenta y generación de autenticación, u64 |
| 94..110 | UUID del vínculo |
| 110..142 | SHA-256 del DER del certificado |

El registro tiene exactamente 142 bytes. El consentimiento tiene 258 bytes:
añade operación (142..158), expediente (158..174), documento (174..190),
versión documental u32 (190..194), digest del PDF (194..226) y digest de su
firma separada (226..258). Verifique que cuenta, despliegue, vínculo y ambas
huellas coincidan entre declaraciones y con el material recibido. Las revisiones
y generaciones no pueden retroceder. Una revisión de confianza igual debe
identificar la misma CRL; una posterior requiere número CRL creciente y
`thisUpdate` no regresivo.

Contexto capturado: cuenta `{{CUENTA}}`, despliegue `{{DESPLIEGUE}}`,
vínculo `{{VINCULO}}`, operación `{{OPERACION}}`, expediente `{{EXPEDIENTE}}`,
documento `{{DOCUMENTO_ID}}`, versión `{{VERSION}}`. La confianza pasa de
revisión `{{REGISTRO_TRUST}}` a `{{FIRMA_TRUST}}`. Confirme estos valores
decodificando los binarios firmados; el texto no los sustituye.

Para cotejar las huellas públicas:

    openssl x509 -in certificado.pem -outform DER | openssl dgst -sha256
    openssl x509 -in ca.pem -outform DER | openssl dgst -sha256
    openssl dgst -sha256 binding-root.der

Las huellas esperadas del certificado y raíz son `{{LEAF_DIGEST}}` y
`{{ROOT_DIGEST}}`. Ambas raíces deben producir la segunda huella.
La revisión de confianza es una referencia del registro auditado: no puede
deducirse únicamente del número de una CRL.

## 3. Vigencia histórica y revocación

Convierta la confianza DER del registro y evalúe el certificado en su instante
capturado, expresado en segundos Unix:

    openssl x509 -inform DER -in binding-root.der -out binding-root.pem
    openssl crl -inform DER -in binding-crl.der -out binding-crl.pem
    cat binding-root.pem binding-crl.pem > binding-ca-and-crl.pem
    openssl verify -attime {{REGISTRO_AT}} -crl_check -CAfile binding-ca-and-crl.pem certificado.pem

Evalúe después la confianza de aceptación en su propio instante:

    cat ca.pem crl.pem > ca-y-crl.pem
    openssl verify -attime {{ACEPTACION_AT}} -crl_check -CAfile ca-y-crl.pem certificado.pem
    openssl crl -in binding-crl.pem -noout -crlnumber -lastupdate -nextupdate
    openssl crl -in crl.pem -noout -crlnumber -lastupdate -nextupdate

Cada verificación debe terminar en `certificado.pem: OK`. Los instantes
`{{REGISTRO_AT}}` y `{{ACEPTACION_AT}}` son capturas del registro auditado de
la aplicación; las firmas canónicas no convierten por sí mismas esos metadatos
en afirmaciones temporales de una TSA. El registro no puede ser posterior a
la aceptación. Sus CRL deben cubrir los respectivos instantes y ambas cadenas
deben alcanzar la misma raíz.

Exportar no actualiza las CRL. Para conocer el estado actual, obtenga de la
autoridad material vigente por un canal independiente y evalúelo por separado.
La expiración, revocación o retirada posterior del vínculo no sustituye las
capturas históricas ni cambia por sí sola el resultado reproducido para ellas.

## 4. Sello de tiempo del contenido

    openssl ts -verify -data {{DOCUMENTO}} -in {{SELLO}} -CAfile {{ANCLA_SELLO}}
    openssl ts -reply -in {{SELLO}} -text

La primera orden debe comenzar con `Verification: OK`. El token liga la huella
del PDF y el tiempo de generación declarado por la TSA configurada. Este sello
no demuestra que la firma personal ya existiera en ese instante: no cubre
la firma separada ni el consentimiento. No confunda su fecha con los instantes
de registro o aceptación auditados.

Una TSA local demuestra el flujo técnico RFC 3161 y no constituye una constancia de un PSC autorizado.
La verificación de firmas y cadenas dentro de esta CA interna tampoco acredita
FIREL, e.firma, identidad civil o una categoría jurídica distinta.
{{MANIFIESTO}}
