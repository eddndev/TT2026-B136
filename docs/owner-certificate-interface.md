# Mi certificado: contrato del cliente y guía del recorrido

## Estado de la entrega

El cliente y los helpers públicos aprobaron **10/10 pruebas Node en 502.6 ms**.
La interfaz Qadra aprobó **6/6 recorridos de navegador con HTTP controlado en
16.8 s**, con un worker. Un recorrido separado con servicios reales aprobó **1/1 en 12.1 s** de
Playwright; el comando completo duró **252.134 s**, con compilación y fixtures.
Ninguno de esos resultados acredita una instalación.

La aceptación HTTP con servicios reales de 334.419 s corresponde a preparación,
registro, consulta por UUID y retiro. La consulta `/current` tiene focales
separados. El recorrido real posterior sí ejercitó esta interfaz y la ruta `/current`,
con PostgreSQL 16.15, Valkey 8.1.10 como almacén compatible con Redis, MFA
y OpenSSL reales; no repitió el ensayo de restauración anterior.
Véanse [el contrato HTTP](http-owner-certificates.md) y
[la decisión de vínculo Owner](adr/0067-owner-certificate-bindings.md).

## Qué representa el vínculo

“Mi certificado” está reservado al Owner autenticado y relaciona un certificado
público con esa misma cuenta. Mantiene la sesión existente de contraseña y MFA.
No permite entrar mediante certificado ni firmar documentos del expediente.
La CA interna es una autoridad de demostración; no equivale a FIREL, e.firma o
un prestador de servicios de certificación.

La consulta inicial descubre el vínculo propio que no tenga retiro terminal.
Ese estado no acredita que el certificado o la CRL sigan vigentes. El recibo
describe la evidencia y confianza capturadas al registrar. Aunque el certificado
venza o sea revocado posteriormente, la cuenta autorizada puede consultar la
historia exacta y retirar el vínculo.

El retiro es terminal: conserva el recibo original y no revoca el certificado en
la CA. Un registro posterior usa una intención nueva y su propia firma explícita;
no reactiva el UUID retirado. La gestión de revocación y custodia de claves tiene
un procedimiento distinto.

## Registrar un vínculo

1. Inicia sesión mediante contraseña y MFA y abre **Mi certificado**. La vista
   debe consultar primero la cuenta vigente y después su vínculo sin retirar.
   Un rol distinto de Owner no recibe acceso a esta función.
2. Inicia un registro y selecciona un **certificado público PEM** de hasta
   **16 KiB**. Debe contener un solo bloque `CERTIFICATE`, sin otros bloques ni
   material añadido. El selector no admite archivos DER, claves privadas,
   claves cifradas ni solicitudes de certificado. El nombre del archivo no
   decide su admisión.
3. Solicita la preparación. El servidor inspecciona el certificado y captura
   el material público, la cuenta, la confianza y los bytes que deben firmarse.
   Preparar no registra el vínculo ni acredita su admisión completa: la prueba
   RSA y las comprobaciones de propósito, vigencia y revocación se realizan al
   registrar. La comprobación local del contenedor PEM tampoco las sustituye.
4. Descarga la **declaración binaria de exactamente 150 bytes** y fírmala fuera
   de la aplicación con la clave correspondiente al certificado, usando el
   perfil SHA-256/RSA-3072 definido por el contrato. Conserva los bytes exactos;
   no los conviertas a texto, JSON ni otra declaración.
5. Selecciona la **firma separada binaria de exactamente 384 bytes**. La pantalla
   no solicita ni envía la clave privada. Una firma del tamaño esperado todavía
   debe superar la verificación del servidor.
6. Confirma expresamente el registro. El cliente envía el UUID elegido para esa
   intención, la declaración exacta, el DER público admitido y la firma. Una
   preparación de otra cuenta o una captura de confianza ya sustituida no se
   convierte silenciosamente en una nueva intención.
7. Consulta el recibo por su UUID y descarga su evidencia pública. La consulta
   histórica conserva declaración, certificado, firma, confianza y fechas; no
   vuelve a firmar ni presenta la captura histórica como confianza vigente.

Si ya hay un vínculo sin retirar, el recorrido debe permitir consultarlo y
decidir su retiro antes de iniciar otro. Retirar exige confirmación expresa y
la revisión original; no requiere cargar otra firma o disponer de la clave
perdida. Los controles mantienen la evidencia original durante la decisión.

## Firma externa de la declaración

Con OpenSSL y una clave RSA-3072 ya correspondiente al certificado público,
guarda la descarga como `owner-declaration.bin` y ejecuta en el equipo que
custodia esa clave:

```sh
openssl dgst -sha256 -sign owner.key.pem \
  -out owner-declaration.sig owner-declaration.bin
openssl x509 -in owner.crt.pem -pubkey -noout > owner-public.pem
openssl dgst -sha256 -verify owner-public.pem \
  -signature owner-declaration.sig owner-declaration.bin
```

