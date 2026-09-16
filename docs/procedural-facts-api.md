# API de hechos declarados de resolucion y notificacion

## Alcance y estado

El modulo `crates/web/src/procedural_facts/` adapta `ProceduralFactWorkflow`
a HTTP. Estan implementadas las rutas, entrada estricta, proyecciones y
composicion. Pasaron 25 pruebas focales de DTO/proyeccion, 26 de rutas con
puertos controlados y 3 de errores. La suite global y la comprobacion HTTP con
servicios reales y restauracion tambien estan aprobadas localmente. El cliente
Qadra esta implementado y su campana de navegador esta aprobada localmente; el
[informe de verificacion](verification-report.md) separa cada campana. Capturar una declaracion no valida
juridicamente un acto, su notificacion, su representacion o sus efectos, ni
habilita calculos operativos de plazos, recursos o alertas.

Son contratos complementarios el [modelo de valores](procedural-facts.md),
los [tiempos declarados](procedural-time.md), el [servicio](procedural-facts-application.md),
los [canones de fuentes y recibos](procedural-facts-receipts.md) y la
[persistencia PostgreSQL](procedural-facts-persistence.md). La motivacion esta
registrada en [ADR-0031](adr/0031-declared-procedural-facts.md).

## Autenticacion y permisos

Todas las rutas requieren `Authorization: Bearer <token>`. El servicio autentica
antes de consultar almacenamiento y reautentica antes de confirmar una escritura.
Owner consulta y gestiona cualquier expediente; Litigator requiere asignacion
vigente para consultar y gestionar; Paralegal asignado solo consulta. Client
no accede a estos hechos ni a sus fuentes. El adaptador comprueba cuenta,
rol, pertenencia y estado actual del expediente en cada operacion.

El cierre actual del expediente impide preparar y confirmar cambios; conserva
las consultas autorizadas de revisiones historicas. Una declaracion retirada
sigue siendo consultable y puede ser fuente exacta de otra declaracion.
El retiro es terminal para esa raiz y no afirma nulidad juridica.

## Rutas

Las dos familias tienen el mismo conjunto de operaciones. Se emplean estas bases:

- Resoluciones: `/api/v1/cases/{case}/resolutions`, identificador de elemento `{resolution}`.
- Notificaciones: `/api/v1/cases/{case}/resolutions/{resolution}/notifications`,
  identificador de elemento `{notification}`.

| Metodo y sufijo respecto de la base | Operacion | Exito |
| --- | --- | --- |
| `GET /` | Listar cabezas autorizadas | 200 |
| `POST /prepare` | Preparar cualquier accion de la familia | 200 |
| `POST /` | Confirmar alta `record` | 201 |
| `GET /{id}` | Consultar cabeza exacta devuelta por el servicio | 200 |
| `PUT /{id}` | Confirmar correccion `correct` | 201 |
| `POST /{id}/withdrawal` | Confirmar retiro `withdraw` | 201 |
| `GET /{id}/revisions/{revision}` | Consultar revision positiva exacta | 200 |
| `GET /{id}/history` | Listar historial ligero | 200 |

El sufijo `/` de la tabla representa la propia base, sin exigir barra final.
La familia, UUID de elemento, padre y accion del cuerpo deben coincidir con la
ruta. Cambiar el padre tanto en el cuerpo como en sus valores no cambia la
identidad de una notificacion almacenada. Una correccion puede seleccionar
otra revision del mismo padre, conservando su UUID.

## Consultas y limites

Listados admiten `limit` entre 1 y 100, por defecto 20; `after_id` es un cursor
UUID exclusivo; `status` acepta `all` (defecto), `recorded` o `withdrawn`.
El orden es UUID ascendente y el UUID cero sigue siendo representable.
La seleccion de cabezas antecede al filtro de estado en almacenamiento.

