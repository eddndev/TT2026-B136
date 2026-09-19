# Alertas personales de audiencias y plazos

El router HTTP y Qadra implementan preferencias personales, consulta de bandeja,
detalle y lectura explícita. Las rutas se construyen mediante `web::alert_router`
sobre `AlertWorkflow`. La composición con persistencia PostgreSQL, generación
periódica y transporte de correo conserva su aceptación integrada pendiente.
Las pruebas del cliente y del navegador con HTTP controlado no acreditan ese
recorrido real. La decisión está en
[ADR-0039](adr/0039-durable-activity-alerts.md).

## Acceso y alcance

Todas las rutas requieren sesión bearer revocable y responden con
`Cache-Control: no-store`. Owner, Litigator y Paralegal acceden exclusivamente
a sus preferencias y alertas; Client recibe 403. Los comandos no admiten
`user_id`, destinatarios, correos arbitrarios ni suscripciones a expedientes.

La política de generación dirige las audiencias a miembros staff activos del
expediente; Owner sólo es destinatario de audiencia cuando es miembro. Los
plazos se dirigen al responsable actual que siga activo y autorizado. La
participación procesal no concede acceso ni determina una cuenta receptora.
La lectura exige destinatario propio y autorización actual sobre el expediente.
Un cierre administrativo no cancela por sí solo los avisos ni sus lecturas.

`AlertService` autentica, invoca el puerto, valida identidad y coherencia de la
respuesta y reautentica el principal completo. `AlertStore` exige autorización
y auditoría transaccionales, filtrando permisos antes de paginar. La aceptación
real de ese adaptador y del runtime se registra por separado en el
[informe de verificación](verification-report.md).

## Rutas y representación común

| Método y ruta | Resultado 200 |
| --- | --- |
| `GET /api/v1/alert-preferences` | `{preferences}` |
| `PUT /api/v1/alert-preferences` | `{preferences}` confirmado para el comando |
| `GET /api/v1/alerts` | `{checked_at,alerts,has_more,next_cursor}` |
| `GET /api/v1/alerts/{id}` | `{checked_at,alert}` |
| `POST /api/v1/alerts/{id}/read` | `{operation_id,checked_at,alert}` |

Los campos usan `snake_case`. Los UUID son canónicos en minúsculas. Todos los
instantes conservan componentes enteros y UTC, con años 1 a 9999:

```json
{"unix_seconds":1767225600,"nanosecond":123456789,"offset_seconds":0}
```

`nanosecond` está entre 0 y 999999999. No se convierte a milisegundos ni a
RFC 3339 para comparar o paginar. Las revisiones son enteros sin signo de
32 bits; una revisión de origen siempre es positiva.

Los cuerpos requieren un único tipo de contenido JSON, admiten como máximo
16 KiB y rechazan campos desconocidos, campos duplicados y contenido posterior
al objeto. `null` no reemplaza los campos obligatorios de un comando.

## Preferencias personales

Ejemplo de `PUT /api/v1/alert-preferences`:

```json
{
  "operation_id":"00000000-0000-4000-8000-000000000005",
  "expected_revision":0,
  "values":{
    "hearing_upcoming":{"lead_hours":[48,24],"channels":{"internal":true,"email":true}},
    "deadline_upcoming":{"lead_hours":[48,24],"channels":{"internal":true,"email":true}},
    "overdue_unattended":{"internal":true,"email":true},
    "review_required":{"internal":true,"email":true},
    "due_changed_soon":{"internal":true,"email":true}
  }
}
```

Cada familia admite hasta ocho anticipaciones distintas, enteras entre 1 y
720 horas. La respuesta las ordena de mayor a menor; una lista vacía desactiva
la proximidad de esa familia. Los cinco grupos y sus dos canales son explícitos.
Desactivar ambos canales no elimina por sí mismo la historia de una alerta.

La propiedad `preferences` contiene:

