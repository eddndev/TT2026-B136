# API del catálogo de perfiles de plazo

Estado: API HTTP implementada, validada con un workflow simulado en 16 pruebas
y con persistencia y restauración reales. El recorrido recuperó diez respuestas
exactas del catálogo; su evidencia está en [el informe](verification-report.md). Esta API no registra todavía una evaluación del expediente ni envía
alertas. Véanse [ADR0035](adr/0035-versioned-deadline-profiles-and-evaluations.md) y
[el alcance operativo](deadline-lifecycle.md).

## Colección y autorización

Las dos bases siguientes ofrecen las mismas operaciones:

- `/api/v1/deadline-profiles`: exclusivamente perfiles globales, incluso para Owner.
- `/api/v1/cases/{case_id}/deadline-profiles`: autoriza ese expediente antes de
  consultar sus perfiles y los globales. No acepta un ámbito alternativo en query.

Owner publica, reemplaza y retira. Owner, Litigator y Paralegal consultan;
Litigator y Paralegal requieren asignación actual al consultar una colección de
expediente. Client queda denegado. El cierre conserva lecturas y bloquea
mutaciones privadas. Una mutación global utiliza siempre la base global.

El ámbito completo es inmutable desde R1. Global no significa aplicabilidad a
todos los expedientes: describe una configuración disponible con ámbito declarado.
Los UUID, incluido cero, no se introducen como nombres ni se deducen del texto.

| Método y sufijo de la base | Operación |
| --- | --- |
| GET `/` | Lista ligera, por UUID ascendente. |
| GET `/{id}` | Detalle de cabeza actual. |
| GET `/{id}/revisions/{revision}` | Detalle exacto, incluso retirado. |
| GET `/{id}/history` | Revisiones ligeras, orden descendente. |
| POST `/prepare` | Prepara un comando sin reserva ni escritura. |
| POST `/` | Confirma publicación. |
| PUT `/{id}` | Confirma reemplazo completo. |
| POST `/{id}/retirement` | Confirma retiro terminal. |

La barra final de la tabla representa el recurso base, sin requerir `/` final.
Lista: `limit`1..100, por defecto20; `after_id` UUID opcional; `status` publicado
por defecto (`published`), `all` o `retired`. Historia: `limit`1..20, por defecto10;
`before_revision` u32 positivo opcional. Detalle, revisión exacta y mutaciones
no admiten parámetros. Parámetros desconocidos o repetidos producen400.

## Comandos

Preparación recibe `{operation_id,profile_id,change}`. `change` es una variante:

- `{action:"publish",expected_revision:0,definition}`.
- `{action:"replace",expected_revision,definition,reason}`.
- `{action:"retire",expected_revision,reason}`; no lleva definición.

Confirmación recibe `{command,expected_submission_digest}`. Operación y raíz
son UUID; revisión esperada positiva para reemplazo/retiro, sin desbordamiento.
Motivo1..1000 escalares, multilineal. La ruta, acción, identidad y colección
deben coincidir. Algoritmo no es un campo editable: publicación/reemplazo usan
`v1`; el retiro conserva definición y algoritmo anteriores.

Preparación devuelve200, confirmación201. DPTX1 vincula actor, operación, raíz,
acción, revisión base, algoritmo, huella DPRF y motivo. El cliente conserva el
comando preparado y la huella; no implementa el canon ni criptografía. Una
respuesta perdida se consulta mediante revisión exacta y recibo;404 no prueba
que no se haya confirmado. No se reenvía automáticamente una escritura.

## Definición estructurada

`definition` requiere todos estos campos, salvo opcionales expresos:

| Campo | Forma y límites |
| --- | --- |
| `title` | Texto1..200, una línea. |
| `description` | Texto1..1000, multilineal. |
| `scope` | `{kind:"global",value:CalendarScope}` o `{kind:"case",case_id:UUID}`. |
| `references` |1..16 referencias públicas con el mismo JSON `Source` del calendario. |
| `trigger` | Requisito tipificado descrito abajo. |
| `template` | Regla fija o unidad con cantidad ordenada, descrita abajo. |
| `completion` | Política explícita de finalización, descrita abajo. |
| `conditions` |1..16 `{id,statement,reference_ids}`; statement1..1000 multilineal. |
| `examples` |1..16 ejemplos completos descritos abajo. |

