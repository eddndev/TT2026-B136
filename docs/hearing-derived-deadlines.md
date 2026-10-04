# Resultado de audiencia y plazo derivado

## Estado y alcance

Contrato HTTP de la creación conjunta de un resultado ordinario de audiencia y
un plazo configurado. Preparación, captura, historia, persistencia atómica,
transporte y editor tienen pruebas focales reproducidas. La aceptación nativa
completa de API, reinicio y respaldo/restauración aprobó; también cinco escenarios
de navegador real con Agenda y alertas del plazo, incluida recuperación explícita
de una respuesta perdida. Los resultados y límites están en el
[informe de verificación](verification-report.md).

[ADR-0070](adr/0070-prospective-hearing-derived-deadlines.md) está **Accepted**.
El cierre remoto y la integración de esta entrega siguen pendientes; la evidencia
local no acredita despliegue ni aprobación de los perfiles jurídicos.

La operación registra un `HearingResult` mediante `record`, revisión inicial R1,
y un plazo mediante `register`, también R1. Requiere una instrucción explícita
del operador y un perfil publicado seleccionado. No identifica consecuencias
jurídicas en el relato ni completa el corpus de reglas jurídicas. No corresponde
a la familia distinta de [audiencias propias de recursos](resource-hearings.md).

El contrato reutiliza los comandos, valores y proyecciones de
[resultados de audiencia](hearing-results-api.md) y [plazos](deadlines-api.md).
Una instrucción crea una consecuencia; no establece que una audiencia sólo pueda
producir un plazo ni impide otras altas ordinarias legítimas.

## Rutas, acceso y cuerpo

Base: `/api/v1/cases/{case_id}/hearings/{hearing_id}/results/derived-deadline`.

| Método y sufijo | Respuesta |
| --- | --- |
| `POST /prepare` | HTTP 200 con `state: "ready"` o `state: "replay"`. |
| `POST /submit` | HTTP 201 con el registro conjunto exacto, también en un reenvío idéntico autorizado. |

Ambas rutas requieren bearer y las capacidades de gestionar resultados y plazos:
Owner o Litigator asignado, con autorización actual sobre el expediente. Paralegal
y Client no gestionan este flujo. Una preparación no reserva identidades ni
autoriza una confirmación posterior; el servicio vuelve a comprobar la sesión y
el adaptador reautoriza bajo el bloqueo común de auditoría.

El cuerpo completo se limita a **1 MiB = 1048576 bytes**, incluido el espacio y
los escapes de JSON. Se exige un único `Content-Type` JSON admitido por el lector
HTTP compartido. Se rechazan claves desconocidas o repetidas, arrays en lugar de
objetos, contenido posterior al objeto y cualquier query, incluso vacía. Los
límites menores de los campos y del soporte de los contratos reutilizados siguen
vigentes; el límite HTTP no amplía la admisión de documentos.

## Comando de preparación

El objeto contiene exactamente `case_id`, `result` y `deadline`. El expediente
debe coincidir con la ruta y con la selección del plazo; `result.hearing_id`
debe coincidir con la audiencia de la ruta.

- `result` es el comando existente de resultado, exclusivamente `record` con
  `expected_revision: 0`. Conserva ancla exacta, continuación opcional, asistentes
  y soporte. No acepta una corrección o retiro como creación conjunta.
- `deadline` es el comando humano existente, exclusivamente `register` con
  `expected_revision: 0` y `tracking` explícito. Perfil, calendario opcional,
  responsable, cantidad y declaraciones conservan sus contratos ordinarios.
- La fuente debe ser conocida, de familia `hearing_result`, con el mismo
  expediente, audiencia, `result_id` y revisión **1** que se van a crear. Un
  `agreement_id` opcional debe pertenecer a los valores propuestos de ese R1.
- Los UUID de ambos comandos se conservan al preparar, confirmar o conciliar.
  La operación del resultado identifica también el origen conjunto; la operación
  del plazo sigue siendo parte del comando revisado. No se generan otras
  identidades durante la recuperación.

Ejemplo de estructura con identidades ilustrativas. El perfil y el responsable
deben existir y estar autorizados; sus UUID no representan datos precargados ni
una regla jurídica aprobada. El ejemplo no promete un vencimiento calculable.