Historial admite `limit` entre 1 y 20, por defecto 10, y `before_revision`
positivo y exclusivo. Las revisiones se ordenan de mayor a menor. Listados e
historial devuelven `has_more` y su cursor siguiente, o `null` cuando no hay
continuacion. Si hay continuacion, el servicio exige una pagina llena y un
cursor que corresponda a su ultima fila.

Los nombres de consulta desconocidos, repetidos o valores malformados se
rechazan. Detalles, revisiones exactas y mutaciones no admiten parametros de
consulta. Una consulta vacia no agrega parametros.

## Cuerpos de comandos

El limite de cuerpo es 512 KiB. Se exige JSON con tipo `application/json` o
`application/*+json`. Los objetos rechazan campos desconocidos, claves
repetidas y representaciones posicionales como arreglos.
Los UUID se validan como UUID y los digests como 64 caracteres hexadecimales.
La salida presenta UUID normalizados y digests hexadecimales.

`POST /prepare` recibe directamente un comando:

```json
{
  "family": "resolution",
  "operation_id": "00000000-0000-0000-0000-000000000101",
  "id": "00000000-0000-0000-0000-000000000201",
  "change": {
    "action": "record",
    "expected_revision": 0,
    "values": {
      "class": {"kind": "known", "value": {"kind": "order"}},
      "subtype": null,
      "issuer": {"kind": "unknown", "reason": "Emisor no identificado"},
      "issued_at": {"precision": "unknown"},
      "summary": "Resolucion declarada por la operadora",
      "provenance": {"kind": "operator_note", "note": "Captura manual"}
    }
  }
}
```

Una notificacion usa `family: "notification"` y agrega `resolution_id` al
comando. Sus `values.resolution` seleccionan `{id, revision}` del mismo padre.
`record` requiere `expected_revision: 0` y valores completos.
`correct` requiere revision esperada positiva, valores completos y `reason`.
`withdraw` requiere revision positiva y `reason`; no admite valores nuevos.
El contador agotado se rechaza antes de producir una revision sucesora.

Las tres rutas de confirmacion reciben este sobre:

```json
{
  "command": {"family": "resolution", "operation_id": "UUID", "id": "UUID", "change": {}},
  "expected_submission_digest": "64 caracteres hexadecimales"
}
```

Este segundo ejemplo muestra solo la estructura del sobre, no un comando valido
completo. Se debe enviar el comando normalizado y el digest obtenidos al preparar.
La preparacion no reserva identidad ni revision. Confirmar vuelve a preparar,
revalida el lote directo y exige el digest esperado antes del commit auditado.

## Valores declarados y tiempo

Las declaraciones usan `{kind: "known", value: ...}` o
`{kind: "unknown", reason: ...}`. Los catalogos, personas, representacion,
procedencia y localizadores conservan las formas del
[corpus de valores](../crates/domain/tests/fixtures/procedural_fact_vectors.json).
Campos opcionales aceptan omision o `null`; la salida expresa su ausencia con
`null`. No se deducen personas ni vinculos a partir del texto.

El tiempo declarado usa `precision`:

- `unknown`: solo ese campo.
- `date`: `year`, `month`, `day`, `offset_seconds` opcional o `null`.
- `minute`: los anteriores mas `hour` y `minute`, sin `second`.
- `second`: los anteriores mas `second`.

No se completan componentes ausentes. Desfase ausente y UTC (`0`) son diferentes;
una fecha no se convierte a medianoche ni una precision de minuto a segundo.
Los componentes no permitidos para una precision se rechazan. Se conservan
los limites y las comprobaciones de representabilidad del contrato temporal.

## Borrador y detalle

El borrador devuelve:
`case_id`, `actor_id`, `command`, `result_revision`, `values`, `values_digest`,
`sources`, `sources_digest`, `submission_digest` y `observed_administration`.
El comando normalizado debe ser el preparado; los valores de alta/correccion
coinciden con el comando. Retiro devuelve los valores conservados de su base.

