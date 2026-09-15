# Etapas procesales por expediente

Contrato del registro procesal persistente y auditado, consumido por Qadra.
Los resultados reproducidos se registran en
[el informe de verificación](verification-report.md).

## Rutas y permisos

Todas las rutas parten de `/api/v1/cases/{case_id}` y requieren bearer vigente.
Owner gestiona cualquier expediente; Litigator asignado consulta y gestiona;
Paralegal asignado consulta. Client no accede a este recurso de personal.

| Método | Ruta | Resultado |
| --- | --- | --- |
| GET | `/stage` | Etapa actual o ausencia explícita. |
| GET | `/stage/history?limit=20&before_revision=3` | Historia descendente por revisión; incluye el registro inicial original. |
| POST | `/stage/adoption` | Registra la etapa conocida de un expediente sin etapa. |
| POST | `/stage/transitions` | Registra un avance ordinario con sus soportes exactos. |

GET y POST devuelven `{ "case_id": "uuid", "current": null }` o un objeto
`current` con una entrada. La ausencia no tiene una revisión cero ficticia.
Las consultas responden 200; ambas mutaciones confirmadas responden 201.
La historia devuelve `{ "entries": [], "has_more": false,
"next_before_revision": null }`; el límite admite 1 a 100 y el cursor es
positivo y exclusivo. No se ordena por la fecha declarada de los actos.

## Fechas declaradas

La precisión y el desfase se conservan. Una fecha sin hora no se convierte en
un acto ocurrido a medianoche. El desfase es explícito, en minutos y entre
`-14:00` y `+14:00`; `-00:00` no representa un desfase conocido.

```json
{ "precision": "date", "date": "2026-09-01", "offset": "-06:00" }
```

```json
{ "precision": "instant", "at": "2026-09-01T10:30:00-06:00" }
```

El tiempo de registro lo obtiene el servidor después de preparar los soportes.
Se rechazan declaraciones enteramente futuras. La fecha sola se comprueba
como intervalo del día declarado en su desfase. Emisión y recepción se rechazan
por orden únicamente cuando todo el intervalo de emisión es posterior al de
recepción; el sistema no inventa un orden dentro de un día sin hora conocida.

## Solicitudes

Un soporte es `{ "document_id": "uuid", "version": 1, "digest": "hex64" }`.
El digest corresponde al contenido de la versión seleccionada. La búsqueda,
historia y detalle documental existentes permiten construir esa selección.

Adopción: `expected_revision` debe ser cero; `stage` admite `investigation`,
`intermediate` y `trial`. Se exige motivo de 1 a 1000 caracteres y soporte.

```json
{
  "expected_revision": 0,
  "stage": "intermediate",
  "known_at": { "precision": "date", "date": "2026-09-01", "offset": "-06:00" },
  "reason": "El despacho incorpora el asunto en su etapa conocida.",
  "support": { "document_id": "uuid", "version": 1, "digest": "hex64" }
}
```

Transición a Intermedia: `expected_revision` positivo, `target: "intermediate"`,
`accusation_declared_at`, `accusation` y `note` opcional.

Transición a Juicio: `expected_revision` positivo, `target: "trial"`,
`opening_order_issued_at`, `opening_order`, `received_at`, `receiving_court`,
`receipt_reference` opcional, `receipt_support` opcional y `note` opcional.
Tribunal y referencia admiten 1 a 200 caracteres, en una línea. Una misma
versión puede cumplir ambos papeles si se declara explícitamente con igual
digest. El trabajo criptográfico y de formato se deduplica por referencia.

Cada solicitud admite únicamente los campos de su variante. El cuerpo completo
se limita a 32 KiB. No se reciben autores, fecha de captura ni evidencia binaria.

## Entradas históricas

Todas las entradas incluyen `kind`, `case_id`, `stage_revision`, `stage`,
`administration_revision`, `administration_digest`, `recorded_at` y
`recorded_by: { "id": "uuid", "email": "correo" }`.

- `kind: "initial"` conserva íntegramente el registro inicial de Investigación
  y su administración original R1. No añade valores o soportes retrospectivos.
- `kind: "change"` añade `from_stage` (null en adopción), `values_digest`,
  `values` y `supports`.
- `values` usa `kind: "adoption"`, `"to_intermediate"` o `"to_trial"` y
  los campos de su solicitud, sin `expected_revision` ni `target`.
- `supports` contiene referencias únicas con `document_id`, `version`,
  `digest`, `name`, `format` (`pdf` o `docx`) y `policy` (`pdf_docx_v1`).

El registro inicial y una adopción son alternativas para R1. Las transiciones
permitidas en este recurso son Investigación a Intermedia e Intermedia a Juicio.
El alcance de recursos se concilia como función adicional; estas dos aristas
no declaran cumplido todo el objetivo de gestión procesal.

## Confirmación y errores

La mutación exige perfil penal completo y estado administrativo activo al
confirmar. Revisa autorización, asociación exacta del soporte, revisión de
etapa y contenido preparado nuevamente dentro de la transacción auditada.
La revisión administrativa y la de etapa son independientes. Un cierre o una
revocación durante la preparación no pueden producir una mutación exitosa.

Un archivo sellado durante la preparación exige repetir explícitamente la
validación de ese soporte. Sellarlo después de confirmar la etapa conserva la
historia registrada. Una versión posterior tampoco modifica el vínculo.

Los errores de soporte distinguen tamaño excesivo, formato rechazado y límite
de validación (422). Corrupción almacenada produce un error interno opaco.
Los conflictos de revisión o soporte se comunican como 409, conservando el
borrador. Recurso ausente o ajeno produce la misma respuesta 404.

| Estado | Códigos propios del recurso |
| --- | --- |
| 400 | `invalid_stage_support_digest` para una huella que no es SHA-256 hexadecimal; JSON, UUID y variantes mal formados siguen los errores generales. |
| 409 | `case_stage_conflict`, `case_stage_revision_exhausted`, `case_stage_required`, `case_stage_transition_rejected`, `case_stage_profile_incomplete`, `stage_support_digest_mismatch`, `stage_support_changed`. |
| 422 | `stage_support_too_large`, `stage_support_format_rejected`, `stage_support_validation_limit`, `invalid_case_stage`, `invalid_declared_stage_time`, `invalid_stage_note`, `invalid_stage_court`, `invalid_stage_receipt_reference`, `invalid_stage_act_order`, `conflicting_stage_support`, `stage_act_in_future`. |

También aplican `case_closed`, permisos, sesión inválida, cuerpo excesivo y
sobrecarga del [contrato general](http-api.md). Un inventario procesal corrupto
o fallo de configuración devuelve `500 internal_error`, sin detalles internos.

Cargar un documento y registrar una etapa son transacciones distintas. Un
rechazo de etapa no elimina una carga confirmada. Ante resultado de red incierto,
la interfaz consulta etapa e historia antes de permitir repetir el comando;
una coincidencia por sí sola no demuestra que se haya confirmado ese envío.
