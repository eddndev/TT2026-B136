# Contrato HTTP del servicio CA

El binario `qadra-ca --config ...` compone una API separada de la del despacho.
Su contrato está implementado en los routers de
[acceso](../../crates/web/src/ca_access.rs) y
[operaciones](../../crates/web/src/ca_operations.rs). El
[estado de la entrega](README.md) distingue implementación y aceptación del
servicio instalado. El [manual](operations.md) describe alta y configuración.
La separación de autoridad y recuperación se registra en
[ADR 0078](../adr/0078-separate-ca-operator-and-recoverable-operations.md).
La admisión anticipada de la política del CSR y el recibo de rechazo descritos
aquí se desplegaron en `v0.1.9`; la [aceptación instalada](verification.md)
conserva el incidente original y su conciliación sin emitir. La CA mantiene
`v0.1.11` durante la entrega Qadra `v0.1.12` (`201fbb02`), según el
[registro de verificación](../verification-report.md).

## Transporte y autoridad

El servicio escucha en loopback; el origen HTTPS propio corresponde a su
adopción de despliegue. Los cuerpos JSON usan `Content-Type: application/json`.
Las respuestas de acceso y operaciones incluyen `Cache-Control: no-store`.
No enviar contraseñas, factores, desafíos o tokens en URL, query, logs o capturas.

Las rutas protegidas requieren un único encabezado
`Authorization: Bearer <token>`. El esquema no distingue mayúsculas, pero el
token debe ser no vacío y contener sólo bytes ASCII gráficos. Encabezados
duplicados, un desafío usado como bearer o un token del despacho no autentican
una sesión CA. No hay autenticación mediante cookie.

La autoridad procede del UUID y correo del operador configurado, de su cuenta
activa y de una sesión CA admitida tras MFA. El servidor vuelve a comprobar
cuenta, generación de autenticación y sesión. El valor interno `Client` es un
formato de almacenamiento; la respuesta pública del operador no contiene rol.
Ser Owner del despacho no concede acceso a esta API.

El alta inicial es exclusivamente el comando administrativo
`enroll-operator`, con entrega privada y TOTP confirmado. No existen aquí
bootstrap HTTP, creación de usuarios, invitaciones, recuperación por correo,
login por certificado ni rutas de expedientes/documentos. Las rutas no
compuestas responden `404`. `GET /healthz` devuelve `200` y texto `ok` sin
sesión; no acredita una operación PKI ni aceptación de usuario.

## Inicio y administración de sesión

Todas las rutas de esta tabla tienen el prefijo `/api/v1/auth`:

| Método y ruta | Entrada | Respuesta correcta |
| --- | --- | --- |
| `POST /login` | JSON con `email`, `password` de tipo string. | `200`, `challenge_token` y `expires_in_seconds`. |
| `POST /mfa/totp` | JSON con `challenge_token`, `code`. | `200`, sesión del operador. |
| `POST /mfa/recovery` | JSON con `challenge_token`, `code`. | `200`, sesión del operador; consume el código aceptado. |
| `GET /me` | Bearer. | `200`, `{"id":"UUID","email":"correo"}`. |
| `GET /session` | Bearer, cuerpo vacío y sin query. | `200`, estado de sesión sin nuevo token. |
| `POST /activity` | Bearer, cuerpo vacío y sin query. | `200`, estado tras actividad explícita. |
| `POST /logout` | Bearer. | `204`, sin cuerpo; revoca el token. |

Los objetos de credenciales y de MFA rechazan campos desconocidos. El límite
del cuerpo de estas rutas es 16 KiB. Una contraseña correcta sólo entrega un
desafío de 300 segundos; éste no autoriza rutas protegidas. El intento MFA
consume el desafío también cuando se rechaza el factor. Para otro intento hay
que iniciar sesión de nuevo. El TOTP tiene seis dígitos; la recuperación utiliza
uno de los códigos emitidos durante el alta y no reemplaza la contraseña.

La respuesta de MFA tiene esta forma; los valores siguientes son ilustrativos:

```json
{
  "access_token": "TOKEN_DE_SESION",
  "token_type": "Bearer",
  "expires_in_seconds": 3600,
  "operator": {
    "id": "11111111-1111-4111-8111-111111111111",
    "email": "operator@example.test"
  },
  "policy": {
    "absolute_ttl_seconds": 3600,
    "idle_ttl_seconds": 900
  },
  "server_now_unix_ms": 1720000000000,
  "absolute_expires_at_unix_ms": 1720003600000,
  "idle_expires_at_unix_ms": 1720000900000
}
```