El detalle devuelve:
`case_id`, `family`, `id`, `revision`, `values`, `values_digest`, `status`,
`reason`, `receipt`, `recorded_administration`, `recorded_at`, `recorded_by`
y `sources`. Solo la familia notificacion agrega `resolution_id`.
`recorded_by` contiene `{id, email}` capturados al escribir; `recorded_at`
es RFC 3339 en UTC y pertenece al servidor, no al tiempo declarado del acto.

El recibo contiene `operation_id`, `action`, `expected_revision`,
`sources_digest` y `submission_digest`. El servicio verifica valores y recibos
por los canones existentes. HTTP comprueba ademas alcance, revision y
coherencia estructural antes de serializar, sin introducir otro algoritmo
criptografico ni sustituir la carga de fuentes reales.

### Administracion capturada

`observed_administration` y `recorded_administration` son objetos discriminados:

- `unrevised`: `{kind, title, reference, status: "active"}`. No hay numero de
  revision, digest, autor ni fecha inventados para la base historica R0.
- `recorded`: `{kind, case_id, revision, title, reference, status, values_digest,
  changed_at, changed_by}`. `changed_by` contiene `{id, email}` y `changed_at`
  es RFC 3339 UTC. La revision y el digest fijan el contexto administrativo;
  esta respuesta no repite el perfil penal completo.

La captura administrativa fue activa al registrar el hecho. Esto es compatible
con consultar ese hecho cuando el expediente se encuentra actualmente cerrado.
La observacion no actua como revision esperada administrativa ni como reserva.

## Fuentes legibles y exactas

`sources` tiene cuatro campos:

| Campo | Forma de cada fuente |
| --- | --- |
| `resolution`, objeto o `null` | `case_id`, `id`, `revision`, `values_digest`, `submission_digest`, `status`, `class`, `issuer`, `issued_at`, `summary` |
| `participants`, arreglo | `case_id`, `id`, `revision`, `values_digest`, `directory_status`, `subject`, `display_name`, `procedural_role`, `organization`, `kind` |
| `hearing_results`, arreglo | `case_id`, `hearing_id`, `result_id`, `revision`, `agreement_id`, `values_digest`, `submission_digest`, `status`, `occurrence`, `event_time`, `summary`, `agreement` |
| `direct_supports`, arreglo | `document_id`, `version`, `digest`, `name`, `format`, `policy` |

`subject` es `null` para ficha manual o `{id, revision, values_digest}` para
la identidad ligada exacta de una ficha tipificada; no revela su identidad
completa, CURP, credenciales o documentos privados. `kind` y `organization`
pueden ser `null`. Cada ficha conserva el estado historico, incluso `archived`.

El tiempo de un resultado de audiencia conserva su modelo propio: `date` o
`instant`, `year`, `month`, `day`, `offset_seconds` obligatorio; solo `instant`
agrega `hour`, `minute`, `second`. No se cambia HRES1 ni se interpreta el inicio
del dia como hora de la audiencia. `agreement` es `null` o `{id, text}`;
`agreement_id: null` no equivale a seleccionar el UUID cero.

Los soportes exponen admision capturada (`pdf` o `docx`, politica `pdf_docx_v1`),
no contenido en claro. La union directa tiene como maximo dos versiones y
no expande documentos del padre o de antecedentes. Cada fuente, vista y
referencia debe coincidir con la seleccion completa de valores. No se
actualiza una referencia historica a su cabeza actual ni se descarta una
fuente extra silenciosamente. Un estado historico retirado no invalida por si
solo la seleccion exacta.

## Listados e historial

Una pagina de resoluciones devuelve `resolutions`, `has_more`, `next_after_id`.
Cada fila contiene `case_id`, `family`, `id`, `revision`, `status`, `class` e
`issued_at`. Una pagina de notificaciones usa `notifications`; cada fila
contiene `case_id`, `family`, `id`, `resolution_id`, `revision`, `status`,
`resolution: {id, revision}`, `outcome` y `practiced_at`.

