# Recursos procesales declarados

La API mantiene recursos organizativos de revocación y apelación independientes
de la etapa del expediente. Conserva la resolución seleccionada, recurrentes,
soportes y actos declarados mediante revisiones auditadas. Registrar un recurso,
una interposición oral o un archivo no certifica presentación, admisión,
desistimiento, firmeza ni efectos sobre términos. Las audiencias y los plazos
asociados requieren la extensión descrita en
[el alcance de recursos](procedural-resources-scope.md).

## Autorización y transporte

Todas las rutas requieren `Authorization: Bearer <token>`, autenticación vigente
y autorización del expediente. Owner gestiona; Litigator asignado gestiona;
Paralegal asignado consulta; Client no accede. El cierre administrativo conserva
consultas y bloquea mutaciones. Cada operación vuelve a comprobar su contexto
antes de confirmar y conserva auditoría junto a la escritura.

El adaptador comparte los límites de admisión y trabajo bloqueante del resto de
la API. Respuestas y errores usan `Cache-Control: no-store`. Los cuerpos JSON
completos tienen un límite de 512 KiB. Se rechazan campos desconocidos,
duplicados, estructuras posicionales, contenido posterior al objeto y consultas
ajenas a la ruta. Identificadores de comandos y rutas son UUID canónicos en
minúsculas; el digest de envío es SHA-256 hexadecimal en minúsculas.

## Rutas

Base: `/api/v1/cases/{case_id}/procedural-resources`.

| Método y sufijo | Operación |
| --- | --- |
| `GET` colección | Lista autorizada por identidad ascendente. |
| `GET /{id}` | Cabeza del recurso. |
| `GET /{id}/revisions/{revision}` | Revisión histórica exacta positiva. |
| `GET /{id}/history` | Historia descendente con contenido y capturas. |
| `POST /prepare` | Prepara cualquiera de las seis intenciones. |
| `POST` colección | Confirma `register`. |
| `PUT /{id}` | Confirma `correct`. |
| `POST /{id}/acts` | Confirma `record_act`. |
| `PUT /{id}/acts/{act_id}` | Confirma `correct_act`. |
| `POST /{id}/archive` | Confirma `archive` organizativo. |
| `POST /{id}/reactivation` | Confirma `reactivate`. |

La colección acepta `limit=1..100` (20 por omisión), `after_id`,
`kind=all|revocation|appeal` y `status=all|active|archived`; filtros omitidos
equivalen a `all`. Devuelve `{resources: [Detail], has_more, next_after_id}`.
Historia acepta `limit=1..20` (10 por omisión) y `before_revision` positiva;
devuelve `{revisions: [Detail], has_more, next_before_revision}`. El cursor es
exclusivo. Sin más resultados, el cursor de continuación es `null`.
Las lecturas exactas y mutaciones no aceptan parámetros de consulta.

## Comandos e intenciones

Preparación recibe el comando completo. Las mutaciones reciben
`{command, expected_submission_digest}`. Preparación responde 200; todas las
confirmaciones responden 201 con `Detail`, incluida la reconciliación de una
operación idéntica ya confirmada. Ruta, recurso, acto e intención deben coincidir.

```json
{
 "operation_id": "00000000-0000-0000-0000-000000000003",
 "resource_id": "00000000-0000-0000-0000-000000000002",
 "change": {
   "action": "register",
   "expected_revision": 0,
   "values": {}
 }
}
```

`values` en el esquema abreviado anterior se sustituye por el objeto completo
descrito abajo. Los cambios permitidos son:

| `action` | Campos de `change` además de `action` |
| --- | --- |
| `register` | `expected_revision: 0`, `values` del recurso. |
| `correct` | `expected_revision` positiva, `values`, `reason`. |
| `record_act` | `expected_revision` positiva, `act_id`, `values` del acto. |
| `correct_act` | `expected_revision` y `expected_act_revision` positivas, `act_id`, `values` del acto, `reason`. |
| `archive`, `reactivate` | `expected_revision` positiva, `reason`. |

Cada cambio crea la siguiente revisión del recurso. Un acto nuevo empieza en
revisión 1; corregirlo crea su siguiente revisión y conserva la referencia a
la revisión del recurso que contenía su captura anterior. Archivo y reactivación
preservan valores, resolución y soportes; no son actos procesales.

## Valores explícitos

Una declaración es `{kind:"known",value:...}` o
`{kind:"unknown",reason:"..."}`. No hay modalidad ni autoridad asumida.
Los campos anulables se envían expresamente como `null` o como objeto válido;
ausencia y desconocimiento son estados distintos.

El objeto del recurso contiene exactamente:

- `kind`: `revocation|appeal`.
- `mode`: declaración de `oral|written`.
- `title`: etiqueta organizativa.
- `resolution`: `{id, revision}`, referencia histórica exacta a una resolución
 del mismo expediente, incluso si su captura histórica fue retirada.
