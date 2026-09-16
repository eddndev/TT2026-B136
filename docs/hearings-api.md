# API de programación de audiencias

Estado: implementado. La evidencia ejecutada se distingue de los pendientes en
[el informe de verificación](verification-report.md).
Rutas bajo `/api/v1`. Autenticación bearer y reglas transaccionales de
[0028](adr/0028-audited-hearing-scheduling.md). Este módulo registra citas y sus
revisiones. Una fecha pasada no demuestra celebración ni resultado.

## Autorización y contexto

Owner consulta y gestiona; Litigator asignado consulta y gestiona; Paralegal
asignado consulta. Client no tiene acceso. Un expediente cerrado conserva sus
consultas e historia, pero rechaza mutaciones. La asignación a una cuenta no la
convierte en participante procesal.

Alta y reemplazo requieren perfil penal completo, expediente activo y etapa
registrada. El comando declara las revisiones administrativa y procesal
esperadas; ambas se vuelven a comprobar al confirmar. Cancelar requiere acceso,
expediente activo y revisión de audiencia esperada. No exige que la etapa actual
siga correspondiendo a la programación anterior.

| Método | Ruta | Resultado |
| --- | --- | --- |
| GET | `/cases/{case_id}/hearings/context` | Contexto consistente |
| POST | `/cases/{case_id}/hearings/prepare` | Propuesta normalizada y digest |
| GET | `/cases/{case_id}/hearings` | Cabeceras actuales autorizadas |
| POST | `/cases/{case_id}/hearings` | Alta, HTTP 201 |
| GET | `/cases/{case_id}/hearings/{id}` | Revisión actual |
| PUT | `/cases/{case_id}/hearings/{id}` | Nueva revisión, HTTP 201 |
| POST | `/cases/{case_id}/hearings/{id}/cancellation` | Cancelación, HTTP 201 |
| GET | `/cases/{case_id}/hearings/{id}/revisions/{revision}` | Revisión exacta |
| GET | `/cases/{case_id}/hearings/{id}/history` | Historia descendente |
| GET | `/hearings` | Agenda transversal autorizada |

Los cuerpos JSON tienen un límite de 64 KiB. Se rechazan campos desconocidos o
duplicados, contenido después del objeto y cabeceras Content-Type duplicadas.
UUID, revisiones, valores y fechas se validan antes del acceso a persistencia.

## Comando y valores

`prepare` recibe este objeto, sin el expediente ni la identidad del actor en el
cuerpo. Ambos proceden de la ruta y de la sesión, respectivamente.

```json
{
  "operation_id": "00000000-0000-4000-8000-000000000011",
  "hearing_id": "00000000-0000-4000-8000-000000000012",
  "change": {
    "action": "schedule",
    "expected_revision": 0,
    "expected_case_revision": 1,
    "expected_stage_revision": 1,
    "values": {
      "kind": "initial",
      "scheduled_at": "2026-10-01T09:00:00-06:00",
      "modality": "in_person",
      "venue": "Sala declarada por el operador",
      "note": null,
      "participants": [],
      "conviction_basis": null
    }
  }
}
```

`replace` exige `expected_revision` positivo y los mismos campos de contexto y
valores, más `reason` obligatorio. `cancel` contiene únicamente `action`,
`expected_revision` positivo y `reason`; el servidor deriva los valores y el
contexto de la revisión exacta anterior. El UUID de operación siempre es nuevo.

Tipos: `initial`, `intermediate`, `oral_trial`, `sentencing`. El tipo de una raíz
no cambia. Los dos primeros corresponden respectivamente a Investigación e
Intermedia; debate e individualización corresponden a Juicio. Esta clasificación
ordinaria no certifica procedencia judicial. Individualización exige además
antecedente declarado de condena y soporte exacto; no se infiere de la etapa.

Modalidades: `in_person`, `videoconference`. Estados organizativos: `scheduled`
y `cancelled`. Cancelar no elimina historia, produce resultados ni declara
nulidad procesal. No existe reactivación automática ni restricción de una única
audiencia por tipo y expediente.