El historial devuelve `revisions`, `has_more`, `next_before_revision`.
Sus filas incluyen identidad completa, metadatos, recibo y administracion
capturada, sin repetir valores o fuentes. Consultar el detalle exacto permite
recuperar esos cuerpos; una fila ligera no los sustituye.

## Errores y confirmacion incierta

| HTTP | Codigos o situacion |
| --- | --- |
| 400 | JSON/consulta/UUID malformado, tipo de contenido no JSON (`invalid_json`), `procedural_fact_command_mismatch`, digest malformado |
| 401 | Sesion ausente, invalida o revocada |
| 403 | Rol sin permiso |
| 404 | `case_not_found`, `procedural_fact_not_found`, `procedural_fact_reference_not_found` |
| 409 | `case_closed`, `procedural_fact_revision_conflict`, `procedural_fact_already_withdrawn`, `procedural_fact_operation_conflict`, `procedural_fact_submission_mismatch`, `procedural_fact_support_changed`, `procedural_fact_revision_exhausted` |
| 413 | `procedural_fact_body_too_large` |
| 422 | Valores/tiempo/referencias invalidos, soporte excesivo, formato rechazado, presupuesto de validacion o digest documental incorrecto |
| 500 | Inconsistencia del puerto o persistencia, fallo interno criptografico o de infraestructura |
| 503 | Presupuesto compartido de operaciones agotado |

El detalle exacto y su recibo permiten cotejar una respuesta perdida contra
la operacion enviada. Reenviar no constituye una lectura idempotente: una
operacion ya usada se rechaza. Un 404, una revocacion o un error de red no
prueban por si mismos si hubo commit. No existe reenvio automatico; el cliente
Qadra incorpora la consulta y conciliacion explicita descritas abajo.


## Cliente Qadra

El cliente está implementado en `web/src/components/CaseFacts.svelte`, con
formularios de campos separados de los helpers de valores, API y conciliación.
El [recorrido de la interfaz](../web/README.md#resoluciones-y-notificaciones-declaradas)
describe sus controles. La campaña de navegador está aprobada localmente con HTTP simulado y servicios
reales aislados; las cifras y límites se conservan en el informe de verificación.

Las dos familias ofrecen lista con filtro de estado y cursor, detalle,
historia exacta, alta, corrección y retiro. Las notificaciones se navegan bajo
su resolución fija. El formulario puede seleccionar otra revisión de ese
padre sin cambiar la raíz. Los datos conocidos y desconocidos, la precisión
temporal, el desfase opcional, las personas no vinculadas y la representación
se eligen expresamente. No hay entrada de JSON libre ni identificadores
manuales como mecanismo principal de selección de fuentes.

Los selectores de participantes, resultados y documentos consultan de forma
secuencial listas, historia y detalle exacto. No excluyen por sí mismos fichas
archivadas, resultados retirados o resoluciones históricas. La selección de
un resultado admite ausencia de acuerdo o un UUID de acuerdo concreto, incluido
el UUID cero. El servidor revalida la autorización, el alcance y la fuente.
La preparación devuelve nombres, estados, metadatos y huellas históricas;
la UI muestra esas vistas sin convertirlas en consultas de datos actuales.

Confirmar exige una preparación revisada. La conciliación compara identidad
completa, revisión, actor, operación, acción, valores, fuentes, recibo y captura
administrativa compatibles con el envío conservado. No calcula los cánones
criptográficos en JavaScript. Solo `procedural_fact_not_found` al consultar la
revisión objetivo produce el estado de ausencia todavía incierta;
`case_not_found`, `procedural_fact_reference_not_found` y cambios de sesión o
permiso conservan su significado propio. No se reenvía automáticamente.

La interfaz bloquea acciones locales durante sus consultas auxiliares. El
presupuesto global de operaciones pertenece al servidor, que puede responder
503. Los formularios conservan borradores ante conflictos; el cierre bloquea
escrituras y la consulta histórica sigue su autorización vigente. La captura
no determina validez, representación eficaz, notificación consumada, firmeza,
recurso procedente, inicio de plazo ni alerta.