| Campo | Contrato |
| --- | --- |
| `user_id` | Cuenta autenticada propietaria. |
| `revision` | Cero para valores iniciales; positiva tras una escritura. |
| `values` | Los cinco grupos mostrados en el comando. |
| `updated_at` | Instante de escritura o `null` para revisión cero. |
| `receipt` | `{operation_id,expected_revision}` o `null` para revisión cero. |
| `email_transport` | `ready` o `disabled`; describe disponibilidad del transporte. |

La revisión cero conserva exactamente anticipaciones 48/24 y ambos canales
activos en todos los grupos, sin fecha ni recibo inventados. Una respuesta
persistida exige `revision = receipt.expected_revision + 1`, fecha presente
y valores correspondientes al comando confirmado. El transporte deshabilitado
no convierte una preferencia de correo activa en un envío satisfactorio.

Una nueva escritura usa la revisión observada; `expected_revision=4294967295`
no permite otro incremento. El mismo identificador de operación y contenido
debe devolver su resultado original, incluso si existen revisiones posteriores.
Reutilizarlo con otro contenido produce conflicto. Una respuesta incierta se
contrasta con recibo, revisión y valores: una lectura actual de otra operación
no demuestra qué ocurrió con el envío. Qadra conserva el comando y exige una
decisión explícita antes de volver a enviarlo o reemplazar valores nuevos.

## Bandeja, filtros y continuación

| Parámetro | Valores |
| --- | --- |
| `limit` | De 1 a 100; predeterminado 20. |
| `read` | `all` (predeterminado) o `unread`. |
| `state` | `active` (predeterminado) o `all`. |
| `cursor` | Continuación devuelta por la ruta; se omite en la primera petición. |

La consulta no acepta filtro por familia ni por otra cuenta. `unread` sólo
devuelve `read_at=null`; `active` sólo devuelve alertas activas. El orden es
descendente por `(created_at.unix_seconds, created_at.nanosecond, id)`.
Distintas alertas del mismo recurso permanecen separadas por su identificador.

`has_more=true` exige un cursor, incluso si `alerts=[]`: un recorrido acotado
puede no haber encontrado filas visibles y mantener trabajo por consultar.
`has_more=false` exige `next_cursor=null`. El tamaño de una página no representa
un total de alertas ni de pendientes del despacho.

La continuación usa este formato canónico, limitado a 128 bytes ASCII:

```text
a1:<unix_seconds>:<nanosecond de nueve digitos>:<uuid>:<read>:<state>
```

Los segundos usan decimal mínimo con signo negativo sólo cuando corresponda;
no admiten `+`, ceros iniciales ni `-0`. El cursor queda ligado a ambos filtros,
avanza estrictamente respecto al cursor recibido y no supera la última clave
examinada ni `checked_at`. No es una credencial ni reserva una instantánea entre
peticiones. Cambiar filtros descarta la continuación; actualizar consulta desde
el principio. Qadra acumula por identidad de alerta y descarta respuestas tardías.

## Alerta y origen capturado

Cada objeto `alert` contiene:

| Campo | Contenido |
| --- | --- |
| `id`, `recipient_id`, `occurrence_id` | Identidad de alerta, cuenta destinataria y ocurrencia. |
| `subject` | `{kind:"hearing"|"deadline",case_id,id}`. |
| `subject_title`, `case_title`, `case_reference` | Contexto capturado; límites de 200, 200 y 100 escalares Unicode. Texto no vacío, sin controles y sin espacios sobrantes en los extremos. |
| `kind` | Uno de los cuatro motivos descritos abajo. |
| `origin` | `{revision,evidence_digest}`; revisión exacta y SHA-256 hexadecimal minúscula de 64 caracteres. |
| `trigger_at`, `created_at` | Umbral y fecha de generación. |
| `read_at` | Instante de lectura o `null`. |
| `state` | Estado de resolución del aviso. |
| `email` | Estado independiente del canal de correo. |