`scheduled_at` es RFC 3339, con segundos completos y desfase explícito de minutos
entre -14:00 y +14:00, incluidos los extremos. Usa `T` y `Z` mayúsculas. Admite `Z`, rechaza `-00:00`,
fracciones, segundos intercalares y años locales o UTC fuera de 1..9999. Conserva
el desfase comunicado, aunque otro desfase describa el mismo instante. Pasado y
futuro son válidos; el reloj no cambia el estado de una audiencia.

`venue` exige 1..500 caracteres Unicode sin controles. `note` opcional y `reason`
exigen como máximo 1000 caracteres; motivo no vacío. Se recorta espacio exterior,
CRLF se convierte a LF y solamente se admite LF entre los caracteres de control.
No hay normalización Unicode interior. Sede y conexión se muestran como texto.

`participants` contiene 0..32 objetos `{ "participant_id": "uuid", "revision": 1 }`.
Se ordena por UUID y no permite repetir una ficha, ni siquiera con otra revisión.
Las fichas nuevas deben corresponder a su revisión vigente y activa del mismo
expediente. Al reemplazar se pueden conservar las referencias exactas ya
vinculadas, aunque la ficha se haya editado o archivado después. No se sustituyen
revisiones ni se deduce asistencia. Lista vacía significa selección no registrada.

Para `sentencing`, `conviction_basis` es obligatorio:

```json
{
  "statement": "Antecedente de condena declarado por el operador",
  "support": {
    "document_id": "00000000-0000-4000-8000-000000000013",
    "version": 1,
    "digest": "0000000000000000000000000000000000000000000000000000000000000000"
  }
}
```

`statement` usa las reglas de `reason`, 1..1000 caracteres. Los otros tipos
rechazan este antecedente. Se valida soporte PDF/DOCX cifrado de versión exacta,
SHA-256 y evidencia capturada bajo la política existente de admisión de soportes,
con máximo 16 MiB. El ejemplo de digest es ilustrativo, no un archivo admitido.
Cancelar conserva el soporte histórico y no lo presenta como nueva validación.

## Preparación y respuestas inciertas

La respuesta de preparación incluye `case_id`, `actor_id`, `command` normalizado,
`result_revision`, `values`, `values_digest` y `submission_digest`. No reserva
UUID ni crea registros. Para escribir, el cliente envía:

```json
{
  "command": {},
  "expected_submission_digest": "sha256-hex-de-la-preparacion"
}
```

`command` debe ser el objeto completo devuelto al preparar. Método, ruta, acción
e identificador deben coincidir. El servidor repite las validaciones, calcula el
digest otra vez y comprueba los snapshots tras obtener el bloqueo transaccional.
La preparación anterior no mantiene permisos ni congela el estado del expediente.

Cada revisión conserva su propio recibo: `operation_id`, `action`,
`expected_revision`, `expected_context` (revisiones `case_revision` y
`stage_revision`, o null al cancelar), y `submission_digest`. El autor queda en
`recorded_by`. Reutilizar un UUID de operación siempre causa conflicto; la
respuesta no revela qué recurso lo utilizó. No existe reintento automático.

Después de perder una respuesta, consultar la revisión exacta y comprobar
expediente, audiencia, revisión, actor, operación, acción y digest esperado.
Valores parecidos no prueban el envío. Un 404 durante una operación en vuelo
mantiene el resultado incierto. Si la revisión pertenece a otro envío, conservar
el borrador y mostrar conflicto. Qadra no vuelve a enviar desde ese estado.

## Proyecciones

Contexto: `case_id`, `case_revision` (0 para raíz anterior sin revisiones),
`case_values_digest` (null si falta), `title`, `reference`, `administrative_status`,
`profile_complete`, `stage_revision`, `stage`, `stage_values_digest`. Los tres
últimos son null sin etapa; el digest es también null para un registro inicial,
que no tiene un digest separado de valores procesales.

Detalle: `case_id`, `id`, `revision`, `values`, `values_digest`, `status`, `reason`,
`receipt`, `scheduling_context`, `recorded_administration_revision`,
`recorded_administration_digest`, `recorded_at`, `recorded_by`, `participants`,
`support`. `recorded_by` contiene `id` y `email` capturados.

