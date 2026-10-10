# Firma documental individual en Qadra

El litigante conserva su clave privada fuera de Qadra. La interfaz recibe
un certificado publico PEM y firmas binarias separadas de 384 bytes. Nunca
solicita claves privadas, contrasenas de la clave ni archivos PKCS12.
El perfil es interno y de demostracion; no representa FIREL ni una emision
por un PSC. El contrato esta en `docs/http-individual-signing.md` y la decision
en `docs/adr/0074-individual-document-signatures.md`.

## Certificado y clave local

En el equipo del titular, con OpenSSL y un directorio privado:

```sh
umask 077
openssl genpkey -algorithm RSA -aes-256-cbc -pkeyopt rsa_keygen_bits:3072 \
  -pkeyopt rsa_keygen_pubexp:65537 -out signer-private.pem
openssl req -new -sha256 -key signer-private.pem -out signer.csr.pem \
  -subj '/C=MX/O=Despacho Juridico Demo/CN=Firma documental'
```

Los valores C y O del ejemplo coinciden con la CA de demostracion. Si usas
otra CA configurada, utiliza su pais y organizacion: la politica de emision
exige esa coincidencia.

OpenSSL solicita la contrasena localmente. El administrador de la CA recibe
solamente `signer.csr.pem`, comprueba la identidad por el procedimiento del
despacho y emite con la CA configurada:

```sh
despacho-cli pki issue-request --csr signer.csr.pem --name litigante-demo
```

Este comando verifica la prueba de posesion del CSR, conserva su clave publica
y aplica las extensiones de firma del perfil interno. No genera ni recibe
la clave del titular. El administrador entrega el certificado publico como
`signer-public.pem`. La confianza de la aplicacion debe contener esa CA y una
CRL vigente mediante la administracion existente de confianza.

## Vincular el certificado publico

Con la cuenta Litigator, abre **Mi firma documental**, selecciona
`signer-public.pem` y prepara el vinculo. Revisa tu cuenta, la huella del
certificado, el identificador del vinculo y la revision de confianza.
Descarga `certificate-binding.bin` y, sin modificar sus bytes, ejecuta
localmente:

```sh
openssl dgst -sha256 -sign signer-private.pem \
  -out certificate-binding.sig certificate-binding.bin
```

Carga solamente `certificate-binding.sig`, confirma la vinculacion y registra.
Conserva el recibo publico. El nombre del certificado es informativo: la cuenta
firmante se identifica dentro de los bytes canonicos, no por el CN.

Si una respuesta se pierde, usa **Consultar recibo del vinculo**. El intento
se conserva en memoria para la misma cuenta y se puede recuperar despues de
volver a autenticarte. La consulta no envia otra solicitud de registro.
**Retirar vinculo** impide su uso futuro y conserva la evidencia historica.

## Firmar una version exacta

Abre una version del historial del expediente y elige **Firmar personalmente**.
Descarga `document.pdf`, revisa la vista del mismo PDF y confirma su contenido.
La interfaz comprueba sus bytes contra el SHA-256 de esa version. Firma ese
archivo localmente:

```sh
openssl dgst -sha256 -sign signer-private.pem -out document.sig document.pdf
```

Carga `document.sig` y prepara el consentimiento. Revisa la cuenta, el
expediente, el documento, la version, el resumen del contenido, la huella del
certificado y el identificador de la operacion. Descarga
`document-consent.bin` y firma sus bytes exactos:

```sh
openssl dgst -sha256 -sign signer-private.pem \
  -out document-consent.sig document-consent.bin
```

Carga `document-consent.sig`, confirma y envia la firma personal. Qadra conserva
las dos firmas junto con el vinculo publico y la evidencia de confianza
capturada. El paquete de evidencia incluye instrucciones de verificacion
independiente.

Si no llega una respuesta, conserva la operacion y usa **Consultar recibo de
firma**. Cambiar de pantalla no repite el envio; **Recuperar intento de firma**
restaura la consulta para la misma cuenta, expediente y version. Los borradores
viven solamente en memoria: cerrar la pagina, cerrar sesion expresamente o
cambiar de cuenta los descarta. No prepares otra operacion para resolver una
respuesta perdida. Consulta el recibo original.

## Estado historico y actual

**Consultar estado de la firma** distingue una firma individual de un sello
historico del servidor. La validez historica comprueba la evidencia y los
instantes capturados. El estado actual del certificado y el retiro posterior
del vinculo se muestran por separado: no reescriben la historia.

El sello RFC 3161 del paquete acredita el resumen del contenido en el instante
de la TSA. No demuestra por si solo que la firma personal o el consentimiento
ya existieran entonces. Los tiempos de registro y aceptacion son recibos de
la aplicacion. La TSA local es evidencia tecnica del prototipo, no una
constancia NOM-151 de un PSC autorizado. Para verificar fuera del despacho,
confirma la raiz de confianza por un canal independiente; incluir una raiz
en el ZIP no la convierte en una autoridad confiable para terceros.