- `resolution_evidence`: soporte `{document_id, version, digest, locator}`.
- `resolution_reference`, `issuing_authority`: declaraciones de etiqueta.
- `receiving_authority`: declaración de etiqueta o `null`.
- `resolution_at`: tiempo declarado; `notification_at`: tiempo declarado o `null`.
- `challenged_part`, `grounds`: textos declarados.
- `appellants`: entre 1 y 32 capturas `{name, role, participant}`; `role` es una
 declaración de etiqueta y `participant` es `null` o `{id,revision}` del
 directorio. Una misma ficha no se repite; personas sin ficha pueden compartir
 nombre. No se convierten en cuentas de acceso.

El acto contiene exactamente `kind`, `mode`, `occurred_at`, `authority`,
`statement` y `evidence`. Sus tipos son
`interposition|admission|inadmissibility|withdrawal|resolution`;
modalidad y autoridad usan declaraciones. `evidence` contiene entre 1 y 2
soportes exactos. El límite es técnico; no es una regla sobre suficiencia legal.
Dos localizadores de la misma versión conservan su significado y comparten
admisión; digests contradictorios para esa versión se rechazan.

Etiquetas admiten hasta 200 caracteres y textos hasta 1000; se aplican las
reglas del dominio de hechos. Los tiempos reutilizan su contrato:

- `{precision:"unknown"}`.
- `{precision:"date",year,month,day,offset_seconds}`.
- `{precision:"minute",year,month,day,hour,minute,offset_seconds}`.
- `{precision:"second",year,month,day,hour,minute,second,offset_seconds}`.

El desfase puede ser desconocido (`null`); nunca se inventa medianoche ni una
hora de notificación. La ausencia de hora no se convierte en un instante.
La admisión usa los formatos documentales existentes PDF/DOCX y sus límites:
una constancia de acto oral no exige fingir que el acto fue escrito. Otros
formatos no se habilitan por declarar modalidad oral.

## Preparación y evidencia histórica

`Draft` contiene `case_id`, `command`, `result_revision`, `values`, `status`,
`sources`, `act`, `previous`, `recorded_by`, `observed_administration`,
`observed_stage` y `submission_digest`. `recorded_by` es `{id,email}` capturado
del principal autenticado. La confirmación vuelve a verificar el mismo comando
y el digest revisado; no admite sustituir fuentes por sus cabezas actuales.

`Detail` contiene `case_id`, `id`, `revision`, `values`, `status`, `sources`,
`act`, `reason`, `receipt`, `recorded_by`, `recorded_at`,
`recorded_administration` y `recorded_stage`. `recorded_at` usa RFC 3339 UTC
con la precisión conservada. `receipt` contiene `operation_id`, `action`,
`expected_revision`, `previous`, `values_digest`, `sources_digest`,
`submission_digest` y `capture_digest`. `previous` es `null` para R1 o
`{revision,capture_digest}` de la revisión inmediatamente anterior.

`sources` contiene:

- `resolution`: proyección legible exacta de hechos: `case_id`, `id`, `revision`,
 `values_digest`, `submission_digest`, `status`, `class`, `issuer`, `issued_at`,
 `summary`.
- `appellants`: capturas ordenadas del directorio con `case_id`, `id`, `revision`,
 `values_digest`, `directory_status`, `subject`, `display_name`,
 `procedural_role`, `organization` y `kind`, reutilizando hechos.
- `supports`: `{document_id,version,digest,name,format,policy}` para cada versión
 admitida directamente por el recurso.

`act` es `null` salvo que esa revisión registre o corrija un acto. Entonces
contiene `{id,revision,values,supports,previous}`. Este `previous` identifica
la captura anterior del mismo acto; puede estar varias revisiones atrás.
La cabeza del recurso no contiene una lista implícita de todos los actos:
para reconstruirla se recorre la historia conservando la revisión más alta
de cada identidad de acto.

Administración reutiliza la proyección histórica de hechos (`unrevised` sin
procedencia inventada o `recorded` con revisión, digest, autor y fecha).
Etapa reutiliza `{case_id,current}`, donde `current` es `null` o la entrada
inicial/cambio del contrato de etapas. Es contexto observado, no una transición
causada por el recurso. Los hashes se verifican en aplicación; proyectar un
valor suministrado no acredita integridad por sí solo.

## Errores y alcance de verificación

403 conserva denegación; 401 indica sesión inválida; 404 incluye
`procedural_resource_not_found`. Conflictos 409:
`procedural_resource_revision_conflict`, `procedural_resource_operation_conflict`,
`procedural_resource_archived`, `procedural_resource_state_unchanged` y
`procedural_resource_submission_mismatch`, además de `case_closed` y los
conflictos documentales existentes. JSON/ruta/query inválidos responden 400;
valores del dominio o evidencias rechazadas, 422; cuerpo excesivo, 413.
Inconsistencias persistidas responden 500 genérico sin exponer detalles internos.

Las pruebas del adaptador con puertos controlados cubren las rutas, capturas,
límites y rechazos; su existencia no acredita PostgreSQL, restauración ni
aceptación con navegador y servidor reales. Los resultados ejecutados y sus
límites se registran por separado en [el informe de verificación](verification-report.md).
Véanse [el contrato HTTP general](http-api.md),
[hechos procesales](procedural-facts-api.md), [etapas](case-stages-api.md)
y [la decisión del modelo](adr/0040-resources-linked-to-historical-resolutions.md).