`/session` y `/activity` devuelven los mismos campos desde `operator` en adelante;
no incluyen `access_token`, `token_type` o `expires_in_seconds`. La política y
los plazos proceden del servidor; el cliente no puede enviarlos. La inactividad
y su plazo pueden ser `null`. `/me` y `/session` no prolongan la sesión;
`/activity` puede extender la inactividad sin superar la expiración absoluta.
Repetir logout con el token ya revocado devuelve `401`, no otro `204`.

## Comandos de CA

`POST /api/v1/ca/operations` requiere bearer y un objeto JSON discriminado por
`action`. La ruta sólo se compone cuando la configuración contiene `operations`;
sin esa sección responde `404`. El límite del cuerpo es 32 KiB. El catálogo
[`CaCommand`](../../crates/application/src/ca_operations.rs) es cerrado:

| `action` | Campos adicionales requeridos | Resultado correcto, siempre `200` |
| --- | --- | --- |
| `status` | Ninguno. | Confianza publicada, fechas y mantenimiento pendiente. |
| `inventory` | `offset`, `limit`. | Página del inventario público. |
| `certificate` | `serial`, `fingerprint`. | Certificado público de esa identidad exacta. |
| `preview` | `csr_pem`, `purpose`. | Sujeto y huellas revisadas, sin emitir. |
| `issue` | `operation_id`, `csr_pem`, `purpose`, `holder`, `reference`, `criterion`, `expected_root_fingerprint`. | Recibo de emisión; al consultar una operación anterior, puede devolver su rechazo terminal. |
| `resume-issue` | `operation_id`. | Recibo de la misma emisión recuperada o rechazo explícito de su solicitud anterior. |
| `history` | `offset`, `limit`. | Página de operaciones persistidas. |
| `revoke` | `operation_id`, `serial`, `fingerprint`, `reason`, `expected_root_fingerprint`. | Recibo de revocación local. |
| `resume-revoke` | `operation_id`. | Recibo de la misma revocación local recuperada. |
| `renew-crl` | `operation_id`, `expected_revision`, `acknowledge_maintenance`. | Recibo de publicación de la sucesora exacta. |
| `resume-crl` | `operation_id`. | Recibo de la misma publicación recuperada. |

Ejemplo de consulta, que no cambia el estado:

```json
{"action":"inventory","offset":0,"limit":20}
```

Se rechazan acciones y campos desconocidos, incluido un `operator_id` enviado
por el cliente. La aplicación obtiene ese actor de la sesión y lo entrega al
worker. Tampoco se admiten rutas, comandos de shell, ejecutables o claves
privadas como opciones HTTP. El sobre interno del worker `{"ok":...,"value":...}`
no forma parte de la respuesta HTTP: se devuelve directamente su valor público.

## Valores y límites de comandos

`operation_id` es un UUID canónico en minúsculas y no nulo. El cliente genera
una identidad para una nueva operación y conserva la misma ante incertidumbre.
Los campos `fingerprint`, `expected_root_fingerprint` y los digests devueltos
son SHA-256 representados por 64 dígitos hexadecimales minúsculos.

`purpose` sólo admite `partner` y `document-signing` en solicitudes y emisiones.
`csr_pem` es texto ASCII público no vacío de hasta 16384 bytes, sujeto además
a la validación criptográfica del CSR. Se comprueban la firma de la solicitud
y la raíz por separado; la huella adoptada fija la autoridad emisora. El titular
conserva la clave fuera de la CA. La inspección no acredita identidad civil.
Antes de reservar se exige CN y que cada C/O coincida por tipo y valor ASN.1
con algún atributo correspondiente de la raíz. Un rechazo de esta política
en `preview` o en una operación nueva produce `422 invalid_input`, sin journal
ni reserva. Una prueba criptográfica inválida o un fallo del inspector no se
clasifican como rechazo terminal de una operación guardada.

`holder`, `reference`, `criterion` y `reason` son strings no vacíos tras quitar
espacios, de hasta 500 bytes UTF-8 y sin caracteres de control. Los nombres y
descripciones capturados se conservan en los registros de la operación.