`scheduling_context` contiene `administration_revision`, `administration_digest`,
`stage_revision`, `stage`, `stage_digest`. Este último es null para el registro
inicial y el digest original de valores para una adopción/transición. La lectura
resuelve y valida la fuente histórica exacta; el digest administrativo vigente
no se inventa como hash del registro inicial. Cancelación conserva este contexto,
pero registra la revisión administrativa actual al realizar su escritura.

Cada participante de detalle incluye `id`, `revision`, `profile` (`manual` o
`typed`), `display_name`, `procedural_role`, `kind` opcional, `subject` opcional
(`id`, `revision`, `values_digest`), `values_digest` de ficha y `directory_status`
histórico. El nombre procede de la identidad exacta vinculada. `support`, cuando
existe, contiene `document_id`, `version`, `digest`, `name`, `format`, `policy`.

Los listados contienen `hearings`, `has_more` y cursor siguiente. Cada fila
incluye `case_id`, `case_title`, `case_reference`, `case_status`, `id`, `revision`,
`kind`, `scheduled_at`, `modality`, `status`, `participant_count`. No incluyen
notas, conexiones ni identidad de participantes. Título y estado administrativo
son actuales; los datos de la audiencia corresponden a su cabecera seleccionada.

## Consultas

Listado por expediente: `status=scheduled|cancelled|all` (all por defecto),
`limit=1..100` (20 por defecto), `after_id` opcional exclusivo, UUID ascendente.
Devuelve `next_after_id`. No limita fechas implícitamente.

Historia: `limit=1..100` (20), `before_revision` positivo opcional y exclusivo.
Devuelve `revisions`, `has_more`, `next_before_revision`.

Agenda: `from` y `until` UTC explícitos (`Z` o `+00:00`), intervalo positivo
`[from,until)` de máximo 366 días. `status=scheduled|cancelled|all` (scheduled),
`limit=1..100` (20), `after_time` y `after_id` juntos o ausentes. El instante del
cursor debe pertenecer al intervalo. Orden ascendente por UTC y UUID; devuelve
`next_after` null u objeto `{ "at": "RFC3339-UTC", "id": "uuid" }`.
Permisos, cabecera actual y filtros se aplican antes de paginar. El conjunto puede
cambiar entre páginas; no se declara un snapshot global. No se arma la agenda
recorriendo todos los expedientes desde el navegador.

## Codificación y errores

`HEAR1` codifica valores normalizados, no actor ni fecha de captura. Máximo
10726 bytes: prefijo, tipo u8, Unix i64 BE, desfase segundos i32 BE, modalidad u8,
sede, nota opcional, conteo u8 y pares UUID/revisión u32 ordenados, antecedente
opcional (texto, UUID documental, versión u32 y digest32). Texto: longitud UTF-8
u32 BE y bytes. Opcional: etiqueta 0/1. Todos los enteros usan big endian.

`HTXN1`: prefijo, UUID de operación/actor/expediente/audiencia, acción u8
(schedule0/replace1/cancel2), revisión esperada u32 (0 sólo alta), contexto
opcional (dos revisiones u32), digest HEAR1 de32 bytes, motivo opcional.
Resultado = expected+1; nunca wrap. El servidor no firma estos recibos.

Errores 409: `hearing_revision_conflict`, `hearing_context_conflict`,
`hearing_participant_changed`, `hearing_already_cancelled`,
`hearing_operation_conflict`, `hearing_submission_mismatch`,
`hearing_support_changed`. Errores 422: `hearing_revision_exhausted`,
`hearing_context_required`, `hearing_stage_incompatible`, `hearing_immutable_kind`,
`invalid_hearing_value`, `invalid_hearing_revision`, `hearing_support_too_large`, `hearing_support_format_rejected`,
`hearing_support_validation_limit` y `hearing_support_digest_mismatch`.
`hearing_not_found` usa404; inconsistencias almacenadas usan500 sin datos internos.
Errores sintácticos/query usan400; cuerpo excesivo413. Sesión401, permiso403 y
`case_not_found`404 mantienen el contrato general. Expediente cerrado usa409.