```json
{
  "case_id": "00000000-0000-4000-8000-000000000100",
  "result": {
    "operation_id": "00000000-0000-4000-8000-000000000101",
    "hearing_id": "00000000-0000-4000-8000-000000000102",
    "result_id": "00000000-0000-4000-8000-000000000103",
    "change": {
      "action": "record",
      "expected_revision": 0,
      "anchor_revision": 2,
      "continuation": null,
      "values": {
        "occurrence": "occurred",
        "extent": "concluded",
        "event_time": {
          "precision": "date",
          "date": "2026-09-16",
          "offset": "-06:00"
        },
        "summary": "Resultado declarado por el operador",
        "attendees": [],
        "agreements": [],
        "provenance": {
          "kind": "operator_note",
          "reference": null,
          "support": null
        }
      }
    }
  },
  "deadline": {
    "operation_id": "00000000-0000-4000-8000-000000000104",
    "deadline_id": "00000000-0000-4000-8000-000000000105",
    "change": {
      "action": "register",
      "expected_revision": 0,
      "definition": {
        "title": "Consecuencia configurada del resultado",
        "profile": {
          "id": "00000000-0000-4000-8000-000000000106",
          "revision": 1
        },
        "responsible_id": "00000000-0000-4000-8000-000000000107",
        "input": {
          "selection": {
            "case_id": "00000000-0000-4000-8000-000000000100",
            "source": {
              "kind": "known",
              "value": {
                "family": "hearing_result",
                "hearing_id": "00000000-0000-4000-8000-000000000102",
                "result_id": "00000000-0000-4000-8000-000000000103",
                "revision": 1,
                "agreement_id": null
              }
            },
            "qualification": null
          },
          "calendar": null,
          "ordered_quantity": null,
          "qualification": {
            "statement": "Aplicabilidad pendiente de verificar",
            "locator": "Antecedente seleccionado por el operador",
            "scope_applies": {"kind": "unknown", "reason": "Falta verificar"},
            "unresolved_incident": {"kind": "unknown", "reason": "Falta verificar"},
            "conditions": []
          }
        }
      },
      "tracking": {
        "profile": "fixed",
        "source": "follow",
        "calendar": "undetermined"
      }
    }
  }
}
```

La preparación obtiene en servidor las fuentes existentes autorizadas. No exige
consultar un resultado R1 que todavía no existe. Hora de finalización, inicio de
un período ordenado, cantidad y condiciones no se extraen de `summary` ni de la
mera declaración `concluded`. Si faltan datos del cálculo, conserva los motivos
tipados de bloqueo o el candidato civil. Material ajeno o inconsistente se rechaza;
no se disfraza como un cálculo bloqueado aceptable.

## Respuesta `ready`: revisión prospectiva

La respuesta contiene `state: "ready"`, `command` normalizado, `result`,
`deadline` y `review_digest`.

`result` reutiliza el borrador ordinario: valores, ancla, continuación,
administración observada, asistentes y soporte exactos, con sus digests.
`deadline` contiene `definition`, `tracking`, `responsible`, `profile`,
`profile_head`, `calendar`, `calendar_head` y `result` con la evaluación.
Las capturas seleccionadas y las cabezas observadas son datos separados.

La evaluación conserva `requirement`, `trigger_outcome`, `rule`, `arithmetic`,
`due_at` y `blocks` con la proyección existente de plazos. Un cálculo bloqueado
continúa siendo una respuesta 200 `ready`; `ready` significa instrucción preparada,
no vencimiento operativo ni aprobación jurídica. Un candidato civil no adquiere
una hora por defecto.

No hay resultado persistido, evento de fuente, instante definitivo de registro
ni recibo final de plazo durante esta preparación. `result.submission_digest`
es el compromiso del resultado ordinario; el `review_digest` exterior liga la
instrucción conjunta y todo el material revisado. No son intercambiables.

## Confirmación y compromiso final

El cuerpo de `submit` contiene exactamente `command`, con el objeto preparado
completo, y `expected_review_digest`, con la huella exterior confirmada de 64
caracteres hexadecimales en minúsculas. No se envía `expected_submission_digest` en su lugar.

```text
{
  "command": <objeto completo devuelto por prepare>,
  "expected_review_digest": <review_digest de esa respuesta ready>
}
```

El esquema anterior indica sustituciones; no es un cuerpo JSON literal. Cambiar
comando, perfil, calendario, responsable, declaraciones, evidencia o políticas
revisadas exige una preparación y aprobación nuevas. El digest no concede permiso.

La transacción revalida el material y captura el reloj real. Registra resultado
R1, su evento existente, plazo R1, origen conjunto y auditoría, o revierte todas
esas escrituras. El plazo usa la fuente que acaba de quedar capturada y conserva
el cálculo revisado. No se emite un segundo evento de resultado ni se marca
procesado el evento para simular la ejecución del dispatcher.

