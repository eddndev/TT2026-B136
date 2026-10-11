# API de invitaciones y enrolamiento recuperable

Este contrato describe las rutas locales de invitacion y su configuracion en
`serve`. La aceptacion integrada y el estado de despliegue se registran por
separado en [el informe de verificacion](verification-report.md). La decision
de diseno esta en [el ADR de invitaciones](adr/0075-recoverable-member-invitations.md).

Un Owner activo invita a un Litigator o Paralegal mediante correo y rol. El
destinatario elige su contrasena y confirma TOTP antes de que se cree una cuenta
utilizable. El flujo no tiene cuota de cuentas, planes comerciales ni campos
personales adicionales. Las cuentas existentes, incluidas las inactivas, se
administran mediante [miembros](members-api.md); no se sustituyen con una
invitacion. El alta directa existente conserva su contrato.

## Rutas y limites comunes

Las rutas administrativas requieren `Authorization: Bearer <sesion>` de Owner.
Las rutas de aceptacion son publicas: la capacidad privada del correo y la
contrasena autorizan el recorrido del destinatario. El cliente publico no
adjunta una sesion de otra cuenta.

| Metodo y ruta | Entrada | Respuesta correcta |
| --- | --- | --- |
| `POST /api/v1/member-invitations` | `operation_id`, `email`, `role` | `201`, resumen |
| `GET /api/v1/member-invitations` | Consulta `limit`, `after` opcionales | `200`, pagina |
| `GET /api/v1/member-invitations/{id}` | UUID de invitacion | `200`, resumen actual |
| `POST /api/v1/member-invitations/{id}/resend` | `operation_id`, `expected_revision` | `200`, resumen del reenvio |
| `POST /api/v1/auth/invitations/prepare` | `token`, `operation_id`, `password` | `200`, preparacion MFA |
| `POST /api/v1/auth/invitations/finish` | Los anteriores y `code` | `200`, recibo |
| `POST /api/v1/auth/invitations/receipt` | `token`, `operation_id`, `password` | `200`, objeto `receipt` |

Los POST requieren un solo encabezado de tipo JSON, un objeto JSON de hasta
16384 bytes y exactamente los campos indicados. Se rechazan campos desconocidos,
incluidos datos de perfil, cuota o material MFA en las solicitudes de Owner.
No admiten parametros de consulta. Los GET tienen cuerpo vacio; solo el listado
admite consulta. Se acepta `application/json` y el tipo `application/*+json`.

Los identificadores son UUID no nulos. `revision` y `expected_revision` son
cadenas decimales, nunca numeros JSON: sin signo ni ceros iniciales. Una revision
esta entre `"1"` y `"9223372036854775807"`; la revision esperada de reenvio debe
ser menor que ese maximo para poder incrementarla. Las fechas se emiten en UTC
con formato RFC 3339.

La admision y el trabajo bloqueante comparten los limites HTTP del servidor.
El permiso de trabajo sigue retenido mientras termina la operacion bloqueante,
aunque el cliente pierda la conexion. Las rutas publicas tienen ademas un
presupuesto Redis independiente: la composicion actual admite hasta 200 intentos
globales y 5 por capacidad en ventanas de 300 segundos. Este presupuesto limita
intentos de preparacion, confirmacion y consulta de recibo; no limita cuentas.

## Invitacion del Owner

Ejemplo de solicitud inicial:

```json
{
  "operation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  "email": "integrante@example.test",
  "role": "litigator"
}
```

`role` solo admite `litigator` o `paralegal`. El correo se normaliza. El UUID de
la primera operacion tambien identifica la invitacion, lo que permite consultar
su resultado despues de perder la respuesta. El resumen tiene esta forma:

```json
{
  "id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  "operation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  "revision": "1",
  "email": "integrante@example.test",
  "role": "litigator",
  "status": "pending",
  "delivery": "pending",
  "issued_at": "2026-10-11T00:00:00Z",
  "expires_at": "2026-10-12T00:00:00Z",
  "accepted_at": null
}
```

