# Asociaciones entre recursos y actividades existentes

Este contrato vincula un recurso o uno de sus actos con una revisión exacta de
una audiencia o un plazo ya registrados en el mismo expediente. La asociación
conserva su propia historia y recibo. No crea audiencias contextuales, activa
plazos, cambia su cálculo ni genera otro episodio de alerta.

Las asociaciones pueden coexistir: una actividad puede vincularse con varios
recursos. Quitar un vínculo es una revisión organizativa, sin borrar evidencia,
cancelar la audiencia, retirar el plazo, registrar atención o resolver avisos.
El archivo del recurso tampoco realiza esas operaciones.

## Autorización y confirmación

| Principal vigente | Lectura | Vínculo y desvinculación |
| --- | --- | --- |
| Owner | Todos los expedientes | Sí |
| Litigator asignado | Su expediente | Sí |
| Paralegal asignado | Su expediente | No |
| Client | No | No |

La aplicación reautentica el principal completo; el adaptador vuelve a comprobar
cuenta y membresía vigentes. Los datos se entregan después de confirmar la
lectura auditada. Un expediente cerrado permite consulta autorizada e historia,
pero impide mutaciones nuevas. La repetición exacta de una operación ya
confirmada conserva su recibo, previa autorización actual.

Vincular exige recurso actual activo. Desvincular permite un recurso archivado,
si el expediente sigue activo y coincide la revisión actual esperada. La
revisión histórica elegida puede ser distinta de esa cabeza actual. Un vínculo
desvinculado no se reactiva; una asociación posterior utiliza otra identidad.

## Rutas

Base: `/api/v1/cases/{case_id}/procedural-resources/{resource_id}/activities`.

| Método y sufijo | Respuesta |
| --- | --- |
| GET base | Página de vistas autorizadas |
| GET `/{association_id}` | Cabeza de la asociación y actividad actual separada |
| GET `/{association_id}/revisions/{revision}` | Asociación histórica exacta y actividad actual separada |
| GET `/{association_id}/history` | Revisiones históricas, sin proyección operativa |
| POST `/prepare` | Borrador revisable, sin reserva ni escritura |
| POST base | Confirmación de vínculo, HTTP 201 |
| POST `/{association_id}/unlink` | Confirmación de desvinculación, HTTP 201 |

Todas las rutas requieren bearer y usan `Cache-Control: no-store`, el límite
compartido de peticiones y el presupuesto compartido de trabajo bloqueante.
Los cuerpos tienen máximo técnico de 16 KiB. No contienen documentos ni copias
de los objetos vinculados: éstos se resuelven y verifican en el servidor.

La lista admite `limit` de 1 a 100, predeterminado 20; `after_id` exclusivo;
`kind=hearing|deadline` y `status=linked|unlinked` opcionales. Sin filtro de
estado incluye ambos. Los filtros se aplican a las cabezas antes de paginar.
La historia admite `limit` de 1 a 20, predeterminado 20, y
`before_revision` exclusivo, en orden descendente. Claves desconocidas,
duplicadas y consultas en rutas sin filtros se rechazan.

## Comando y selección exacta

El cuerpo de preparación es el comando. Los POST de confirmación reciben
`{"command": comando, "expected_submission_digest": "sha256 hexadecimal"}`.
Ejemplo de comando para una audiencia:

```json
{
  "case_id": "00000000-0000-0000-0000-000000000001",
  "resource_id": "00000000-0000-0000-0000-000000000002",
  "association_id": "00000000-0000-0000-0000-000000000003",
  "operation_id": "00000000-0000-0000-0000-000000000004",
  "expected_resource_revision": 5,
  "change": {
    "action": "link",
    "expected_revision": 0,
    "resource": {
      "id": "00000000-0000-0000-0000-000000000002",
      "revision": 2,
      "capture_digest": "0707070707070707070707070707070707070707070707070707070707070707"
    },
    "act": null,
    "target": {
      "kind": "hearing",
      "id": "00000000-0000-0000-0000-000000000005",
      "revision": 1,
      "submission_digest": "0707070707070707070707070707070707070707070707070707070707070707"
    }
  }
}
```

Los identificadores y digests del ejemplo son sintéticos; deben sustituirse por
referencias verificadas del expediente. `case_id`, `resource_id` y, para
mutation por identidad, `association_id` deben coincidir con la URL antes de
invocar el puerto. Los UUID son canónicos en minúsculas y los digests SHA-256
son 64 caracteres hexadecimales en minúsculas. Las revisiones son u32 positivos;
solamente la revisión esperada del nuevo vínculo es cero.

`act` es obligatorio y nullable. Si existe, contiene
`{id, revision, resource_revision, capture_digest}`: el digest pertenece a la
revisión del recurso que contiene ese acto exacto. Puede seleccionarse un
recurso R1 y un acto declarado en R2; no se infiere cronología o efecto jurídico.
Para plazo, `target` contiene
`{kind:"deadline", id, revision, capture_digest}`.