| Variante `kind` | Campos adicionales y sentido |
| --- | --- |
| `upcoming` | `lead_hours`, `activity_at`: proximidad de audiencia o plazo. |
| `overdue_unattended` | `due_at`: vencimiento de plazo sin atención declarada. |
| `review_required` | Ninguna fecha: revisión humana requerida de un plazo. |
| `due_changed_soon` | `previous_due_at`, `current_due_at`: transición comprobada de fecha próxima de un plazo. |

Sólo `upcoming` admite audiencias. Las fechas y la evidencia pertenecen a la
generación del aviso; `active`, `accepted` en correo y un título capturado no
acreditan vigencia operativa actual. La huella conserva la referencia de origen;
su forma no constituye una verificación criptográfica hecha por el navegador.
Qadra vuelve a consultar alerta y expediente antes de abrir `origin.revision`
exacta. Consultar la cabeza actual es una acción separada del detalle histórico.

Las respuestas exigen `trigger_at <= created_at <= checked_at`. Lectura,
resolución y aceptación por el proveedor, cuando existen, están dentro de
`[created_at,checked_at]`. Para proximidad, `trigger_at` es `activity_at` menos
las horas declaradas y la generación pertenece a `[trigger_at,activity_at)`.
Para vencimiento sin atención, `trigger_at=due_at`. Un cambio exige fechas
distintas; la nueva puede adelantarse al pasado y no se reemplaza por otra fecha.

## Resolución, correo y lectura

`state` es `{kind:"active"}` o `{kind:"resolved",at,reason}`. Los motivos de
resolución son `superseded`, `attention_recorded`, `target_retired`,
`cancelled_hearing` y `no_longer_eligible`. Resolver el aviso y leerlo son
operaciones distintas; un aviso resuelto puede seguir sin leer.

`email` admite `{kind:"disabled"|"pending"|"sending"|"failed"|"unknown"|"cancelled"}`
o `{kind:"accepted",accepted_at}`. `accepted` significa **Aceptado por proveedor**;
no prueba entrega al buzón ni lectura del correo. La bandeja no expone dirección
de envío, texto de errores del proveedor ni una acción pública de reintento.
El mensaje externo previsto es genérico y enlaza al acceso a Qadra, sin datos
del recurso. Su transporte y recuperación requieren aceptación propia.

`POST /api/v1/alerts/{id}/read` sólo recibe:

```json
{"operation_id":"00000000-0000-4000-8000-000000000005"}
```

La respuesta liga ese identificador y la alerta solicitada, con `read_at` no
nulo. La repetición idempotente conserva el instante de lectura original; no
declara atención del plazo, no confirma aplicabilidad ni modifica su historial.
Abrir el recurso tampoco marca la alerta como leída. Ante respuesta incierta,
**Comprobar lectura** consulta el detalle sin repetir automáticamente la escritura.

## Errores

Se conserva la envoltura `{error:{code,message}}` de [la API general](http-api.md#errores).

| Estado | Código y situación |
| --- | --- |
| 400 | `invalid_json`: tipo de contenido, cuerpo, campos o tipos JSON inválidos. `invalid_alert`: UUID no canónico o forma inválida de consulta. |
| 401 | `invalid_session`: sesión ausente, inválida o principal cambiado. |
| 403 | `permission_denied`: rol sin permiso. |
| 404 | `alert_not_found`: alerta inexistente, ajena o sin acceso actual, sin distinguir estos casos. |
| 409 | `alert_revision_conflict` o `alert_operation_conflict`. |
| 413 | `alert_body_too_large`: cuerpo mayor de 16 KiB. |
| 422 | `invalid_alert`: valores, límites, filtros o cursor semánticamente inválidos. |
| 500 | `internal_error`: almacenamiento o respuesta incoherente del puerto, sin diagnósticos internos. |

La saturación y los fallos transitorios conservan los errores generales del
runtime HTTP. Ninguna respuesta incierta autoriza reenvíos automáticos.
