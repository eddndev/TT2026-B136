# Agenda autorizada de audiencias y vencimientos

`GET /api/v1/agenda` reúne audiencias ordinarias, audiencias propias de recursos
y fechas operativas de plazos. La tercera familia tiene implementación local;
su presentación en Qadra y aceptación integrada aún están pendientes. Usa
sesión bearer revocable y respuestas `Cache-Control: no-store`. Owner consulta
el despacho; Litigator y Paralegal sólo expedientes asignados. Client recibe
403. La ruta de audiencias `GET /api/v1/hearings` conserva su contrato anterior.

## Consulta

| Parámetro | Valores |
| --- | --- |
| `from`, `until` | Requeridos: RFC 3339 UTC terminado en `Z`, segundos enteros y años 0001–9999. Intervalo positivo `[from,until)` de hasta 366 días. |
| `kind` | `all` (predeterminado), `hearing`, `deadline` o `resource_hearing`. |
| `hearing_status` | `scheduled` (predeterminado), `cancelled` o `all`. Para `kind=deadline` sólo se acepta `scheduled`; `resource_hearing` rechaza `cancelled`. |
| `limit` | Entre 1 y 100, predeterminado 20. Limita actividades emitidas. |
| `cursor` | Continuación opaca devuelta por esta ruta, máximo 512 bytes ASCII. Se omite en la primera página. |

Campos desconocidos, duplicados, límites inválidos y cursores de otros filtros
producen 400. Ejemplo:

```http
GET /api/v1/agenda?from=2026-09-01T00:00:00Z&until=2026-10-01T00:00:00Z&kind=all&hearing_status=scheduled&limit=20
Authorization: Bearer <session>
```

## Respuesta

La envoltura contiene `from`, `until`, `kind`, `hearing_status`, `checked_at`,
`items`, `complete` y `next_cursor`. `checked_at` es la observación común de la
página bajo la transacción auditada. Los instantes usan
`{unix_seconds,nanosecond,offset_seconds}`; el instante común `at` y la observación
son UTC. Cada elemento es uno de los siguientes:

- `{kind:"hearing",at,hearing}`: `hearing` reutiliza el resumen de
  [audiencias](hearings-api.md), incluida fecha original, revisión y expediente.
- `{kind:"deadline",at,case_title,case_reference,case_status,deadline}`:
  `deadline` reutiliza el resumen de [plazos](deadline-tracking-api.md).
  Su observación coincide con `checked_at` y `at` deriva de `operational.due_at`.

- `{kind:"resource_hearing",at,case_title,case_reference,case_status,resource_hearing}`:
  resumen de la [audiencia propia del recurso](resource-hearings.md). Contiene
  `case_id`, `resource_id`, `id`, `revision`, `kind`, `scheduled_at`, `modality`,
  `participant_count`, `association_id` y `capture_digest`. Conserva la asociación
  inicial y la huella verificada; no inventa estado ni etapa ordinaria.
  `scheduled_at` es RFC 3339 con el desfase original.

`kind=hearing` conserva sólo audiencias ordinarias. `all` reúne las tres familias;
con `hearing_status=cancelled` excluye las audiencias propias y conserva la
selección anterior de audiencias ordinarias canceladas y plazos operativos.
Desvincular la asociación, archivar el recurso o cerrar administrativamente el
expediente no cancela una audiencia propia ni la elimina de la agenda autorizada.

Las fechas originales conservan sus desfases. No se incluyen notas, sedes o
listas de participantes de la audiencia. Un plazo retirado, bloqueado, legado,
pendiente o con dependencias cambiadas no aporta actividad; su cálculo histórico
permanece disponible en el módulo de plazos. Atención declarada y cierre
administrativo no eliminan por sí solos una fecha vigente autorizada.

## Orden y continuación

El orden es segundo UTC, nanosegundo, familia (`hearing`, `deadline`, `resource_hearing`, en ese orden) y
UUID. Un identificador igual en las tres familias representa tres actividades.
El servidor examina como máximo 100 candidatos por llamada y puede devolver
menos de `limit`. El cursor avanza hasta el último candidato examinado; no salta
una actividad elegible que no haya devuelto.

`complete=false` exige `next_cursor`, incluso con `items=[]`: quedan candidatos
por consultar, pero no se promete otra actividad. `complete=true` exige cursor
nulo e indica agotamiento en esa lectura. Los clientes conservan filas
acumuladas, muestran que la consulta es parcial y permiten cargar otra página.

Cada petición reautoriza y observa el estado actual. El cursor no reserva un
snapshot; cambios entre páginas pueden mover una actividad. Qadra conserva la
revisión mayor recibida por `(kind,id)` y ofrece actualizar desde el inicio.
Cambiar intervalo o filtros descarta la continuación y las respuestas tardías.

La versión de transporte `a1` codifica límites, filtros y clave; el cliente debe
tratarla como continuación, no como una credencial ni una fecha de vencimiento.
Los rangos anteriores cero y uno se conservan; la audiencia propia añade el rango dos.

## Consistencia y errores

Una lectura exitosa registra `agenda.read`, aun vacía. Fecha, captura, estado
actual y administración se verifican dentro de la misma transacción. Corrupción
o fallo de auditoría rechazan la respuesta completa; no entregan filas parciales.
El servicio reautentica después de leer y rechaza cambios del principal.
La audiencia propia reconstruye dentro de esa transacción su captura, asociación
inicial y marcador de origen. Una proyección alterada o un origen ausente rechaza
la página completa; no se transforma en una omisión ni una audiencia ordinaria.

401 identifica sesión ausente o inválida; 403, permiso insuficiente. Consultar
un rango sin membresías devuelve una página vacía autorizada. Se conservan los
errores generales de saturación y almacenamiento de [la API HTTP](http-api.md).

La decisión y límites de implementación están en
[ADR-0038](adr/0038-authorized-combined-agenda.md). Las vistas de Qadra sólo
transforman el intervalo y su presentación; no activan términos ni envían avisos.
