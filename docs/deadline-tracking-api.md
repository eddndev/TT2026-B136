# Seguimiento V2 y vigencia en la API de plazos

Este contrato complementa [la API de plazos](deadlines-api.md). Los bytes y las
invariantes de los recibos se describen en [seguimiento persistido](deadline-tracking-receipts.md).
Las rutas conservan `/api/v1`; la versión de un recibo no es una segunda API.

## Decisión humana y preparación

`change.tracking:{profile,source,calendar}` es obligatorio al registrar o corregir.
Cada dependencia presente requiere `fixed` o `follow`. Una fuente desconocida o
un calendario ausente requiere `undetermined`; el perfil siempre está presente.
La selección de una revisión exacta no determina automáticamente su política.
Atención y retiro no aceptan el campo, ni siquiera nulo, y preservan el seguimiento.

La preparación devuelve `author:{kind:"user",id,email}`, `tracking` y
`receipt_version` además de los campos del cálculo y sus tres compromisos. El
comando normalizado conserva las políticas sólo para registro/corrección. La
autoría procede del usuario autenticado; el servicio revalida UUID, correo y rol
antes de devolver datos o confirmar. Registro y corrección exigen administraciones
activas iguales en cálculo y seguimiento. Atención/retiro conservan las capturas
heredadas, aun si la administración observada por una reevaluación estaba cerrada;
el permiso de escribir sigue comprobándose contra el expediente actual.

Un borrador y una confirmación no prueban vigencia futura. La confirmación y las
consultas exactas devuelven `not_checked`; para consultar vigencia se utiliza GET
actual. Un cliente no debe copiar la fecha calculada al campo operativo.

## Captura del seguimiento

`tracking` es nulo únicamente para V1. En V2 contiene:

- `policies:{profile,source,calendar}` con los valores anteriores.
- `review:{state,reasons}`. `state` es `accepted`, `pending` o
  `legacy_undeclared`. Cada motivo tiene `dependency` (`profile`, `source`,
  `calendar`) y `reason` (`profile_changed`, `source_changed`,
  `dependency_retired`, `policy_undetermined`). Se conserva el orden canónico.
- `observations:{case_id,entries}`: dependencias efectivamente capturadas.
- `administration`: captura administrativa con la misma forma que el material
  del cálculo; puede proceder de otro momento durante una reevaluación técnica.

Cada observación contiene `role`, `family`, `id`, `revision`, `case_id`,
`hearing_id`, `parent_resolution`, `submission_digest` y `evidence_digest`.
Los tres campos opcionales se proyectan explícitamente como nulos cuando faltan.
`role` es `profile`, `source`, `calendar` o `notification_parent`; `family` es
`profile`, `resolution`, `notification`, `hearing_result` o `calendar`.

El padre de una notificación seleccionada, el padre de la notificación observada
y la cabeza observada de la resolución se conservan por separado. El último usa
`notification_parent`; su revisión no reemplaza las referencias históricas. La
API conserva las identidades y los compromisos; la autenticidad de esas
referencias se verifica en el servicio y el almacenamiento, no por el mero formato
hexadecimal de una huella.

## Autoría, recibos y causas

Todo detalle e historia lleva autor etiquetado:

```json
{"kind":"user","id":"00000000-0000-0000-0000-000000000004","email":"owner@example.test"}
```

```json
{"kind":"technical","service":"deadline_reevaluator","policy_version":1}
```

El autor técnico no tiene UUID ni correo ficticios. El responsable del plazo sigue
siendo una captura humana independiente. Las acciones consultables añaden
`reevaluate`; ninguna ruta de comando humano acepta esa acción.

`receipt.version` es `{kind:"v1"}` o
`{kind:"v2",observations_digest,predecessor,cause}`. `predecessor` es nulo al
registrar y, en revisiones posteriores, conserva `submission_digest` y
`capture_digest` del recibo inmediatamente anterior. La historia valida ambas
huellas de las filas adyacentes visibles y rechaza el regreso de V2 a V1.

`cause` es nulo en acciones humanas. En una reevaluación es una de estas formas:

- `{kind:"legacy_bootstrap",job_id,policy_version:1}`. Inicia revisión pendiente
  con todas las políticas sin determinar; no califica el registro por el usuario.
- `{kind:"source_event",job_id,event}`. `event` contiene `sequence`, `family`,
  `source_id`, `revision`, `case_id`, `hearing_id` y `operation_id`.

`sequence` es una **cadena decimal** positiva hasta `9223372036854775807`, para
preservar secuencias mayores que la precisión entera de JavaScript. No convertirla
a `Number`. Un evento puede preceder a la cabeza observada: su identidad y ámbito
deben pertenecer a una observación y su revisión no puede superarla. Un evento de
resolución puede corresponder al padre independiente de una notificación.

## Vigencia de una consulta actual

GET actual y listado verifican la captura histórica y las cabezas pertinentes
bajo la misma transacción autorizada con bloqueo de auditoría. La lista aplica
filtros, cursor y límite antes de cargar las dependencias de cada fila. Confirma
`deadline.list_read` antes de divulgarla; GET actual confirma
`deadline.current_read`. Un fallo de integridad o auditoría devuelve error y no
convierte la lectura en una respuesta histórica de reserva.

`operational` siempre tiene esta forma:

```json
{
  "freshness":"not_checked",
  "checked_at":null,
  "changed_dependencies":[],
  "due_at":null
}
```

| `freshness` | Significado |
| --- | --- |
| `not_checked` | Consulta exacta, confirmación, legado sin políticas o plazo retirado; no afirma vigencia y no entrega fecha operativa. |
| `current` | Las cabezas comprobadas no exigen cambiar el seguimiento capturado según sus políticas. |
| `changed` | Las cabezas exigen actualizar o revisar el seguimiento; se retiene sólo el cálculo histórico. |

Para `current`/`changed`, `checked_at` registra el instante de comprobación dentro
de la transacción. `changed_dependencies` enumera dependencias únicas en orden
perfil/fuente/calendario; sólo tiene elementos en `changed`. Los instantes llevan
`unix_seconds`, `nanosecond` y `offset_seconds` como el resultado del cálculo.

Una fecha operativa sólo se entrega con plazo activo, revisión aceptada y
vigencia `current`, cuando existe resultado calculado. `pending` conserva su
resultado anterior pero no lo habilita para seguimiento; puede ser `current` si
las cabezas ya corresponden a sus observaciones pendientes. Vigencia y aceptación
son dimensiones distintas. Un cambio ordinario de una dependencia `fixed` puede
conservar `current`; su retiro sí exige atención. Un expediente cerrado conserva
la lectura y no suspende por sí mismo un término jurídico.

La consulta compara evidencia actual directamente. Una cola vacía, un trabajo
antiguo pendiente o una finalización sin cambios no demuestran vigencia. GET no
recalcula ni modifica revisiones, trabajos o capturas históricas. Una comprobación
es válida para su instante; una respuesta almacenada en pantalla no constituye
monitoreo continuo. Agenda y avisos deben volver a autorizar y comprobar vigencia
al usar estos datos.

## Resúmenes

Cada fila de lista conserva `receipt_kind` (`v1`/`v2`), `review_state`,
`calculation_due_at`, `calculation_blocked` y `operational`, además de identidad,
título, responsable, atención y estado. El resumen está vinculado a la misma
captura verificada que produjo su proyección operativa. `calculation_blocked`
sólo expresa ausencia de fecha en el cálculo histórico; no sustituye revisión,
retiro ni vigencia. Los campos anteriores `due_at` y `blocked` ya no aparecen en
la raíz de la fila.
