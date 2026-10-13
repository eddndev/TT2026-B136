# Autoridad certificadora interna (PKI)

Scripts de shell sobre OpenSSL (probados con OpenSSL 3.5.7) para operar
una autoridad certificadora (CA) interna de prototipo, pensada para un
despacho juridico mexicano. La CA emite certificados de entidad final
para firmar documentos y autenticarse contra sistemas internos, y
mantiene una lista de revocacion (CRL).

ADVERTENCIA: el material de llaves privadas NUNCA se versiona. El material
operativo de la CA, incluidos su clave y su base de datos, vive unicamente
en el directorio de trabajo en tiempo de ejecucion (ver abajo). La clave
del titular que presenta un CSR permanece en su propio equipo. El
`.gitignore` de la raiz del repositorio ya ignora
`pki/**/*.key` y `pki/**/*.pem` como red de seguridad, pero la regla es
mas simple: mantenga el directorio de trabajo de la CA fuera de
cualquier copia del repositorio.

## Archivos

| Archivo | Proposito |
| --- | --- |
| `openssl.cnf` | Configuracion de OpenSSL: politica de la CA, valores por defecto del nombre distinguido, extensiones v3 para la CA raiz y para certificados de entidad final, y ajustes de la CRL. |
| `init-ca.sh` | Crea la CA raiz: llave RSA de 3072 bits, certificado autofirmado valido 5 anios (1825 dias) e inicializa la base de datos (`index.txt`, `serial`, `crlnumber`). |
| `issue-cert.sh` | Emite un certificado de entidad final: genera llave RSA de 3072 bits y CSR para el nombre comun dado, y lo firma con la CA por 1 anio (365 dias). |
| `issue-declaration-cert.sh` | Emite una hoja RSA de 3072 bits para declaraciones internas, con usos de firma y sin EKU de autenticacion o correo. |
| `issue-request.sh` | Firma un CSR publico externo con el perfil documental por defecto o Partner explicito; la CLI valida la solicitud, la clave publica emitida y el perfil, sin generar una clave del titular. |
| `revoke.sh` | Revoca un certificado emitido, marcandolo en la base de datos de la CA. |
| `gen-crl.sh` | Genera la CRL en formato PEM, valida por 7 dias. |

## Directorio de trabajo de la CA

Todos los scripts leen la variable de entorno `PKI_CA_DIR`. Si no esta
definida, usan `pki-ca` bajo el directorio actual de trabajo. Ese
directorio es un artefacto de ejecucion: contiene llaves privadas y la
base de datos de la CA, y nunca debe confirmarse en control de
versiones. Ejecute los scripts desde un directorio fuera del
repositorio o apunte `PKI_CA_DIR` a una ruta externa, por ejemplo:

```sh
export PKI_CA_DIR="$HOME/pki-ca"
```

Estructura que crea `init-ca.sh` dentro de `PKI_CA_DIR`:

```
pki-ca/
  ca.crt.pem        certificado raiz (publico)
  index.txt         base de datos de certificados emitidos
  serial            siguiente numero de serie
  crlnumber         siguiente numero de CRL
  private/          llaves privadas (permisos 700, nunca versionar)
  certs/            certificados emitidos
  csr/              solicitudes de firma
  crl/crl.pem       lista de revocacion generada
  newcerts/         copias por numero de serie que guarda openssl ca
```

## Orden de uso

Los scripts deben ejecutarse en este orden, porque cada paso depende
del anterior:

1. `init-ca.sh` - una sola vez, crea la CA. Re-ejecutarlo es seguro:
   no sobreescribe una CA existente.
2. `issue-cert.sh COMMON_NAME` - tantas veces como certificados se
   necesiten.
3. `revoke.sh CERT_PATH` - cuando un certificado deba invalidarse.
4. `gen-crl.sh` - despues de cada revocacion y al menos cada 7 dias,
   porque la CRL expira a los 7 dias.

Ejemplo completo:

```sh
export PKI_CA_DIR="$HOME/pki-ca"
./init-ca.sh
./issue-cert.sh "Juan Perez"
./revoke.sh "$PKI_CA_DIR/certs/juan-perez.crt.pem"
./gen-crl.sh
```

## Verificacion

### Emision desde una clave privada externa

Para firma documental individual, el titular genera la clave y el CSR en su
equipo, fuera del directorio de la CA. Este ejemplo usa una clave cifrada;
OpenSSL solicita su contrasena localmente:

```sh
umask 077
openssl genpkey -algorithm RSA -aes-256-cbc -pkeyopt rsa_keygen_bits:3072 \
    -pkeyopt rsa_keygen_pubexp:65537 -out signer-private.pem
openssl req -new -sha256 -key signer-private.pem -out signer.csr.pem \
    -subj '/C=MX/O=Despacho Juridico Demo/CN=Firma documental'
```

El administrador recibe solamente `signer.csr.pem`. Con `PKI_CA_DIR` apuntando
a la CA inicializada, usa la CLI, que aplica las validaciones de entrada antes
de ejecutar el script de emision:

```sh
cargo run -p despacho-cli -- pki issue-request \
    --csr signer.csr.pem --name litigante-demo --json
```

El proposito por defecto de `pki issue-request` es `document-signing`; puede
declararse tambien con `--purpose document-signing`. Este perfil conserva los
usos de firma y no incluye EKU de autenticacion o correo. El proposito
`--purpose partner` selecciona el perfil existente de acceso Owner, con los
EKU exactos `clientAuth` y `emailProtection`. Ambos perfiles son hojas
CA=false y exigen los usos `digitalSignature` y `nonRepudiation`.

El CSR PKCS#10 PEM admite como maximo 16 KiB y debe probar posesion de una clave
RSA de 3072 bits, exponente 65537 y firma SHA-256. El nombre de salida tiene
entre 1 y 64 caracteres ASCII, empieza por letra o digito y solo admite letras,
digitos, punto, guion y guion bajo. No identifica por si mismo a una cuenta.
La emision usa extensiones controladas, sin copiar las solicitadas, y verifica
que la hoja conserva la clave publica del CSR. Pais y organizacion deben
coincidir con la CA; ajuste el sujeto del ejemplo si su configuracion difiere.

El resultado publico queda en `$PKI_CA_DIR/certs/litigante-demo.crt.pem`;
la salida JSON contiene `serial` y `certificate`. No se crea ni se recibe una
clave privada del titular. El vinculo con la cuenta Litigator requiere despues
su prueba de posesion propia; el consentimiento documental requiere una segunda
firma que fija el PDF y su contexto. Siga la
[guia de firma individual](../docs/individual-signing-guide.md) para esos pasos.
La CA interna y las claves sinteticas de las pruebas no acreditan identidad
civil ni una emision de FIREL o de un PSC.

Para el acceso Owner, el titular prepara una clave y un CSR propios en su
equipo. La contrasena de la clave cifrada se introduce solo en ese equipo:

```sh
umask 077
openssl genpkey -algorithm RSA -aes-256-cbc -pkeyopt rsa_keygen_bits:3072 \
    -pkeyopt rsa_keygen_pubexp:65537 -out owner-private.pem
openssl req -new -sha256 -key owner-private.pem -out owner.csr.pem \
    -subj '/C=MX/O=Despacho Juridico Demo/CN=Acceso Owner'
```

El administrador recibe solamente `owner.csr.pem` y selecciona Partner:

```sh
cargo run -p despacho-cli -- pki issue-request \
    --csr owner.csr.pem --name owner-access --purpose partner --json
```

El certificado publico queda en `$PKI_CA_DIR/certs/owner-access.crt.pem`.
La clave `owner-private.pem` permanece con el titular; no se genera una clave
del titular dentro de la CA. La emision no crea una cuenta Owner, no registra
su vinculo y no activa el acceso por certificado. Esos pasos conservan su
[recorrido operativo](../docs/owner-certificate-login-operations.md), incluida
la prueba de posesion y MFA. El perfil Partner no sustituye al documental
en la firma individual.

La extension del emisor CSR esta implementada localmente y tiene verificacion
CLI focal. Su publicacion y despliegue conservan sus propios gates; no acredita
la operacion del [portal CA](../docs/ca/README.md), cuyo enrolamiento, servicio
y aceptacion siguen pendientes. `pki issue` conserva su comportamiento
existente y su proposito Partner por defecto.

### Certificados para declaraciones de participantes

Desde la raiz del repositorio, con `PKI_CA_DIR` apuntando a una CA inicializada:

```sh
bash pki/issue-declaration-cert.sh "Participante Sintetico"
# La misma emision a traves de la CLI:
cargo run -p despacho-cli -- pki issue --cn "Otro Participante Sintetico" \
    --purpose participant-declaration --json
```