Sustituye los nombres por tus archivos. Si la clave está cifrada, OpenSSL solicita
su frase de acceso localmente. Selecciona únicamente `owner-declaration.sig` en
**Firma separada**. `owner.key.pem` nunca se selecciona en Qadra ni se envía al
servidor. La verificación local confirma esa firma sobre esos bytes; la admisión
de cuenta, propósito, confianza, vigencia y revocación corresponde al registro
en el backend. Este ejemplo no emite certificados, entrega claves ni configura
un dispositivo de custodia.

## Reingreso y respuestas inciertas

El contrato de recuperación conserva sólo material público admitido: preparación
y DER, firma separada y comando exacto si el envío ya comenzó. No conserva el
File original, PEM aún sin admitir, claves, bearer, contraseñas, clientes API
ni operaciones pendientes. La captura reside en memoria, sin almacenamiento
persistente; no promete recuperación al cerrar o recargar la página.

Tras vencer la sesión, volver a entrar como el mismo Owner no envía comandos.
Antes de ofrecer continuar se consultan la identidad vigente y `/current`.
Retomar es una decisión explícita y conserva la declaración original, incluso
si una nueva preparación produciría otros bytes. Cancelar o cerrar sesión
descarta el borrador; otra cuenta no puede recuperarlo. Una denegación vigente
de autoridad descarta ese contexto personal; un fallo de consulta no autoriza
la recuperación ni confirma una escritura.

Un fallo de transporte, del servidor o una pérdida de autoridad después de
iniciar el envío puede dejar un resultado incierto. El comando se captura antes
de esperar la respuesta. Para resolverlo:

- Consulta el **UUID exacto** de esa intención. Una coincidencia en `/current`
  sirve para descubrir el vínculo, pero no confirma por sí sola el envío.
- Compara cuenta, UUID, declaración completa, DER y firma. Un registro exacto
  conserva su identidad incluso si después fue retirado.
- La ausencia de recibo no reenvía nada. Sólo una decisión explícita posterior
  puede repetir el mismo comando, con el mismo UUID y los mismos bytes. Un
  nuevo reingreso exige volver a consultar antes de habilitar esa decisión.
- Para un retiro, el recibo debe conservar el registro original y contener su
  retiro terminal. Si sigue sin retirar, el resultado continúa sin confirmar;
  una evidencia distinta es un conflicto.

Los seis recorridos controlados comprobaron registro y retiro, permisos,
reingreso del mismo Owner, ausencia y confirmación de un envío incierto, descarte
al cerrar sesión o cambiar de cuenta y rechazo de un recibo terminal discrepante.
Son complementarios a los tests Node; no sustituyen RSA y persistencia reales.

## Contrato técnico del cliente

La única entrada es `createApi().ownerCertificates(ownerId)`. Reutiliza el
transporte común: bearer de la sesión, admisión de actividad, respuesta sin
caché, rechazo de redirecciones y descarte de respuestas de sesiones anteriores.
Cada instancia queda ligada a la sesión en que se creó y debe sustituirse
después de otra MFA. `dispose()` impide usarla o entregar respuestas tardías.

| Método | Resultado y frontera |
| --- | --- |
| `current()` | Recibo propio sin retirar o `null`; rechaza evidencia ajena o terminal. |
| `get(id)` | Recibo propio del UUID exacto, incluido su retiro histórico. |
| `prepare(id, certificateBase64)` | Envía el PEM público acotado y valida la identidad de la preparación. |
| `register(id, data)` | Envía una copia inmutable de declaración, DER y firma; exige recibo del mismo comando. |
| `withdraw(id, expectedRevision)` | Envía revisión 1 y exige recibo terminal de la misma cuenta y UUID. |
| `dispose()` | Cierra la instancia; no revoca la sesión ni escribe al servidor. |

`account_revision`, `auth_generation` y `crl_number` permanecen cadenas decimales
canónicas; no se convierten a `Number`. Los dos contadores de cuenta se acotan a
i64 y el número de CRL a u64. Las revisiones de confianza y vínculo son enteros.
Las fechas conservan su texto original, incluida la precisión de nanosegundos.

`readOwnerPublicCertificate` devuelve un Blob independiente después de comprobar
los límites antes y después de leer. `ownerStatementDownload` conserva los 150
bytes y rechaza otro propósito o codificación. `registrationIntent` y
`withdrawalIntent` producen comandos públicos independientes, congelados y
compatibles con JSON. `reconcileOwnerIntent` devuelve `matched`, `absent`,
`conflict` o `unconfirmed`; `uncertainOwnerWrite` clasifica la respuesta sin
realizar IO, reintentos o generación de UUID. Estos helpers no firman, calculan
huellas ni sustituyen la verificación criptográfica del backend.