La emision dura 24 horas desde su registro transaccional. `status` distingue
`pending`, `pending_mfa`, `accepted` y `expired`. `accepted_at` solo contiene la
fecha de aceptacion cuando existe una cuenta creada por este recorrido. Ninguna
respuesta de Owner incluye token, contrasena, secreto TOTP o codigos de recuperacion.

El listado devuelve `{"items": [...], "next": null}`. `limit` vale 50 por
defecto y admite 1 a 100. Las filas se ordenan por UUID ascendente; cuando hay
continuacion, `next` contiene el ultimo UUID de la pagina y se pasa como `after`.
La consulta por ID devuelve el estado actual, incluido el reenvio vigente.

Ante una respuesta perdida, se conserva el mismo UUID y la misma solicitud.
Repetir el POST recupera el resultado de esa operacion; reutilizar su UUID con
otros datos produce conflicto. Consultar el ID permite comparar su
`operation_id` con el intento pendiente. Si existe un reenvio posterior, repetir
el POST original permite recuperar el resultado original sin rotar otra vez.

Un reenvio es una accion explicita con un UUID nuevo:

```json
{
  "operation_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
  "expected_revision": "1"
}
```

El reenvio conserva ID, correo y rol; incrementa la revision, emite otra
capacidad y establece otra vigencia de 24 horas. Invalida atomicamente el enlace
anterior y su preparacion MFA incompleta. Una invitacion aceptada no se reenvia.
Si se pierde la respuesta del reenvio, se repite exactamente su UUID y revision
esperada. Una revision obsoleta en una operacion nueva produce conflicto.

## Aceptacion y recuperacion del destinatario

El correo contiene `https://<origen-publico>/#invitation=<token>`. El token es
una capacidad aleatoria de 32 bytes codificada en base64url canonico, sin relleno
(43 caracteres). El fragmento no forma parte de la peticion de navegacion. Qadra
lo retira de la barra de direcciones antes de usarlo y lo conserva en memoria.
Las llamadas publicas envian ese token en el cuerpo JSON; nunca en query,
encabezados de identidad, mensajes de error o almacenamiento persistente del
navegador. Para retomar tras cerrar la pagina se abre el correo original.

`prepare` recibe un UUID de operacion y la contrasena elegida, de 12 a 1024 bytes
UTF-8. Devuelve los campos `id`, `revision`, `operation_id`, `email`, `role`,
`expires_at`, `totp_secret_base32`, `otpauth_uri` y `recovery_codes`. Los tres
ultimos son secretos destinados solo al invitado. Se debe conservar los codigos
de recuperacion antes de continuar.

La preparacion MFA se guarda cifrada. Repetir la preparacion con la misma
capacidad valida y contrasena recupera el material pendiente original, sin
reemplazar el segundo factor. La respuesta identifica la operacion de preparacion
que quedo persistida. En esta etapa todavia no se crea una cuenta ni una sesion.

`finish` utiliza un UUID propio para confirmar el alta, la misma capacidad y
contrasena, y `code` con seis digitos ASCII de TOTP. La cuenta, el consumo de la
capacidad, la auditoria y el recibo se registran en una transaccion. Solo tras
esa confirmacion puede usarse el inicio de sesion normal con contrasena y MFA.
La respuesta de `finish` es directamente el recibo:

```json
{
  "id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  "revision": "1",
  "operation_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
  "user_id": "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
  "email": "integrante@example.test",
  "role": "litigator",
  "accepted_at": "2026-10-11T00:05:00Z"
}
```

Si la respuesta de `finish` es incierta, se consulta `receipt` con el UUID de
esa confirmacion, token y contrasena antes de repetir el alta. Esta ruta devuelve
`{"receipt": <recibo>}` o `{"receipt": null}`; no devuelve secretos MFA. La
consulta verifica la contrasena contra la cuenta resultante. Puede recuperar
una aceptacion ya registrada aunque haya pasado la vigencia original del enlace.
Un resultado nulo permite continuar el registro si la capacidad sigue vigente.

