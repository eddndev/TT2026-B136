# Administración del expediente penal por HTTP

Este contrato amplía [la API autenticada](http-api.md) con el perfil penal,
revisiones administrativas y cierre organizativo. La decisión y sus límites
están en [ADR-0022](adr/0022-audited-penal-case-administration.md).

## Rutas y autorización

| Método y ruta bajo `/api/v1` | Resultado |
| --- | --- |
| `POST /penal-cases` | Alta completa y detalle confirmado, `201`. |
| `GET /case-administrations` | Índice del personal del despacho, `200`. |
| `GET /cases/{id}/administration` | Detalle actual, `200`. |
| `PUT /cases/{id}/administration` | Edición con revisión esperada y detalle confirmado, `200`. |
| `GET /cases/{id}/administration/history` | Historia administrativa descendente, `200`. |
| `PUT /cases/{id}/administrative-status` | Cambio exclusivo de estado y detalle confirmado, `200`. |

Todas requieren Bearer. Owner y Litigator pueden crear un expediente penal sin
asignación previa; el alta asigna a su creador. Para expedientes existentes,
Owner consulta y administra cualquiera; Litigator asignado consulta y administra;
Paralegal asignado consulta. Client
recibe `403 permission_denied` en las seis operaciones, incluso si está asignado.
Conserva únicamente la API básica de expedientes. El actor activo, rol y alcance
se revalidan dentro de la transacción. Las consultas confirman un evento antes
de devolver datos; los comandos devuelven su propia instantánea confirmada.

## Alta y edición

El alta recibe:

```json
{
  "title": "Defensa inicial",
  "reference": "INTERNA-001",
  "profile": {
    "nuc": "NUC-001",
    "nuc_authority": "Fiscalia registrada",
    "judicial_case_number": "CJ-001",
    "judicial_authority": "Organo registrado",
    "offenses": ["Descripcion del delito"],
    "general_information": "Primera linea\nSegunda linea",
    "complementary_identifiers": null
  }
}
```

Confirma raíz, asignación del creador, revisión administrativa 1 activa y registro
inicial de Investigación, junto con sus cuatro eventos, en una transacción.
El registro inicial refiere exactamente a esa primera revisión y no acredita
la fecha de un acto judicial. No hay transiciones ni adopción de etapa por HTTP.

La edición recibe los mismos campos y `expected_revision` entero entre 0 y
4294967295. `profile` puede faltar o ser `null` solo mientras la ficha siga
pendiente. Incorporar un perfil exige todos sus campos obligatorios; después
no se puede retirarlo. La edición conserva estado y etapa. Una revisión esperada
0 solo corresponde a una raíz sin historia administrativa, no a cualquier ficha
pendiente. Guardar los mismos valores también agrega una revisión.

| Campo | Máximo de escalares Unicode | Condición |
| --- | ---: | --- |
| `title` | 200 | Obligatorio. |
| `reference` | 100 | Obligatorio; referencia interna no única. |
| `nuc`, `judicial_case_number` | 100 cada uno | Obligatorios dentro del perfil. |
| `nuc_authority`, `judicial_authority` | 200 cada uno | Obligatorios dentro del perfil. |
| `offenses` | De 1 a 8 textos, 120 cada uno | Obligatorios, sin duplicados literales tras recortar. |
| `general_information` | 1000 | Opcional; admite varias líneas. |
| `complementary_identifiers` | 300 | Opcional. |

Se rechazan controles antes de recortar blancos Unicode exteriores. Información
general convierte CRLF a LF y admite LF; rechaza CR aislado y los demás controles.
Los otros textos rechazan todos los controles. Un opcional vacío queda `null`.
No se cambian mayúsculas, acentos ni espacios interiores. Delitos conserva orden;
una coma forma parte del texto, no separa entradas.

NUC y carpeta son únicos por separado entre perfiles vigentes de la instancia,
incluidos los cerrados. La comparación es literal; la autoridad es contexto,
no parte de la clave. Corregir un identificador libera el anterior al confirmar
la transacción, conservando su historia. El conflicto es genérico y no expone
otro expediente. Estos valores manuales no acreditan unicidad nacional ni
verificación institucional.

Los tres cuerpos JSON tienen límite de 64 KiB, incluidos espacios finales y
fragmentos de transporte. Rechazan campos desconocidos o repetidos, también
dentro del perfil. No admiten actor, fecha, etapa ni digest suministrados por
el cliente. Los máximos caben incluso con todos los escalares escapados.

## Estado administrativo

```json
{"expected_revision":3,"administrative_status":"closed"}
```

Los estados son `active` y `closed`. El comando conserva los textos y el perfil
vigentes dentro de la transacción y agrega una revisión. No sustituye una edición
completa ni cambia la etapa. Reactivar no reactiva participantes archivados.