El proposito por defecto de `pki issue` sigue siendo `partner`, con los EKU
`clientAuth` y `emailProtection`. El proposito `participant-declaration` usa
`basicConstraints` critico CA=false y `keyUsage` critico con `digitalSignature`
y `nonRepudiation`, sin `extendedKeyUsage`. La emision conserva los archivos
existentes y rechaza reutilizar una ruta de certificado o llave.

Una declaracion binaria puede firmarse fuera del servidor con la llave del
titular sintetico:

```sh
openssl dgst -sha256 \
    -sign "$PKI_CA_DIR/private/participante-sintetico.key.pem" \
    -out declaracion.sig declaracion.bin
```

La firma resultante tiene 384 bytes. El servidor recibe el certificado publico
y la firma, nunca la llave privada ni una contrasena de esta. La CA interna
permite ensayar la comprobacion criptografica; no emite FIREL oficial ni
comprueba la identidad civil, profesion o nombramiento judicial del titular.

### Publicar confianza para declaraciones

La API admite credenciales contra una raiz y una CRL publicadas en PostgreSQL.
Despues de aplicar las migraciones, use una conexion administrativa:

```sh
export DATABASE_URL='postgresql://administrador@localhost/despacho'
cargo run -p despacho-cli -- credential-trust publish \
    --root-cert "$PKI_CA_DIR/ca.crt.pem" --crl "$PKI_CA_DIR/crl/crl.pem" \
    --expected-revision 0 --json
```

La primera publicacion fija el UUID del despliegue y la raiz. Para publicar otra
CRL, ejecute `gen-crl.sh` y repita el comando con la ultima revision conocida.
El numero de CRL debe aumentar y `thisUpdate` no puede retroceder. La raiz no
se sustituye mediante este comando. El rol operativo de la API tiene solo
lectura sobre estas tablas; no puede publicar confianza.

Revocar en la base local de OpenSSL no actualiza por si solo la confianza de la
API: se debe generar y publicar la nueva CRL. La publicacion y su auditoria son
una transaccion; capturan `SESSION_USER` del administrador de base de datos.
Una restauracion debe conservar todas las revisiones publicadas y el mismo UUID
del despliegue, ademas de la evidencia de cada participante. La evidencia
historica informa la comprobacion capturada, no su vigencia actual.

En `--json`, `crl_number` es una cadena decimal para preservar los 64 bits al
leer el resultado en JavaScript. Los certificados y CRL son material publico;
las claves individuales se mantienen fuera del servidor y del archivo de
fixtures utilizado por el navegador. El contrato detallado esta en
[la API de participantes tipificados](../docs/typed-participants-api.md).

### Cadena y revocacion

Comprobar que un certificado emitido es valido frente a la CA:

```sh
openssl verify -CAfile "$PKI_CA_DIR/ca.crt.pem" \
    "$PKI_CA_DIR/certs/juan-perez.crt.pem"
```

Comprobar la revocacion contra la CRL (debe reportar
`certificate revoked` despues de `revoke.sh` + `gen-crl.sh`):

```sh
cat "$PKI_CA_DIR/ca.crt.pem" "$PKI_CA_DIR/crl/crl.pem" > /tmp/ca-crl.pem
openssl verify -crl_check -CAfile /tmp/ca-crl.pem \
    "$PKI_CA_DIR/certs/juan-perez.crt.pem"
```

Inspeccionar la CRL y los seriales revocados:

```sh
openssl crl -noout -text -in "$PKI_CA_DIR/crl/crl.pem"
```

## Detalles tecnicos

- Llaves RSA de 3072 bits y firmas SHA-256 en todos los casos.
- Certificado raiz: `basicConstraints CA:true`, `keyUsage` con
  `keyCertSign` y `cRLSign`, `subjectKeyIdentifier`.
- Certificados de entidad final de proposito `partner`: `basicConstraints CA:false`,
  `keyUsage` con `digitalSignature` y `nonRepudiation`,
  `extendedKeyUsage` con `clientAuth` y `emailProtection`,
  `authorityKeyIdentifier` y `subjectKeyIdentifier`.
- Certificados de declaracion: la misma clave RSA y usos de firma, sin EKU;
  la CLI inspecciona tambien el certificado emitido antes de informar exito.
- La politica de la CA exige que pais (C) y organizacion (O) del
  certificado emitido coincidan con los de la CA; `issue-cert.sh` ya
  construye el sujeto con esos valores.
- El nombre comun aceptado por `issue-cert.sh` se limita a un
  subconjunto ASCII (letras, digitos, espacios y `. _ @ -`) para poder
  usarlo con seguridad en nombres de archivo y en la linea de comandos.