Repetir un `finish` ya aplicado exige el mismo UUID y contenido, incluido su
codigo TOTP original. Para recuperar su resultado despues, se prefiere `receipt`,
que no pide codigo. Un UUID aplicado con contenido distinto se rechaza y nunca
crea otra cuenta. Qadra persiste solo identificadores de operacion y fase,
asociados a una huella del enlace; no persiste token, contrasena o material MFA.

## Correo y configuracion

Las invitaciones estan deshabilitadas por defecto. Su activacion es independiente
de recuperacion de contrasena. `serve` reconoce:

| Variable | Uso |
| --- | --- |
| `TT_INVITATIONS_ENABLED` | Activa expresamente el flujo; tambien existe `--invitations-enabled` |
| `INVITATION_EMAIL_FROM` | Remitente obligatorio; tambien `--invitation-email-from` |
| `INVITATION_PUBLIC_URL` | Origen HTTPS publico obligatorio; tambien `--invitation-public-url` |
| `RESEND_API_KEY` | Secreto del proveedor, requerido cuando se activa |

El remitente debe ser una direccion simple. La URL debe apuntar a la raiz `/`
de un origen HTTPS, sin credenciales, query ni fragmento. La configuracion
incompleta o invalida impide construir el flujo activado. Con la opcion
deshabilitada no se construye el consumidor de correo y las rutas responden
`invitation_unavailable` despues de validar su entrada correspondiente.

El transporte usa el endpoint fijo `https://api.resend.com/emails`; no sigue
redirecciones, usa proxy ni reintenta automaticamente una solicitud. La
composicion configura 2 segundos de conexion y 5 segundos totales. Cada revision
conserva un `delivery_id` y la clave del proveedor `invitation:<delivery_id>`.
Los intentos repetidos conservan el cuerpo del mensaje bajo la misma configuracion.

| `delivery` | Significado |
| --- | --- |
| `pending` | Envio persistido pendiente de intento |
| `sent` | El proveedor confirmo aceptacion; no acredita lectura ni llegada al buzon |
| `rejected` | Rechazo definitivo reconocido; requiere revisar la causa operativa y reenviar explicitamente |
| `uncertain` | No se pudo confirmar el resultado; el mensaje pudo haberse enviado |

El consumidor reclama un envio por intento mediante un lease de hasta 30
segundos. Comprueba lease, vigencia y ventana del proveedor antes de enviar;
registra el resultado solo si la revision, generacion y lease siguen vigentes.
Los resultados inciertos admiten otro intento tras 60 segundos, siempre antes
de la expiracion y del limite de 24 horas contado desde la primera reclamacion.
El limite no se renueva al reintentar. Fuera de esa ventana hace falta un reenvio
explicito, con otra revision y otra clave de idempotencia.

## Invalidacion administrativa despues de restaurar

Sobre una copia restaurada, el operador mantiene cerrada la admision de la
aplicacion y detenidos los escritores y consumidores de correo durante toda la
restauracion. La invalidacion es una operacion administrativa explicita; no la
ejecuta `serve` al arrancar ni activa las invitaciones o el correo publico.

`database invalidate-restored-invitations` utiliza `DATABASE_URL` con la
autoridad propietaria de la tabla de invitaciones en la copia restaurada. El rol
Owner de la aplicacion no sustituye esa autoridad PostgreSQL. Los argumentos
son los mismos de `database invalidate-restored-password-resets`:

| Argumento | Valor que debe conservarse |
| --- | --- |
| `--operation-id` | UUID canonico no nulo de esta operacion administrativa |
| `--expected-database` | Nombre exacto de la base restaurada |
| `--expected-schema` | Esquema real de la aplicacion |
| `--expected-audit-sequence` | Secuencia original, desde cero y hasta `i64::MAX` |
| `--expected-audit-head` | SHA-256 original, 64 caracteres hexadecimales en minusculas |
| `--expected-empty-audit` | Alternativa explicita al par anterior, solo para cadena vacia |

