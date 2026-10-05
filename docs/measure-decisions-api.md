# API de decisiones cautelares declaradas

## Estado y acceso

Transporte implementado en la rama cautelar y comprobado con puertos controlados.
Qadra permite registrar los efectos agrupados y el resultado sin cambios,
consultar el recibo original y recuperar borradores/envios inciertos con HTTP
controlado. La aceptación completa con servicios reales y restauración permanece
pendiente. El [cierre finito](four-front-closure.md) conserva esas distinciones.
Este contrato expone decisiones declaradas; no establece procedencia judicial.

Base: `/api/v1/cases/{case_id}/measure-decisions`. Bearer obligatorio y respuestas
con `Cache-Control: no-store`. Owner y Litigante asignado preparan/confirman;
Paralegal asignado puede consultar; Client queda denegado. La aplicación verifica
el acceso vigente y las escrituras requieren expediente activo. Las lecturas y
recuperación originales conservan autorización actual incluso con expediente cerrado.

| Método y sufijo | Resultado |
| --- | --- |
| POST `/prepare` | 200 con revisión normalizada, familia y ambas confirmaciones. |
| POST `/submit` | 201 con grupo original completo, también en replay autorizado. |
| GET `/` | Página ordenada por identidad de decisión. |
| GET `/{decision_id}` | Grupo original de esa decisión. |
| GET `/operations/{operation_id}` | Grupo original de la operación exacta. |

Las rutas se componen con el mismo presupuesto de solicitudes y trabajo del
resto del servidor; no crean un ejecutor independiente.

## Comando y confirmación

Preparar recibe exactamente `case_id`, `operation_id`, `decision_id`, `context`,
`values`, `anchor` y `outcome`. El expediente coincide con la ruta. Contexto es
`{administration_revision, stage_revision, context_digest}` de la
[consulta cautelar](precautionary-hearings-api.md).

`values` contiene `authority`, `declared_at`, `justification`, `support` y
`locator`. Soporte es `{document_id, version, digest}`. La fecha declarada usa
las precisiones y campos de [registros de medidas](measure-records-api.md):
desconocida con motivo, fecha, minuto o segundo, sin inventar precisión/desfase.

`anchor` debe estar presente; null conserva una decisión independiente. Un objeto
selecciona exactamente una de estas fuentes:

- `kind: initial`: `hearing_id`, `revision`, `values_digest`, `submission_digest`.
- `kind: precautionary`: `hearing_id`, `revision`, `capture_digest`.

Una ancla inicial reutiliza esa audiencia; no registra una convocatoria duplicada.

`outcome` es `no_measure_change` con `statement`, o `changes` con `effects`.
Los efectos forman un grupo indivisible con las acciones existentes:

| Acción | Campos |
| --- | --- |
| `impose` | `proposal` con `id` y `values`. |
| `confirm`, `revoke`, `cease` | `previous` exacto. |
| `modify` | `previous` y `values`. |
| `substitute` | `predecessors` y `successors`, conservando los enlaces del grupo. |

Una referencia previa contiene `{id, revision, capture_digest}`. Los valores de
medida contienen `subject`, `kind`, `conditions`, `validity` y `supervision`;
sujeto es `{id, revision, values_digest}`. Las catorce clases, validez y distinción
entre efecto judicial y rectificación conservan el
[contrato de alcance](precautionary-hearings-scope.md). Supervisión es una
referencia de participante/revisión con declaración, o desconocida con motivo.
Los cambios no extraen obligaciones, duraciones ni efectos del texto libre.

Confirmar recibe `{command, expected_submission_digest, expected_review_digest}`.
Preparar normaliza texto y orden; el cliente conserva ese comando y ambos digests.
El servidor revalida fuentes, contexto, autorización y revisiones al confirmar.
No recibe autor, reloj de captura ni historial suministrados por el cliente.

## Lecturas y respuesta perdida

La preparación devuelve `{family, review}`. El recibo conserva `{family, group,
origin}` y la historia de su familia original: `measure_history` para G1 o
`record_history` para G2. Una respuesta G1 no se reescribe como G2. Grupo e historia
retienen decisiones, medidas, anclas, sujetos, participantes y soportes capturados.

Una respuesta incierta se recupera por la misma operación. Reenviar expresamente
el mismo comando con las mismas confirmaciones recupera el grupo original;
utilizar esa operación para otro comando produce conflicto. Un registro parecido
no demuestra éxito de la operación y no autoriza generar otra identidad.

Lista admite `limit` de 1 a 20, por defecto 10, y `after_id` UUID exclusivo.
Devuelve `{case_id, items, has_more, next_after_id}`. Las otras rutas no admiten
query. Identidades, orden, continuación y compromisos de la respuesta se comprueban
antes de devolver los grupos; no se truncan historias para hacerlas caber.

## Límites y errores

Cuerpo JSON máximo de 4 MiB, incluidos escapes y espacios. Se conservan los
límites existentes de campos y del grupo de hasta 32 identidades, contando
anteriores y nuevas. Claves desconocidas/repetidas, arrays en lugar de objetos,
contenido posterior, UUID no canónicos y digests distintos de 64 caracteres
hexadecimales minúsculos se rechazan. El límite de transporte no amplía el dominio.

Autenticación inválida devuelve 401; autorización denegada, 403; ausencia,
404; operación o confirmaciones conflictivas, 409; historia incompleta, 422.
Integridad persistida inconsistente devuelve el mensaje interno opaco con 500.
Estas respuestas no sustituyen la prueba de atomicidad y restauración del flujo
completo. Las [rectificaciones administrativas](measure-records-api.md) conservan
sus rutas y significado separados.

## Recorrido de Qadra

La sección **Medidas cautelares** del expediente conserva decisiones y registros
de medidas. Owner y Litigator pueden preparar los seis tipos de efectos
existentes, incluidos grupos de varios sujetos y sustituciones múltiples, o
declarar expresamente que no hubo cambios. La modificación conserva sujeto y
clase; una selección explícita de otra base reemplaza sus valores visibles.
Paralegal y expedientes cerrados conservan lecturas.

Los selectores conservan soporte, sujeto, supervisor y audiencia por revisión
exacta. El vínculo inicial reutiliza la audiencia; una convocatoria cautelar
puede seleccionarse por una captura histórica. La revisión presenta lo declarado,
sus fuentes y ambas confirmaciones antes del envío. No determina vigencia
jurídica por el reloj ni impone consecuencias a partir de texto libre.

Navegar conserva el borrador, incluso tiempos incompletos. Recuperarlo exige
la misma identidad completa y autorización actual; retira la aprobación anterior.
Un envío incierto conserva comando, operación y confirmaciones, y solo se
reintenta expresamente después de consultar el resultado. La recuperación puede
devolver una captura anterior a la cabeza actual sin sustituirla silenciosamente.
Las rectificaciones administrativas y la aceptación real completa siguen su
propio criterio de cierre en [la lista finita](four-front-closure.md).
