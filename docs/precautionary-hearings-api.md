# API de contexto y audiencias cautelares

Estado: router local implementado y compuesto en `serve` sobre los puertos
autorizados de aplicación. Comparte el presupuesto de trabajo de la API.
Agenda y alertas PostgreSQL/HTTP tienen evidencia focal propia. La consulta
exacta Qadra desde ambas aprobó recorridos de escritorio/móvil con HTTP
controlado. Los formularios Qadra de imposición/revisión, reprogramación, cancelación
y recuperación de operación aprobaron aceptación focal controlada. La aceptación
HTTP integrada con reinicio/restauración permanece pendiente. Este documento describe contexto y convocatorias; decisiones
y rectificaciones tienen sus contratos propios en [la API HTTP](http-api.md). Véanse
[el alcance](precautionary-hearings-scope.md),
[ADR-0071](adr/0071-declared-precautionary-hearings-and-measures.md) y
[la evidencia ejecutada](verification-report.md).

Las citas registran declaraciones y revisiones. Programar, reemplazar o cancelar
una audiencia no impone, modifica ni extingue por sí mismo una medida judicial.

## Rutas y acceso

Todas las rutas usan `Authorization: Bearer <token>` y respuestas con
`Cache-Control: no-store`. Base: `/api/v1/cases/{case_id}`.

| Método | Sufijo | Respuesta |
| --- | --- | --- |
| GET | `/precautionary-context` | Contexto observado, HTTP 200 |
| POST | `/precautionary-hearings/prepare` | Revisión normalizada y dos digests, HTTP 200 |
| POST | `/precautionary-hearings/submit` | Operación original completa, HTTP 201 |
| GET | `/precautionary-hearings` | Página de revisiones actuales, HTTP 200 |
| GET | `/precautionary-hearings/{hearing_id}` | Revisión actual y prefijo completo, HTTP 200 |
| GET | `/precautionary-hearings/{hearing_id}/revisions/{revision}` | Revisión exacta y su prefijo, HTTP 200 |
| GET | `/precautionary-hearings/operations/{operation_id}` | Operación original, HTTP 200 |

Owner y Litigator asignado pueden escribir; también Paralegal asignado puede
consultar. Client no tiene acceso. Los servicios y almacenes comprueban sesión,
principal completo y pertenencia vigente. Un expediente cerrado permite contexto,
lecturas y replay autorizados; las mutaciones nuevas requieren contexto activo.
La preparación no reserva una operación ni conserva permisos para confirmar.

Sólo la lista acepta `limit` y `after_id`: límite predeterminado 10, rango 1..20
y cursor UUID exclusivo. Rechaza claves repetidas o desconocidas. Las demás
rutas rechazan parámetros de consulta; la revisión exacta de audiencia no lleva
digest en la URL.

## Transporte y comando

POST requiere un único Content-Type JSON y un cuerpo de hasta 128 KiB. Se
rechazan objetos posicionales, claves repetidas o desconocidas y contenido tras
el objeto JSON. UUID usa su forma canónica con guiones y minúsculas; SHA-256 usa
64 caracteres hexadecimales en minúsculas. Versiones y revisiones son enteros
positivos dentro de `u32`; la revisión resultante tampoco puede desbordarse.

`prepare` recibe un objeto con `case_id`, `operation_id`, `hearing_id` y `change`.
El expediente del cuerpo debe coincidir con la ruta. `change` tiene esta forma:

| `action` | Otros campos obligatorios |
| --- | --- |
| `schedule` | `context`, `values` |
| `replace` | `expected_revision`, `expected_capture_digest`, `context`, `values`, `reason` |
| `cancel` | `expected_revision`, `expected_capture_digest`, `reason` |

`context` contiene `administration_revision`, `stage_revision` y
`context_digest`, obtenidos de la consulta de contexto. Cancel no acepta un
contexto o valores del cliente: conserva la programación y fuentes anteriores.
Replace y Cancel requieren la cabeza exacta actual de la audiencia. Una nueva
acción usa otro UUID de operación; repetir el mismo comando sirve para recuperar
su operación original, sin producir una revisión adicional.

`values` contiene todos estos campos:

- `purpose`: `imposition` o `review`.
- `scheduled_at`: RFC 3339 con segundos completos, `T` mayúscula y `Z` o un
  desfase explícito de minutos entre -14:00 y +14:00. Conserva el desfase;
  rechaza fracciones, segundos intercalares, `-00:00` y años locales o UTC fuera
  de 1..9999. Una fecha pasada no acredita celebración.
- `modality`: `in_person` o `videoconference`; `venue`: texto de sede o conexión.
- `note`: texto o `null` explícito; omitir el campo es inválido.
- `participants`: 0..32 objetos `{participant_id, revision}`.
- `scheduling_basis`: `{statement, support, locator}`, donde `support` es
  `{document_id, version, digest}` y selecciona un PDF/DOCX exacto admitido.
- `review_targets`: objetos `{id, revision, capture_digest}`. Imposition exige
  lista vacía; Review exige 1..32 selecciones exactas de medida.