La respuesta 201 es el objeto de registro, con estos campos exactos:

| Campo | Contenido |
| --- | --- |
| `case_id` | Expediente de la ruta y de ambos componentes. |
| `command` | Comando conjunto normalizado; conserva las dos operaciones. |
| `result` | Detalle exacto ordinario del resultado R1, con su recibo y registro real. |
| `deadline` | Detalle exacto ordinario del plazo R1, con su recibo y registro real. |
| `origin` | Identidades, autor y compromisos del origen conjunto, descritos abajo. |
| `review_digest` | Aprobación prospectiva HRDL1; coincide con la del origen. |
| `capture_digest` | Captura real HRDC1; coincide con la del origen. |

`origin` contiene exactamente `case_id`, `hearing_id`, `result_id`,
`result_revision`, `result_operation_id`, `deadline_id`, `deadline_revision`,
`deadline_operation_id`, `recorded_by`, `review_digest`, `capture_digest` y
`source_event`. Ambas revisiones son 1. `recorded_by` conserva `{id,email,role}`
del autor original. El comando está en el registro exterior, no duplicado dentro
del origen. No se añade un reloj de captura conjunto: los dos detalles ordinarios
conservan el instante real compartido.

El `capture_digest` conjunto liga la captura real HRDC1, el evento y los recibos
de ambos componentes. No reemplaza ni modifica HRES1/HRTX1 o el recibo ordinario
del plazo. El resultado ordinario no adquiere un `capture_digest` propio.

## Recuperación `replay` e historia

Reenviar expresamente el mismo comando a `prepare` puede devolver HTTP 200
`{"state":"replay","record":...}`. `record` es la creación conjunta original,
no otro borrador. La coincidencia de dos registros creados por separado no basta:
debe existir y verificarse el origen conjunto inmutable y su auditoría.

El registro conserva resultado y plazo R1, comando, autor original, evento exacto,
`review_digest` y `capture_digest`. No carga las cabezas actuales para reemplazar
las capturas ni recalcula la historia. Mantiene correo y rol capturados del autor;
el acceso se decide por su identidad y permisos actuales, antes de divulgarla.

La proyección del evento reutiliza la de plazos:

```json
{
  "sequence": "9007199254740993",
  "family": "hearing_result",
  "source_id": "00000000-0000-4000-8000-000000000103",
  "revision": 1,
  "case_id": "00000000-0000-4000-8000-000000000100",
  "hearing_id": "00000000-0000-4000-8000-000000000102",
  "operation_id": "00000000-0000-4000-8000-000000000101"
}
```

`sequence` es una cadena decimal, no un número JSON sujeto al redondeo de
JavaScript. Conserva el valor real entre 1 y 9223372036854775807; el ejemplo es
ilustrativo y no permite al cliente elegir o fabricar el evento.

`submit` con comando y revisión confirmada idénticos devuelve también 201 y el
mismo registro, sin otra escritura. Ese estado HTTP no distingue una creación
nueva de una conciliación. Un error o desconexión posterior al commit no demuestra
ausencia de escritura. El cliente conserva el intento y recupera explícitamente;
no cambia UUID, repite automáticamente ni adopta un recibo de la cabeza posterior.

El detalle ordinario del plazo conserva `operational.freshness: "not_checked"`
hasta la comprobación operativa correspondiente. Agenda y alertas mantienen sus
controles de permiso, aceptación y dependencias. Guardar una instrucción bloqueada
no la convierte en un vencimiento operativo.

## Cliente de Qadra: contrato interno

`caseApi(...).caseHearingDerivedDeadlines(caseId, hearingId)` expone la
preparación y confirmación compuestas al cliente. Comprueba el expediente,
audiencia, fuente R1, instrucciones, identidad actual y ambas respuestas, sin
consultar un resultado prospectivo que todavía no existe. Una respuesta `replay`
requiere el origen conjunto y los componentes exactos; dos recibos ordinarios
separados no bastan para acreditar esa creación.

El cliente copia la instrucción y la aprobación antes de esperar la respuesta.
Los cambios posteriores al formulario no alteran el intento enviado. Una sesión
con otra identidad o un contexto cerrado no puede aceptar respuestas tardías.
El cierre del expediente se comunica por el observador compartido, sin impedir
por sí mismo la consulta explícita de una captura anterior autorizada.