`CalendarScope`, `Source` y `CalendarValues` reutilizan exactamente
[los valores de calendarios](judicial-calendars-api.md#valores), incluyendo
fechas civiles, entidades, URLs y reglas/excepciones explícitas. No se descargan
referencias. El calendario de un ejemplo contiene valores completos de prueba;
no es una referencia al catálogo ni una selección de calendario para un expediente.

Cada colección identifica sus entradas por UUID único; `reference_ids` contiene
1..16 UUID únicos de `references` de esta definición. Se conserva el orden
explícito del corpus, condiciones y referencias del perfil. Los calendarios
anidados conservan su propia normalización y orden canónico.

Los textos recortan espacios exteriores y normalizan CRLF a LF. No se normaliza
Unicode interior. Rechazar controles salvo LF en campos multilineales; cotas por
escalares Unicode. No rellenar condiciones, precisión, cantidades ni políticas.

### Requisito temporal

- `{kind:"source_field",field}`: `resolution_issued_at`,
  `notification_practiced_at`, `notification_received_at`,
  `notification_stated_effect_at` o `hearing_session_event_time`.
- `{kind:"qualified",purpose,family}`: purpose `hearing_end` u
  `ordered_period_start`; family `resolution`, `notification` o `hearing_result`.

La configuración no selecciona por sí sola un hecho ni interpreta su narrativa.

### Regla y finalización

`template` es `{kind:"fixed",rule}` o
`{kind:"ordered",unit,maximum:null|u32_positivo}`. Regla y unidad usan `kind`:

- `days`: `inclusion` = `on_anchor`/`after_anchor`, `basis` =
  `natural`/`calendar_countable`, `final_day` = `preserve`/`next_countable`.
- `civil_months`: `final_day`, con los mismos valores.
- `elapsed_hours`: sin políticas civiles.

Solo `rule` requiere `quantity` u32 positivo; `unit` no la admite. Un máximo
nunca rellena una cantidad ordenada ausente. Los meses no recortan días inválidos.

`completion` tiene `kind`:

- `arithmetic_instant`, obligatorio exclusivamente para horas transcurridas.
- `civil_candidate_only`, conserva fecha sin inventar hora.
- `civil_cutoff`, con `time` exacto `HH:MM:SS`, `offset_seconds` entero múltiplo
  de60 en±50400, `from`/`through` fechas inclusivas1..1096, `channel`1..200 y
  `reference_id` presente en referencias. El corte tiene desfase propio.

### Ejemplos reproducibles

Cada ejemplo es `{id,anchor,ordered_quantity,calendar,expected,reference_ids,locator}`.
Locator1..200, una línea; cantidad opcional positiva; calendario opcional completo.
Los opcionales ausentes se proyectan como null. `anchor` reutiliza el tiempo de
[hechos declarados](procedural-facts-api.md): precision `unknown` sin componentes;
`date` con año/mes/día; `minute` añade hora/minuto; `second` añade segundo.
`offset_seconds` opcional no se sustituye por UTC; los componentes omitidos por
precisión se rechazan si aparecen. Rigen las validaciones temporales existentes.

`expected` es `{kind:"arithmetic",outcome}` o `{kind:"rule_blocked",block}`.
Outcome tiene una de estas formas:

- `{kind:"civil_candidate",date:"YYYY-MM-DD"}`.
- `{kind:"instant_candidate",instant:{unix_seconds,nanosecond,offset_seconds}}`.
- `{kind:"blocked",block}`.

El instante esperado conserva segundos Unix i64, nanosegundo0..999999999 y el
desfase original admitido por OffsetDateTime. No se limita al perfil temporal
del acto declarado ni se pierde su representación al leer historia. El motor
compara el instante esperado; el canon conserva también su desfase original.

Bloqueos aritméticos: `{kind:"unknown_anchor"}`, `missing_offset`,
`missing_calendar`, `date_range_exhausted`; `insufficient_precision` añade
`observed` (`unknown`/`date`/`minute`/`second`); `missing_homologous_day` añade
`year`u32, `month`u8 y `requested_day`u8; `unresolved_calendar_date` y
`outside_calendar_coverage` añaden `date` civil. Cada variante solo admite sus campos.

Bloqueos de cantidad: `missing_ordered_quantity`, `unexpected_ordered_quantity`,
o `ordered_quantity_exceeds_maximum` con `maximum` y `supplied` positivos.
El servidor reproduce todos los ejemplos y exige al menos una candidata exitosa.
Una discrepancia rechaza la definición; no corrige silenciosamente el resultado
esperado ni acredita interpretación jurídica.

## Respuestas

Colección: `{kind:"global"}` o `{kind:"case",case_id}`; acompaña las respuestas.
Preparación: `{collection,actor_id,command,result_revision,initial_scope,definition,
algorithm,definition_digest,submission_digest}`. Command y definición están normalizados.

Detalle: `{collection,id,revision,status,algorithm,definition_digest,scope,reason,
receipt,recorded_at,recorded_by,definition}`. Scope coincide con definition.scope.
`recorded_at` es RFC3339 del servidor; `recorded_by` conserva `{id,email}` histórico.
Receipt: `{operation_id,action,expected_revision,submission_digest}`.

Lista: `{collection,profiles,has_more,next_after_id}`. Cada perfil conserva
`id,revision,status,algorithm,definition_digest,title,scope`, sin ejemplos.
Historia: `{collection,revisions,has_more,next_before_revision}`; cada entrada
conserva los campos de detalle salvo `collection` y `definition`. No amplía corpus.
Cursores terminales son null; una página con más resultados ocupa su límite.
Las proyecciones se validan contra identidad, colección y solicitud antes de devolverse.

## Rechazos y límites

Bearer se valida antes de cuerpo/parámetros y el workflow reautentica al operar.
JSON exige objetos con nombres, sin campos desconocidos ni duplicados en ningún
nivel; arrays posicionales no representan objetos. Content-Type JSON único,
texto posterior al objeto o JSON mal formado producen400 `invalid_json`.

| HTTP | Código |
| --- | --- |
|400 | `invalid_query`, `invalid_case_id`, `invalid_deadline_profile_id`, `invalid_deadline_profile_operation_id`, `invalid_deadline_profile_revision`, `invalid_deadline_profile_digest`, `deadline_profile_command_mismatch`. |
|401/403 | Sesión inválida/permiso denegado, códigos comunes existentes. |
|404 | `deadline_profile_not_found`; expediente inaccesible conserva `case_not_found`. |
|409 | `deadline_profile_revision_conflict`, `deadline_profile_operation_conflict`, `deadline_profile_retired`, `deadline_profile_revision_exhausted`, `deadline_profile_scope_change_forbidden`, `deadline_profile_submission_mismatch`; cierre conserva `case_closed`. |
|413 | `deadline_profile_body_too_large`. |
|422 | `invalid_deadline_profile`, `deadline_profile_example_mismatch`, `deadline_profile_no_successful_example`, o validaciones existentes de calendario/tiempo/texto anidados. |
|500 | Inconsistencia almacenada o proyección incoherente; no incluir contenido privado en error. |

Límite16MiB =16777216bytes para el cuerpo completo, incluida confirmación. Una
cota conservadora de JSON compacto normalizado es15607604bytes: contempla16
calendarios máximos, escapes ASCII en claves/UUID/fechas/URL y hasta12bytes por
escalar Unicode, más el sobre de confirmación. No es un máximo alcanzable.
Espacio sintáctico o texto exterior descartado por trim puede crecer sin cota;
una representación equivalente que exceda el presupuesto también produce413.
El límite se comprueba antes de deserializar; no hay decodificación base64 de DPRF.