`serial` es hexadecimal positivo de hasta 40 dígitos, con valor menor que
`2^159 - 1`; los resultados usan mayúsculas sin ceros iniciales. Enviar la serie
devuelta por inventario junto con su huella. La descarga y revocación comprueban
ambas; la serie por sí sola no selecciona otro certificado.

`offset` es un entero no negativo representable como `u32`. `limit` debe estar
entre 1 y 100; la interfaz utiliza páginas de 20. `expected_revision` debe estar
entre 1 y `2^32 - 2` y coincidir con la confianza publicada. Para renovar,
`acknowledge_maintenance` debe ser literalmente `true`. La confirmación reconoce
que el procedimiento detiene el ingreso del despacho durante el mantenimiento.

## Respuestas públicas de operaciones

Los instantes de operaciones, certificados y CRL son segundos Unix. Los campos
de sesión con sufijo `_unix_ms` usan milisegundos. No inferir vigencia actual
únicamente de una inspección histórica o de la presencia de un recibo.

`status` devuelve `deployment_id`, `revision`, `root_fingerprint`, `crl_digest`,
`crl_number`, `valid_from`, `valid_until`, `checked_at`, `valid_now`,
`maintenance_pending`, `revoked_serials`, `root_pem` y `crl_pem`. La fuente es
la raíz y CRL persistidas por Qadra, reconciliadas con la autoridad adoptada.
El número de revocados mostrado por la interfaz se deriva de `revoked_serials`.

Las páginas tienen `items`, `total`, `offset` y `limit`. Inventario se ordena por
serie numérica. Cada elemento tiene `serial`, `subject`, `purpose`, `not_before`,
`not_after`, `fingerprint`, `root_fingerprint`, `validity`, `local_status` y
`published_status`:

| Campo | Valores |
| --- | --- |
| `purpose` en inventario | `partner`, `document-signing`, `time-stamping`. |
| `validity` | `not-yet-valid`, `valid`, `expired`, según las fechas y la consulta. |
| `local_status` | `issued`, `revoked`, `expired`, según el índice local. |
| `published_status` del portal | `not-revoked`, `pending`, `revoked`, según la CRL persistida consultada y el índice. |

`time-stamping` permite consultar y descargar el TSA existente de la misma CA;
no habilita emisión ni revocación TSA en el portal, acceso Owner o firma
documental. El adaptador de inventario aislado usa `unknown` cuando no ha
consultado la confianza publicada; el comando del portal la consulta antes
de devolver los estados anteriores. Si esa consulta falla, no inventa un estado.

`certificate` devuelve `serial`, `fingerprint` y `certificate_pem` como JSON.
No devuelve clave privada ni inicia por sí mismo una descarga del navegador.
`preview` devuelve `csr_sha256`, `spki_sha256`, `root_fingerprint`, `subject` y
`purpose`; no reserva una serie ni crea una emisión.

Una emisión confirmada en `issue` o `resume-issue` devuelve `operation_id`, `operator_id`, `recorded_at`,
`serial`, `fingerprint`, `purpose`, `certificate_pem`, `holder`, `reference` y
`criterion`. El recibo confirma la emisión exacta, no un alta o vínculo en Qadra.

Un journal anterior con incompatibilidad de política demostrada admite un
recibo alternativo sólo si coinciden sus huellas originales y permanecen
intactos el estado completo de OpenSSL y la ausencia de las hojas previstas.
Su respuesta `200` contiene exactamente `state:"rejected"`,
`reason:"csr_subject_policy"`, `operation_id`, `operator_id`, `recorded_at`,
`rejected_at`, `csr_sha256`, `root_fingerprint`, `reserved_serial`, `holder`,
`reference`, `criterion` y `purpose`. `reserved_serial` identifica la reserva
que no se consumió; no es una serie emitida. No hay `serial`, `fingerprint` ni
`certificate_pem`. Repetir el UUID conserva ese recibo y la solicitud original.

`revoke` y `resume-revoke` devuelven `operation_id`, `operator_id`, `recorded_at`,
`serial`, `fingerprint`, `reason`, `local_status:"revoked"` y
`published_status:"pending"`. La publicación se solicita por separado.

`renew-crl` y `resume-crl` devuelven `operation_id`, `operator_id`, `recorded_at`,
`expected_revision`, `revision`, `crl_number`, `root_fingerprint` y
`state:"published"`. La revisión es la sucesora exacta de la esperada.