El cierre impide editar el perfil, cargar archivos, agregar versiones,
clasificar, sellar y crear/editar/archivar/reactivar participantes. Mantiene
consultas, historias, verificación y descarga autorizadas; Owner conserva
asignación y revocación. Cada mutación comprueba el cierre después de autorizar
y resolver el recurso exacto, aun si preparó cifrado o sellado antes del cierre.
Un recurso ajeno conserva `404` sin revelar el estado de otro expediente.
Cerrar administrativamente no concluye un proceso judicial ni altera plazos.

## Consultas

El índice admite solo estos parámetros, combinados con AND sobre los valores
actuales antes de paginar:

| Parámetro | Valor |
| --- | --- |
| `limit` | Entero 1–100; predeterminado 50. |
| `after_id` | UUID exclusivo; orden UUID ascendente. |
| `status` | `active` predeterminado, `closed` o `all`. |
| `profile` | `all` predeterminado, `complete` o `pending`. |
| `title` | Subcadena literal que distingue mayúsculas; máximo 200. |
| `nuc`, `judicial_case_number` | Coincidencia literal exacta; máximo 100. |

Filtros vacíos se omiten después de normalizar. Controles o excesos de longitud
son errores semánticos. No se ofrecen totales ni instantánea compartida entre
páginas. La respuesta contiene `cases`, `has_more` y `next_after_id`; el cursor
es el último UUID devuelto solo si quedan resultados, de otro modo es `null`.

Cada fila contiene `id`, `title`, `reference`, `created_by`, `created_at`,
`revision`, `administrative_status`, `profile_status`, `penal_identifiers` e
`initial_stage`. Los identificadores son `null` o el objeto con `nuc` y
`judicial_case_number`; no incluye autoridades, delitos, información general,
digests ni correos históricos. `profile_status` es `pending` o `complete`;
`initial_stage` es `null` o `investigation`, independientemente de la revisión.

La historia admite `limit` 1–100, predeterminado 50, y `before_revision` entero
positivo exclusivo. Devuelve `revisions`, `has_more` y `next_before_revision`.
Las revisiones van en orden descendente y son siempre positivas; el cursor es
la última revisión devuelta solo cuando quedan resultados, de otro modo `null`.
No se fabrica una fila histórica para la proyección cero.

## Detalle y procedencia

El detalle contiene `id`, `created_by`, `created_at`, `administration` e
`initial_stage`. La administración contiene:

- `case_id`, `revision`, `title`, `reference`, `administrative_status` y `profile`.
- `values_digest`: SHA-256 de valores canónicos `CADM1`, en 64 caracteres hexadecimales minúsculos.
- `changed_at`: fecha UTC RFC3339 capturada en la revisión.
- `changed_by`: objeto con UUID `id` y `email` capturado, independiente del correo actual.

El perfil completo serializa siempre ambos opcionales, como texto o `null`.
La historia usa la misma forma de administración con procedencia no nula.
La raíz anterior sin revisiones se proyecta como revisión 0, estado activo,
perfil y los tres campos de procedencia `null`. La creación por API básica
produce revisión 1 real con perfil pendiente y sin etapa. Completar cualquiera
de esas fichas conserva la ausencia de etapa; no inventa un registro inicial.

Cuando existe, `initial_stage` contiene `case_id`, `stage_revision` 1,
`administration_revision` 1, `stage` `investigation`, `administration_digest`,
`recorded_at` y `recorded_by` con `id` y `email`. La huella y procedencia son
las de la revisión administrativa 1 exacta, incluso después de editar o cerrar.
Todas las fechas se serializan en UTC RFC3339.

## Errores y concurrencia

Se conserva la envoltura `{"error":{"code":"...","message":"..."}}`.

| Estado | Códigos |
| --- | --- |
| 400 | `invalid_json`, `invalid_query`, `invalid_case_id`. |
| 401 / 403 / 404 | `invalid_session`, `permission_denied`, `case_not_found`. |
| 409 | `case_revision_conflict`, `case_revision_exhausted`, `case_closed`, `case_identifier_conflict`, `case_profile_required`. |
| 413 | `case_administration_body_too_large`. |
| 422 | `invalid_penal_case_profile`, `invalid_case_metadata`, `invalid_input`. |
| 500 | `internal_error`, sin datos de almacenamiento o conexión. |

Tipos, parámetros desconocidos/repetidos y enteros negativos/desbordados son
400. Un estado desconocido produce `400 invalid_json`; una revisión histórica
cero o un límite fuera del intervalo producen `422 invalid_input`. La etapa
no es un dato de entrada de estas rutas.
Una revisión esperada desactualizada es 409: conservar el borrador, consultar
la versión actual, compararla y confirmar explícitamente el nuevo envío.
Si el estado consultado ya es el solicitado, la interfaz evita otro cambio.

No hay clave de idempotencia. Una conexión perdida puede ocurrir después del
commit; no demuestra rechazo. Conservar los datos, reconciliar mediante índice
y detalle autorizados y decidir el siguiente envío sin reintento automático.
Ante cierre concurrente, la interfaz conserva borradores, consulta el estado y
bloquea mutaciones. Una reapertura no envía esos borradores por sí sola.