La secuencia y la huella se proporcionan juntas; no se combinan con
`--expected-empty-audit`. El operador conserva la solicitud completa para
conciliar una respuesta perdida. Ejemplo de forma, con valores obtenidos de la
copia restaurada y `DATABASE_URL` ya configurada para ella:

```sh
despacho-cli --json database invalidate-restored-invitations \
  --operation-id "$RESTORE_OPERATION_ID" \
  --expected-database "$RESTORED_DATABASE_NAME" \
  --expected-schema "$RESTORED_SCHEMA_NAME" \
  --expected-audit-sequence "$RESTORED_AUDIT_SEQUENCE" \
  --expected-audit-head "$RESTORED_AUDIT_HEAD"
```

La operacion fija el esquema y adquiere el bloqueo de migracion de ese esquema,
ademas del bloqueo de mutacion auditada. Revalida base, esquema, identidad y
propiedad de la tabla, estructura, permisos e inventario relacionado. Verifica
la cadena de auditoria completa y compara su cabeza con el antecedente exigido.
Una discrepancia rechaza la operacion; no se usa para reparar un catalogo o una
cadena incompatible.

Tras bloquear las invitaciones no aceptadas ni canceladas, toma la hora de
PostgreSQL y rechaza tiempos invalidos o emisiones futuras. En una transaccion
marca esas invitaciones como canceladas, borra el token cifrado y la preparacion
MFA, retira los leases y reintentos de correo, establece entrega rechazada y
agrega `identity.invitation_restore_invalidated` a la auditoria. Comprueba que el
numero actualizado coincide con el bloqueado y que no quedan pendientes. No
crea cuentas ni modifica usuarios, invitaciones aceptadas o recibos de aceptacion.
Las invitaciones canceladas requieren un reenvio explicito posterior.

La salida JSON contiene `operation_id`, `applied`, `invalidated`,
`audit_sequence` y `audit_head`. Un reintento exacto recupera el mismo recibo con
`applied: false`, sin agregar otro evento. Si ya existen invitaciones pendientes
nuevas, ese reintento se rechaza en lugar de cancelarlas. Un error durante el
commit puede tener resultado incierto: se conserva el UUID, destino y antecedente
originales para comprobarlo. Esta operacion solo invalida invitaciones; no
restaura ni invalida sesiones Redis y no reemplaza el procedimiento de
restauracion de los demas componentes. Su descripcion no acredita una ejecucion
de restauracion ni una activacion productiva.

## Errores

El formato es `{"error":{"code":"...","message":"..."}}`. Los mensajes
publicos no revelan existencia de cuenta, tokens ni detalles del proveedor.

| HTTP / codigo | Interpretacion |
| --- | --- |
| `400 invalid_invitation_request` | UUID, revision, rol o consulta invalidos |
| `400 invalid_json` | Cuerpo no valido o campos extra |
| `400 invitation_rejected` | Capacidad, contrasena, TOTP o intento no aceptados; incluye presupuesto publico agotado |
| `401 invalid_session` | Sesion de Owner invalida; aplican tambien los errores comunes de bearer |
| `403 permission_denied` | La cuenta autenticada no es Owner |
| `404 user_not_found` | La consulta administrativa no encontro la invitacion |
| `409 user_already_exists` | Cuenta o correo incompatible con una invitacion nueva |
| `409 concurrent_modification` | Revision o contenido de operacion en conflicto |
| `413 invitation_body_too_large` | Se superaron 16384 bytes |
| `415 unsupported_media_type` | Falta un unico tipo JSON admitido |
| `503 invitation_unavailable` | Flujo deshabilitado |
| `503 invitation_uncertain` | El recorrido publico no pudo confirmar el resultado |
| `503 server_busy` | Admision HTTP agotada; esperar antes de consultar o reintentar |

Un error de transporte o una desconexion no prueban que una mutacion no se haya
aplicado. Se conserva su identificador y se reconcilia el resultado mediante las
consultas y repeticiones exactas descritas arriba.
