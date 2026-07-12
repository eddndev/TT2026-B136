# Autoridad certificadora interna (PKI)

Scripts de shell sobre OpenSSL (probados con OpenSSL 3.5.7) para operar
una autoridad certificadora (CA) interna de prototipo, pensada para un
despacho juridico mexicano. La CA emite certificados de entidad final
para firmar documentos y autenticarse contra sistemas internos, y
mantiene una lista de revocacion (CRL).

ADVERTENCIA: el material de llaves privadas NUNCA se versiona. Todas
las llaves, certificados, solicitudes y la base de datos de la CA viven
unicamente en el directorio de trabajo en tiempo de ejecucion (ver
abajo). El `.gitignore` de la raiz del repositorio ya ignora
`pki/**/*.key` y `pki/**/*.pem` como red de seguridad, pero la regla es
mas simple: mantenga el directorio de trabajo de la CA fuera de
cualquier copia del repositorio.

## Archivos

| Archivo | Proposito |
| --- | --- |
| `openssl.cnf` | Configuracion de OpenSSL: politica de la CA, valores por defecto del nombre distinguido, extensiones v3 para la CA raiz y para certificados de entidad final, y ajustes de la CRL. |
| `init-ca.sh` | Crea la CA raiz: llave RSA de 3072 bits, certificado autofirmado valido 5 anios (1825 dias) e inicializa la base de datos (`index.txt`, `serial`, `crlnumber`). |
| `issue-cert.sh` | Emite un certificado de entidad final: genera llave RSA de 3072 bits y CSR para el nombre comun dado, y lo firma con la CA por 1 anio (365 dias). |
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
- Certificados de entidad final: `basicConstraints CA:false`,
  `keyUsage` con `digitalSignature` y `nonRepudiation`,
  `extendedKeyUsage` con `clientAuth` y `emailProtection`,
  `authorityKeyIdentifier` y `subjectKeyIdentifier`.
- La politica de la CA exige que pais (C) y organizacion (O) del
  certificado emitido coincidan con los de la CA; `issue-cert.sh` ya
  construye el sujeto con esos valores.
- El nombre comun aceptado por `issue-cert.sh` se limita a un
  subconjunto ASCII (letras, digitos, espacios y `. _ @ -`) para poder
  usarlo con seguridad en nombres de archivo y en la linea de comandos.