`history` ordena por `recorded_at` y UUID, descendentes. Cada elemento incluye
`operation_id`, `kind`, `state`, `recorded_at` y `operator_id`; `kind` es
`issue`, `revoke` o `publication`, y `state` es `pending`, `completed` o
`rejected`; este último sólo corresponde a emisión.
Las emisiones añaden `holder`, `reference`, `criterion`, `serial` y
`fingerprint`, con los dos últimos `null` mientras están pendientes o rechazadas.
Las rechazadas añaden `reason:"csr_subject_policy"`. Las
revocaciones añaden `serial`, `fingerprint` y `reason`. Las publicaciones añaden
`expected_revision` y `revision`, esta última `null` mientras están pendientes.
El listado no incluye CSR, certificados PEM, credenciales ni claves.

## Errores y recuperación

Los errores de aplicación tienen el sobre:

```json
{"error":{"code":"invalid_session","message":"invalid session"}}
```

El cliente debe interpretar `code` y estado HTTP; el texto de `message` no es
una identidad de operación ni un recibo. Los rechazos del extractor JSON de
Axum, las rutas/métodos ausentes y el proxy pueden devolver otro formato,
incluido texto. No asumir JSON para cualquier respuesta fallida.

| HTTP | Código de aplicación, cuando corresponde | Significado real |
| --- | --- | --- |
| `400` | `invalid_session_request` | `/session` o `/activity` recibió cuerpo o query. JSON mal formado puede ser un rechazo propio de Axum. |
| `401` | `invalid_credentials` | Credenciales incorrectas o correo diferente del operador admitido. |
| `401` | `mfa_rejected` | Factor o desafío rechazado, expirado, consumido o no admitido. |
| `401` | `invalid_session` | Bearer ausente, ambiguo, revocado, vencido o sesión no válida. |
| `403` | `permission_denied` | Una identidad no coincide con el operador admitido; no se entrega un bearer utilizable por esa identidad. |
| `404` | Sin código CA específico. | Ruta no compuesta, incluidas operaciones deshabilitadas. Una serie ausente del worker no tiene un `404` especializado. |
| `405` | Sin código CA específico. | Método no admitido para una ruta. |
| `413` | Rechazo de transporte. | Cuerpo superior al límite HTTP o del proxy. |
| `415` | Rechazo de Axum. | Falta un tipo de contenido JSON admitido. |
| `422` | `invalid_input`, o rechazo de Axum. | Campos/tipos/acción incorrectos, límites de aplicación o respuesta `invalid` del worker; los campos desconocidos se rechazan. |
| `429` | `account_locked` | Límite de intentos de contraseña alcanzado. |
| `500` | `internal_error` | Fallo opaco de infraestructura, worker, inspección nativa, timeout o bloqueo clasificado por el ejecutor. |
| `503` | `server_busy` | Capacidad de admisión HTTP agotada antes de ejecutar el handler. |

Actualmente el bloqueo del worker se representa internamente como
`ClassifiedPort::Busy`, que el mapeo HTTP conserva como `500 internal_error`.
No hay un `409` específico de journal ni un `503` específico de ese bloqueo.
Un rechazo criptográfico del inspector nativo también puede llegar como fallo
opaco; el cliente no puede deducir su causa precisa a partir de un `500`.

El ejecutor nativo admite hasta 64 KiB de entrada interna y 3 MiB de salida,
con plazo de 180 segundos. Esos límites no son una promesa de duración total
HTTP: existe espera por trabajo disponible y el proxy tiene su propio plazo.
La configuración actual crea presupuestos de 8 peticiones y 2 trabajos
bloqueantes para cada router de acceso y de operaciones.

Después de timeout, desconexión o respuesta incierta, consultar `history` y
usar el `resume-*` del UUID original. La operación puede haber cambiado el
índice o publicado confianza antes del fallo de respuesta. Una reanudación
reconcilia los parámetros, actor, raíz y evidencia originales; no sustituye una
petición por otra ni presupone rollback. Los estados parciales o distintos
mantienen bloqueadas las mutaciones incompatibles y requieren investigación.
Nunca volver a emitir o renovar con un UUID nuevo para resolver incertidumbre.
Un fallo genérico o material modificado no produce el recibo `rejected` ni
libera la guarda. Sólo después de un rechazo terminal confirmado puede el
operador presentar un CSR corregido como una operación nueva.