Las selecciones se ordenan por UUID y no repiten identidad dentro de una lista.
Los participantes nuevos requieren revisión actual activa del expediente; un
reemplazo puede conservar la selección exacta anterior aunque esté archivada.
Review admite M1, M2 y C1 capturados como Valid, incluidos objetivos históricos
y terminales. No exige la cabeza actual de la medida ni sustituye sus fuentes.
Se verifica el propietario completo y su ascendencia, contexto y reloj exactos.

Los textos se normalizan mediante los tipos del dominio: espacio exterior y
saltos CRLF, sin normalización Unicode interior. `venue` admite 1..500 caracteres;
notas, motivos, declaración y localizador usan el límite de 1000, con LF como
único control permitido en las notas. HTTP no acepta actor, reloj de captura,
fuentes completas, pruebas de propietarios ni indicadores de verificación.

## Confirmación y recibos

`submit` recibe `{command, expected_submission_digest, expected_review_digest}`.
`command` es el objeto completo normalizado, incluido `case_id`. Ambos digests
deben coincidir con la revisión que se confirma; confirmar sólo las instrucciones
no confirma fuentes o procedencia cambiadas. El servidor vuelve a comprobar
autorización y material antes de la confirmación atómica con auditoría.

Prepare devuelve `case_id`, `actor` (`id`, `email`, `role`), `command`,
`resolved_values`, `result_revision`, `status`, `scheduling_context`,
`observed_context`, `sources`, `participants`, `submission_digest` y
`review_digest`. `status` es `scheduled` o `cancelled`. `sources` conserva fichas
históricas completas y soporte `{document_id, version, digest, name, format,
policy}`; los participantes derivados conservan `snapshot` y `overview`.

Submit y las consultas de detalle devuelven `{capture, history}`. La captura
contiene `{review, recorded_at, capture_digest}`. La historia contiene el origen,
el prefijo `captures` y `record_history`, con propietarios completos en
`records.judicial.groups`, `records.administrative` y `decisions`. Conserva las
familias G1/G2/A, hermanos, fuentes, raíces y enlaces de reemplazo reales; no
reexpande recursivamente propietarios. Los relojes UTC mantienen nanosegundos.
Los límites de prueba son independientes: 256 propietarios/8192 miembros y
256 capturas de audiencia/8192 objetivos, sin truncamiento.

La lista devuelve `{case_id, items, has_more, next_after_id}`; cada elemento es
una operación completa. Contexto devuelve `{case_id, administration, stage,
stage_administration, context_digest, expectation}`. Incluye revisiones y
procedencia originales; observar contexto no concede permiso de escritura.

Ante una respuesta incierta, conservar comando y ambos digests y consultar la
operación exacta. No reconstruir el envío a partir de valores parecidos. Replay
devuelve HTTP 201 con captura, actor, reloj y prueba originales; una consulta de
operación devuelve HTTP 200. No hay reintento automático ni actualización de
recibos antiguos a otra familia.

## Errores

El cuerpo es `{"error":{"code":"...","message":"..."}}`. El cliente debe
usar `code`, no analizar el texto del mensaje.

| HTTP | Código o familia |
| --- | --- |
| 400 | `invalid_json` o `invalid_precautionary_hearing`: transporte o comando inválido |
| 401 / 403 | Errores existentes de sesión, rol o acceso al expediente |
| 404 | `precautionary_hearing_not_found` |
| 409 | `precautionary_hearing_operation_conflict`, `precautionary_hearing_submission_mismatch`, `precautionary_hearing_review_mismatch` |
| 413 | `precautionary_hearing_body_too_large` |
| 422 | `precautionary_hearing_incomplete_history`; validación/admisión rechazada por aplicación |
| 500 | `internal_error`, mensaje fijo `internal application error` |

El mapper común también define los prefijos `measure_decision_` y
`measure_administrative_`, con los mismos sufijos `not_found` (404),
`operation_conflict`, `submission_mismatch`, `review_mismatch` (409) e
`incomplete_history` (422). La familia administrativa añade `stale_head` y
`known_dependants` (409). Sus rutas están compuestas con evidencia focal propia.
Las inconsistencias almacenadas, errores de persistencia/auditoría y respuestas
del puerto que contradigan los selectores solicitados devuelven el 500 genérico,
sin diagnósticos SQL ni material privado.

## Consulta exacta en Qadra

Agenda permite filtrar convocatorias cautelares programadas/canceladas y abre
la revisión seleccionada. La bandeja de alertas abre la revisión de origen del
aviso; no reemplaza una captura histórica por la cabeza posterior. El panel
muestra horario y desfase, propósito, estado, sede, participantes, soporte y
las capturas hasta esa revisión. Mantiene filtros al cerrar.

Ambas aperturas reautorizan el expediente y vinculan identidad, revisión y
digest a la selección original. Un 403/404 retira sus filas y detalle. Cambiar
filtros o sesión invalida consultas pendientes; no hay reintentos automáticos.
La consulta no marca lectura del aviso ni presenta acciones de escritura.
Los formularios de convocatoria/decisión/rectificación continúan pendientes.