Desvincular conserva las referencias y fuentes anteriores. Su `change` contiene
`{action:"unlink", expected_revision, reason}`; no admite reemplazos de fuente.
`expected_resource_revision` sigue en el nivel superior y comprueba la cabeza
actual. La razón usa el texto declarado acotado existente. Campos adicionales,
ausentes, duplicados o representaciones posicionales son inválidos.

## Respuestas históricas y vigentes

`Association` contiene `case_id`, `resource_id`, `id`, `revision`, `selection`,
`status`, `reason`, `sources`, `receipt`, `recorded_by`, `recorded_at`,
`recorded_administration` y `recorded_resource_head`.

- `selection` conserva `resource`, `act` y `target` exactos del comando.
- `sources.resource` reutiliza el detalle de recurso existente.
  `sources.act` es null o el detalle de la revisión que contiene el acto.
  `sources.target` es `{kind, record}` con el detalle histórico existente de
  audiencia o plazo. Véanse [recursos](procedural-resources-api.md),
  [audiencias](hearings-api.md) y [plazos](deadlines-api.md).
- `receipt` contiene `operation_id`, `action`, `expected_revision`,
  `expected_resource_revision`, `previous`, `submission_digest` y
  `capture_digest`. `previous` es null o `{revision, capture_digest}`.
- `recorded_by` conserva `{id, email}` y `recorded_at` usa RFC 3339 UTC.
  Administración y cabeza del recurso son capturas, no su estado actual.

GET detalle y revisión exacta devuelven:

```text
{
  association: Association,
  checked_at: {unix_seconds, nanosecond, offset_seconds: 0},
  current_target: {kind: "hearing" | "deadline", record: detalle_actual}
}
```

La asociación conserva la revisión seleccionada aunque la actividad avance.
`current_target.record` reutiliza íntegramente la proyección actual existente;
para plazo carga su cabeza real y verifica dependencias bajo la misma
transacción. Un `operational.checked_at` no nulo coincide con `checked_at`.
Los casos retirados o legados conservan su proyección no comprobada y fecha
operativa nula. Nunca se usa `calculation.result.due_at` histórica como sustituto.
La [API de seguimiento](deadline-tracking-api.md) define esos estados.

La lista devuelve
`{case_id, resource_id, associations:[vista], has_more, next_after_id}`.
La historia devuelve
`{case_id, resource_id, association_id, revisions:[Association], has_more,
next_before_revision}`. La continuación es null al terminar. Cada petición
obtiene una lectura nueva; el cursor no reserva una instantánea entre páginas.

La preparación devuelve `case_id`, `resource_id`, `command`, `result_revision`,
`selection`, `status`, `sources`, `previous`, `recorded_by`,
`observed_administration`, `observed_resource_head` y `submission_digest`.
La confirmación devuelve `Association`, sin inventar vigencia. Para actualizar
el estado operativo tras confirmar, consultar el detalle.

## Errores y respuestas inciertas

| HTTP | Código |
| --- | --- |
| 400 | `invalid_resource_activity`, `invalid_query`, `invalid_json` |
| 401 / 403 | `invalid_session` / `permission_denied` |
| 404 | `resource_activity_not_found` o error del padre autorizado |
| 409 | `case_closed` |
| 409 | `resource_activity_revision_conflict`, `resource_activity_resource_revision_conflict` |
| 409 | `resource_activity_operation_conflict`, `resource_activity_submission_mismatch` |
| 409 | `resource_activity_resource_archived`, `resource_activity_state_unchanged`, `resource_activity_source_mismatch` |
| 413 | `resource_activity_body_too_large` |
| 503 | `server_busy` |
| 500 | `internal_error`, sin texto privado de persistencia |

Preparar no reserva una operación. Ante conflicto se conserva el borrador y se
acepta explícitamente una nueva base antes de otra confirmación. Ante una
respuesta incierta se conservan identidad, operación, comando y digest; se
reconcilia con detalle/historia y, si hace falta, se repite exactamente esa
operación. No se genera una clave nueva automáticamente.

La confirmación revalida autorización, administración, cabeza del recurso,
base de asociación y fuentes históricas; asociación, recibo y auditoría se
confirman juntos. Vincular evidencia histórica no readmite documentos ni
sustituye una fuente por su cabeza. El estado de ejecución de pruebas y la
aceptación integrada se registran en el [informe de verificación](verification-report.md).

La aceptación reproducible se incorpora a `scripts/api-demo.sh` mediante
`scripts/api-resource-activities-demo.py`: conserva las audiencias y plazos
existentes, crea recursos propios para las asociaciones y comprueba las
respuestas después de restaurar las dos tablas de asociaciones. Sólo normaliza
el instante de lectura después de comprobar su formato y vínculo con la
proyección operativa; las capturas y recibos se comparan completos. La presencia
del guion no acredita su ejecución.