La comparación de captura conserva segundos, nanosegundos y desfase; el evento
permanece como cadena decimal. Las comprobaciones son de contrato y coherencia:
el navegador no recalcula el plazo ni verifica criptográficamente las huellas.
Los asistentes históricos conservan el estado de directorio `active` o
`archived`; no se sustituyen por las revisiones actuales. El presupuesto de
1 MiB se aplica al JSON enviado. Una revisión con capturas de catálogos mayores
puede confirmarse si su comando y huella caben en ese presupuesto: no se envía
la revisión completa como cuerpo de confirmación.

## Edición y recuperación en Qadra

La vista de resultados de una audiencia ordinaria ofrece **Registrar resultado
y plazo** a quien puede gestionar ambos registros. Mantiene la acción ordinaria
de registrar sólo el resultado. La nueva acción conserva el ancla exacta de la
audiencia y fija la fuente del plazo al resultado R1 propuesto, sin consultarlo
como si ya existiera. Permite seleccionar un acuerdo de ese resultado, el perfil,
responsable, calendario cuando corresponda y políticas de seguimiento.

La revisión muestra ambos registros, el acuerdo elegido, inicio declarado con
finalidad, precisión y desfase, cantidad, aplicabilidad, incidencia y condiciones
con sus localizadores. Separa revisiones seleccionadas y cabezas observadas.
Expone el cálculo prospectivo y sus bloqueos; requiere confirmación explícita
antes de enviar el comando y su huella conjunta. La confirmación identifica
el resultado y plazo R1 originales y su origen, sin adoptar cabezas posteriores.

El borrador conserva declaraciones y entradas parciales bajo la identidad y
expediente actuales. Al caducar la sesión, la reentrada del mismo usuario vuelve
a autorizar el expediente y elimina la aprobación anterior. Una identidad
distinta, denegación del expediente o cierre explícito descarta el borrador
correspondiente. El soporte documental no disponible se retira sin borrar el
resto de las declaraciones; su denegación específica no equivale a perder todo
el expediente.

Si el envío quedó incierto, se preservan el comando, las dos operaciones y la
revisión enviada. La recuperación consulta explícitamente ese intento antes de
cargar catálogos o soportes actuales. Un `replay` coincidente confirma la captura
original. Un `ready` idéntico conserva la incertidumbre y permite un reenvío
expreso si el expediente admite escritura; no demuestra que el envío anterior
haya fallado. Una revisión distinta mantiene el conflicto sin cambiar identidades.
El cierre del expediente bloquea un nuevo envío, pero conserva la conciliación
autorizada del intento anterior. No hay reenvío automático.

La aceptación completa con HTTP nativo, restauración y consumidores operativos
permanece pendiente; las pruebas de interfaz con respuestas controladas no la
sustituyen ni acreditan despliegue.

## Errores y alcance de la verificación

Se reutiliza el sobre `{error:{code,message}}` y los errores de los módulos
existentes; no se devuelve material protegido como diagnóstico de rechazo.

| Estado | Casos |
| --- | --- |
| 400 | JSON o `Content-Type` inválido (`invalid_json`), propiedades o query no admitidas, identificadores mal formados o contrato de comando incorrecto. |
| 401 / 403 | Sesión ausente, inválida o revocada / autoridad insuficiente. |
| 404 | Expediente o fuente no disponible bajo las reglas de acceso existentes. |
| 409 | `deadline_submission_mismatch` para revisión no coincidente; conflictos de operación, revisión, soporte, perfil o responsable conservan sus códigos existentes. |
| 413 | Cuerpo JSON mayor de 1 MiB. |
| 422 | Valores, referencias o tiempos inválidos y rechazo de admisión de soporte, según resultados/plazos. Un bloqueo válido del evaluador no basta para este error. |
| 500 | Evidencia persistida inconsistente o respuesta del puerto ajena al comando/ámbito solicitado; `internal_error`. |

La verificación focal HTTP cubre ambas alternativas de preparación, confirmación
y reenvío, ámbito y fuente R1 exactos, huellas, autorización, cuerpo estricto y
rechazo de proyecciones inconsistentes. También comprueba la conservación del
offset declarado y la secuencia de evento como cadena sin pérdida de precisión.
La prueba de composición verifica ambas rutas dentro de la API protegida y
`Cache-Control: no-store`; utiliza puertos simulados y no acredita persistencia
HTTP real. El [informe de verificación](verification-report.md) separa estas
ejecuciones de las pruebas anteriores de aplicación y PostgreSQL. Permanecen
pendientes la aceptación completa con captura/restauración y los gates
globales de esta entrega.
