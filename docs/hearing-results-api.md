# API de sesiones y resultados declarados de audiencia

Estado: implementado y verificado localmente, pendiente de integración. Fundamento en [ADR-0029](adr/0029-declared-hearing-sessions.md).
No modifica la programación ni reinterpreta sus cánones HEAR1 y HTXN1.

## 1. Corte y decisiones fijadas

- Raíz propia por sesión/acto declarado; varias raíces por audiencia, sin deduplicar
  por fecha, asistentes o texto. Una revisión corrige una captura; una continuación
  crea otra raíz. No determinar validez judicial, notificación ni nulidad.
- Ancla inmutable a una revisión EXACTA de programación del mismo expediente,
  seleccionada expresamente. No sustituirla por la cabecera al preparar o confirmar.
  Se admite revisión `scheduled` o `cancelled`, con su estado visible.
- Continuación opcional a una revisión exacta de otra raíz de resultado del mismo
  expediente, incluso retirada. Debe existir antes del alta, no ser la propia raíz,
  y no cambia después. No consultar su cabecera actual para autorizar el vínculo.
- El módulo no bloquea globalmente reemplazar/cancelar programación ni requiere
  modificar esas operaciones. Una revisión posterior no invalida el ancla histórica.
- Acciones `record`, `correct`, `withdraw`. Retiro administrativo conserva contenido;
  no anula el acto. `withdrawn` es terminal, sin reactivación ni borrado físico.
- `occurred/not_started` separado de `partial/concluded/unspecified`.
  `not_started` exige `unspecified`; permite comparecencias y acuerdos.
- Captura tardía y etapa posterior admitidas. Sin CAS de administración/etapa y sin
  exigir perfil penal completo actual. Revalidar permiso y expediente activo tras
  el lock, capturando entonces la administración vigente.
- Owner gestiona; Litigator asignado gestiona; Paralegal asignado consulta; Client
  denegado. Expediente cerrado conserva consulta e historia y bloquea escrituras.
- Tiempo declarado independiente de programación/captura, con fecha o instante.
  Cota no futura usando Clock después del lock. Nunca inventar una hora conocida.
- Máximo 32 comparecencias de fichas históricas exactas y 16 acuerdos declarados.
  No cálculo, alertas, avance de etapa ni disparadores extraídos del texto.

## 2. Identidades, fuentes y estados

`HearingResultId`, `HearingResultOperationId`: UUID. Revisión u32 positiva, inicial
1, sucesión exacta sin wrap; la expectativa del alta es 0. Un UUID de operación
es único en TODA la familia de resultados, no solo dentro de una raíz. No prometer
unicidad entre programación y resultados: tienen prefijos y recibos distintos.

Raíz: `id`, `case_id`, `hearing_id`, `anchor_revision`, digest de valores HEAR1 y
digest del recibo HTXN1 del ancla; continuidad opcional con `result_id`, `revision`,
`hearing_id`, digest de valores y digest del recibo del antecedente. Los digests y
el hearing_id del antecedente se resuelven en servidor. No llegan como autoridad
desde un campo editable. La asociación y ambos antecedentes quedan inmutables.

Revisión: valores HRES1, digest, acción/estado, motivo, recibo HRTX1, actor/email
capturados, instante de captura UTC, revisión/digest administrativos vigentes y
proyección admitida del soporte. `record/correct` producen `recorded`; `withdraw`
produce `withdrawn` y copia exactamente valores, ancla, continuidad y soporte base.
No es una declaración de que el hecho se volvió falso.

Se permite corregir con valores normalizados iguales y motivo nuevo: sigue siendo
otra operación explícita auditada, no una deduplicación por contenido. Después de
retirar, una nueva captura exige intención explícita y UUID nuevo; no crear una
continuación automáticamente para representar la recuperación de un error.

## 3. Valores y normalización

| Campo | Contrato |
| --- | --- |
| `occurrence` | `occurred` o `not_started`; selección explícita. |
| `extent` | `partial`, `concluded`, `unspecified`; selección explícita, con regla anterior. |
| `event_time` | Unión fecha/instante descrita abajo; requerida para ambos estados. Describe el hecho/comparecencia informado, no un inicio de plazo. |
| `summary` | Relato del operador, obligatorio, 1..1000 escalares Unicode. |
| `attendees` | 0..32 objetos exactos; vacía significa comparecencias no registradas. Una lista no afirma ser exhaustiva. |
| `attendees[].participant_id/revision` | Ficha del mismo expediente y revisión positiva, incluyendo historia/archivo. No exigir vigencia ni estado activo de esa revisión. |
| `attendees[].capacity` | Calidad declarada en esta sesión, texto obligatorio 1..100, una línea. No reemplaza el rol del directorio. |
| `attendees[].observation` | Texto opcional 1..500, admite LF; no exigir horas de entrada/salida. |
| `agreements` | 0..16 objetos `{id,text}`. UUID único dentro de la lista, texto 1..1000. No tipificar como acuerdo probatorio ni afirmar unanimidad. |
| `provenance.kind` | `operator_note`, `oral_reference`, `written_record`; clase de antecedente declarado. |
| `provenance.reference` | Localizador descriptivo 1..200, una línea; obligatorio para oral/written, opcional para operator_note. No implica descarga ni acceso a URL. |
| `provenance.support` | Opcional, una versión documental exacta: `{document_id,version,digest}`. PDF/DOCX hasta 16 MiB bajo admisión existente. |

Recortar espacio exterior y normalizar CRLF a LF; rechazar otros controles. LF solo
en los campos multilineales. No normalizar Unicode interior. Cotas por escalares,
longitudes canónicas por bytes UTF-8. Opcionales ausentes/null o texto opcional que
queda vacío normalizan a null; campo obligatorio vacío causa 422.

Ordenar asistentes por bytes UUID; duplicado de ficha se rechaza aunque cambien
revisión/nombre/calidad. Dos fichas homónimas pueden seleccionarse. Cada selección
es expresa, también si procede de convocados del ancla; no completar ausencias ni
inferir notificación. Conservar orden explícito de acuerdos: reordenar cambia el
contenido. Un acuerdo rectificado conserva UUID; uno nuevo recibe otro. Quitar un
acuerdo afecta la nueva revisión, no la historia. Referencias futuras a un acuerdo
necesitarán raíz, revisión e id; el UUID aislado no identifica un hecho jurídico.

`summary` siempre es relato del operador. Procedencia común y soporte coexisten;
una referencia oral puede adjuntar constancia escrita sin convertirse en emisión
escrita ni sustituir lo comunicado. No importar audio/video, abrir referencias ni
exigir soporte como formalidad legal. No capturar fuente/sujetos por acuerdo en
este corte; un cálculo posterior necesitará su propio antecedente estructurado.

### Tiempo declarado

```json
{"precision":"date","date":"2026-09-16","offset":"-06:00"}
{"precision":"instant","at":"2026-09-16T10:30:00-06:00"}
```

Fecha exacta YYYY-MM-DD y desfase de minutos entre -14:00 y +14:00. Instante RFC3339
con segundos enteros, T/Z mayúsculas, desfase explícito; aceptar Z o +00:00,
rechazar -00:00, fracciones y segundos intercalares. Años locales/UTC 1..9999.
Para fecha, comprobar representabilidad UTC del día entero y conservar fecha y
desfase; para instante, conservar también el desfase aunque otro describa igual UTC.
No reutilizar/refactorizar `DeclaredStageTime`; reproducir su distinción semántica
mediante tipo de resultados, con segundos enteros como cota propia.

Alta/corrección: instante <= captura; fecha admite el día cuando su límite inferior
UTC <= captura. Esa cota NO se convierte en hora del acto. Retiro no vuelve a
juzgar el tiempo histórico contra un reloj que pudo retroceder. No imponer orden
entre tiempos de programación, antecedente y resultado: puede haber precisión
incompleta, captura tardía o corrección. La aciclicidad depende de las raíces.

## 4. DTO de comandos y preparación

Rutas bajo `/api/v1`, bearer. El expediente y actor vienen de ruta/sesión. El
hearing_id del cuerpo debe coincidir con la ruta y con la raíz existente.

```json
{
  "operation_id":"00000000-0000-4000-8000-000000000101",
  "hearing_id":"00000000-0000-4000-8000-000000000102",
  "result_id":"00000000-0000-4000-8000-000000000103",
  "change":{
    "action":"record","expected_revision":0,"anchor_revision":2,
    "continuation":null,
    "values":{
      "occurrence":"occurred","extent":"partial",
      "event_time":{"precision":"date","date":"2026-09-16","offset":"-06:00"},
      "summary":"Relato comunicado por el operador",
      "attendees":[],"agreements":[],
      "provenance":{"kind":"operator_note","reference":null,"support":null}
    }
  }
}
```

Continuidad de alta: `continuation:{result_id,revision}` o null. No se asignan a
ciegas ancla ni antecedente desde sus cabeceras. `correct` contiene solamente
`action`, `expected_revision` positiva, `values`, `reason` 1..1000. `withdraw`
contiene solamente `action`, `expected_revision` positiva y `reason`. El servidor
deriva las fuentes fijas de la base; rechazar campos de alta en estas acciones.

Preparación devuelve `case_id`, `actor_id`, `command` normalizado, `result_revision`,
`values`, `values_digest`, `anchor`, `continuation`, `submission_digest` y
`observed_administration` informativa (revisión/digest/estado). Esta última no es
CAS ni promesa de qué revisión administrativa terminará capturándose. Incluye las
proyecciones exactas de participantes y soporte admitido necesarias para revisión.
El contrato no crea ni reserva raíz/operación; no guarda preparación temporal.

Para confirmar: `{command,expected_submission_digest}` con el mismo comando
normalizado. Recalcular valores/recibo y volver a resolver fuentes. Action, método,
hearing_id, result_id y ruta deben coincidir. No aceptar un digest como autorización.
Cada confirmación resuelve de nuevo la sesión, incluida después del trabajo de
formatos/criptografía, y revalida actor/membresía/expediente tras el lock.

## 5. Endpoints y consultas

Prefijo de tabla: `/cases/{case_id}/hearings/{hearing_id}/results`.

| Método | Sufijo | Resultado |
| --- | --- | --- |
| POST | `/prepare` | Propuesta; HTTP 200. |
| POST | vacío | Alta `record`; HTTP 201. |
| GET | vacío | Raíces de esta audiencia, cada una con cabecera actual. |
| GET | `/{result_id}` | Detalle de cabecera. |
| PUT | `/{result_id}` | Rectificación `correct`; HTTP 201. |
| POST | `/{result_id}/withdrawal` | Retiro `withdraw`; HTTP 201. |
| GET | `/{result_id}/revisions/{revision}` | Detalle exacto, también retirado/histórico. |
| GET | `/{result_id}/history` | Revisiones descendentes, separado del listado de raíces. |

Listado: `status=all|recorded|withdrawn`, all por defecto; `limit=1..100`, 20 por
defecto; `after_id` exclusivo, UUID ascendente. Sin filtro temporal ni orden
cronológico implícito de actos. Resolver cabeceras antes de estado/paginación.
Respuesta `{results,has_more,next_after_id}` con resúmenes: id/case/hearing/revisión,
estado, occurrence/extent/event_time, número de asistentes/acuerdos y anchor_revision.
No incluir relato, nombres, localizadores o soporte en ese listado.

Historia: `limit=1..20`, 10 por defecto; `before_revision` positiva exclusiva;
`{revisions,has_more,next_before_revision}`. Entradas ligeras con scope, revisión,
estado, motivo, digests, recibo, referencias de ancla/continuación y administración,
recorded_by/recorded_at. El contenido y las proyecciones de asistentes/soporte se
consultan en el detalle exacto, sin multiplicarlos por cada fila de historia.

Sin endpoint global, agenda nueva, búsqueda inversa ni árbol de continuaciones.
La UI inicia continuación desde un detalle exacto ya consultado. El detalle del
antecedente incluye hearing_id para navegar aunque corresponda a otra audiencia.
Permisos antes de devolución/paginación, consultas auditadas. Páginas pueden cambiar
entre peticiones; no prometer snapshot global ni usar fanout de expedientes.

JSON: objetos/campos estrictos, duplicados rechazados recursivamente, sin contenido
posterior ni Content-Type duplicados; números enteros positivos donde corresponda.
UUID se normaliza por el patrón actual; SHA256 a hexadecimal canónico. Comandos de
resultados limitados a **512 KiB = 524288 bytes**, incluido todo JSON/espacio.
Excederlos causa 413 aunque los valores de dominio sean válidos. No prometer
aceptar cualquier representación con espacios ilimitados. Los 64 KiB de audiencias
no cambian. Consultas desconocidas/duplicadas o mal formadas causan 400.
Detalle, revisión exacta, preparación y escrituras no admiten parámetros de query:
aceptan query ausente o vacía y rechazan cualquier contenido con `invalid_query`.
La selección histórica se expresa exclusivamente en la ruta `/revisions/{revision}`.

## 6. Proyección, autorización histórica y recibo exacto

Detalle: scope e identidad, revisión, values/digest, status/reason, receipt,
anchor, continuation, recorded_administration_revision/digest, recorded_at UTC,
recorded_by {id,email}, asistentes resueltos y soporte admitido opcional.

`anchor`: hearing_id/revision, values_digest/submission_digest, estado/tipo/fecha
programada y scheduling_context original. `continuation`: result_id/hearing_id,
revisión, values_digest/submission_digest y estado EXACTO del antecedente; no su
cabecera actual. No expandir todo el árbol. La administración de captura actual
es independiente del contexto de programación histórico; ninguno suplanta al otro.

Asistentes: ficha/revisión/digest, perfil manual/typed, nombre/rol y estado históricos,
identidad vinculada exacta/digest cuando exista, más capacity/observation declaradas.
Soporte: id/versión/digest/nombre/formato/política capturados. Límites y validadores
existentes, incluido nombre de soporte máximo 128 bytes, siguen aplicándose.

Recibo: `operation_id`, `action`, `expected_revision`, `submission_digest`.
Scope, actor y fuentes se encuentran en el mismo detalle y se incluyen en HRTX1.
Sin contexto esperado administrativo/procesal. Resultado exacto = expected+1.
El recibo no es firma digital ni sello judicial.

Envío incierto: conservar intento preparado en memoria de la sesión y consultar
su revisión objetivo EXACTA; cotejar familia de resultados, case/hearing/result,
revisión, actor, operation_id, acción, expected_revision, valores/fuentes y digest.
No usar la cabecera posterior ni similitud de texto como prueba. 404 puede ser
operación aún en vuelo; conservar incierto. Otro recibo en esa revisión es conflicto.
401/403 limpia datos según política de sesión; jamás repetir escritura automáticamente.

## 7. Puertos y secuencia de aplicación

Los tipos públicos viven en `crates/application/src/hearing_results/`:
`HearingResultStore`, `HearingResultWorkflow`, `HearingResultCommand`,
`HearingResultPreparation`, `PreparedHearingResultChange`, `HearingResultDetail`,
`HearingResultDraft`, `HearingResultQuery` y `HearingResultHistoryQuery`.

Store: `list(actor,case,hearing,query,at)`, `get(actor,case,hearing,id,revision?,at)`,
`history(actor,case,hearing,id,query,at)`, `prepare(actor,case,command,limits)` y
`commit(actor,case,prepared)`. Workflow mismos queries con token, más
`prepare(token,case,command)` y `submit(token,case,command,expected_digest)`.
No puerto de mutación de audiencias ni contexto procesal nuevo. Constructor PG
con hasher y Clock inyectado; servicio con identidad, documentos/validador, hasher,
Clock, análogo a HearingService, sin depender de infraestructura.

Preparation contiene base opcional, administración observada, ancla exacta,
antecedente exacto opcional, snapshots de asistentes y records cifrados del soporte.
El prepared, construido solo por servicio, contiene comando normalizado, valores,
digests, fuentes resueltas, formatos y preparación; sin campos públicos mutables.

1. Resolver identidad/acceso y preparar fuentes autorizadas.
2. Validar valores, tiempo preliminar, soporte PDF/DOCX exacto y evidencia con
   política vigente fuera del lock; calcular HRES1 y HRTX1 con hasher port.
3. Comparar expected_submission_digest; refrescar sesión antes de commit.
4. Abrir transacción común auditada y, después del lock, revalidar actor, rol,
   membresía, caso activo, revisión objetivo y operación global de esta familia.
5. Resolver de nuevo ancla/antecedente/participantes/soporte y comparar snapshots.
   Cambios de CABECERAS históricas no son cambios de las referencias elegidas.
   Administración actual puede diferir; capturar la nueva, sin conflicto artificial.
6. Capturar Clock tras lock y comprobar no futuro en alta/corrección; insertar
   raíz cuando corresponda, revisión y auditoría atómicamente. Fallo implica rollback.

Admisión de soporte: alta/corrección vuelven a admitir el soporte que proponen,
aunque su referencia sea igual; coincide con programación existente. Retiro copia
admisión histórica sin volver a analizar el formato ni validar criptografía antigua.
Lecturas históricas verifican integridad y fuentes, sin afirmar nueva admisión.
La rectificación revalida también el soporte idéntico. La conservación de la
admisión anterior se limita a retiro e historia.

## 8. Canon propio y cotas calculadas

Todos los enteros big endian. UUID16 bytes, digest32. Texto: u32 longitud UTF-8,
bytes normalizados. Opcional: u8 0/1 y valor solo si1. Rechazar tags desconocidos,
longitudes incongruentes, UTF-8 inválido, formas no normalizadas y trailing bytes.
Revisiones u32; tiempo UTC segundos i64 y desfase segundos i32. No canon JSON/JS.

**HRES1**, en este orden:

1. Prefijo ASCII HRES1 (5 bytes).
2. occurrence u8 (occurred0/not_started1), extent u8 (partial0/concluded1/unspecified2).
3. Tiempo: tag0 + año u16 + mes u8 + día u8 + desfase i32 (9 bytes), o tag1 +
   Unix i64 + desfase i32 (13 bytes). Sin nanosegundos en el hecho declarado.
4. summary texto; conteo asistentes u8; por cada uno ordenado por UUID:
   participant UUID16, revisión u32, capacity texto, observation opcional texto.
5. Conteo acuerdos u8; por cada uno en orden declarado: UUID16 + texto.
6. Provenance kind u8 (operator_note0/oral_reference1/written_record2), reference
   opcional texto, support opcional (document UUID16 + versión u32 + digest32).

Máximo **146933 bytes**: 5+1+1+13+4004+1+32*(16+4+404+1+2004)
+1+16*(16+4004)+1+1+804+1+52. Mínimo válido **26 bytes**. Los extremos y las
variantes se fijan con vectores independientes en
`crates/domain/tests/fixtures/hearing_result_vectors.json`, generados por
`generate_hearing_result_vectors.py` en el mismo directorio. Hash SHA256 de estos
bytes; las fuentes fijas se vinculan en el recibo.

**HRTX1**, en este orden:

1. Prefijo ASCII HRTX1; UUID operación, actor, expediente, audiencia y resultado.
2. Acción u8 (record0/correct1/withdraw2), expected_revision u32.
3. Ancla: revisión u32, digest HEAR1 y digest HTXN1 exactos (68 bytes).
4. Continuidad opcional: hearing UUID16, result UUID16, revisión u32, digest HRES1,
   digest HRTX1 de antecedente exacto (100 bytes, más tag opcional).
5. values_digest HRES1; reason opcional texto (solo correct/withdraw, obligatorio).

Máximo **4296 bytes**; mínimo de alta sin continuidad/motivo 192. HRTX1 incluye
las fuentes resueltas también al corregir/retirar. No incluye administración/tiempo
actuales de captura, desconocidos al preparar. Deben comprobarse en persistencia,
auditoría y proyección, como metadatos capturados, sin presentarlos como el envío.

Máximo de textos normalizados en comando de corrección con motivo: 37400 escalares.
En JSON, pares escapados para escalares suplementarios usan hasta 12 bytes cada
uno (~448800 bytes de textos, más estructura); por eso 512 KiB finitos. El límite
de bytes opera antes de parsear. Un objeto sintético máximo de corrección, generado
solo para medir serialización, ocupa 154771 bytes UTF-8 o 453971 con escape ASCII
compacto. No es una prueba del endpoint ni garantiza toda representación JSON.
Los vectores HRTX1 se contrastan en
`crates/application/tests/hearing_result_canonical.rs` y con el decodificador SQL
en `crates/infrastructure/tests/hearing_result_receipt_sql.rs`.
HEAR1/HTXN1 no cambian ni se reinterpretan.

## 9. Persistencia e integridad

Las migraciones `0012_hearing_results*.sql` definen `case_hearing_results` y
`case_hearing_result_revisions`, junto con los decodificadores y triggers.
Raíz con scope, fuentes inmutables y FK a primera revisión diferida; revisiones
append-only, PK(result_id,revision), operación UNIQUE, canon/digests/proyecciones generadas,
actor/admin capturados, soporte admitido y campos de contexto verificables.

Guard de alta bajo lock común: antecedente y su revisión ya existen antes de
insertar raíz; mismo caso; distinta raíz. Referencias opcionales completamente
nulas o completas; FK compuestas y verificación de digests/recibos exactos.
Los triggers prohíben UPDATE/DELETE/TRUNCATE y cambiar raíces. Existencia previa + inmutabilidad
impiden crear ciclos durante escrituras autorizadas; no deducirlo de timestamps.

SQL independiente decodifica HRES1/HRTX1 y comprueba proyecciones, secuencia,
acción/estado/motivo, ancla y continuidad, fuentes de participantes/soporte, actor,
membresía/rol/caso y tiempo declarado <= captura. La columna de captura no equivale
a un reloj externo certificado; el Clock se verifica en el adapter después del lock.
Retiro copia contenido y soporte exactos, conservando la nueva autoría/motivo.
La administración capturada no puede preceder la administración registrada en el
ancla, antecedente o revisión base; si coincide la revisión, también debe coincidir
su digest. Esta comprobación de integridad no introduce un CAS de administración.

Lecturas reconstruyen fuentes exactas y recibos, sin expansión recursiva de
continuaciones; validan referencia inmediata y proyección del antecedente. Inventario
de arranque valida todas las filas/aristas, incluida ausencia de ciclos mediante
recorrido finito con visitados. Las inconsistencias causan rechazo, sin normalizar
silenciosamente una fila corrupta. Las consultas mantienen una transacción de
lectura coherente, autorización y auditoría como en audiencias.

El catálogo comprueba tablas, funciones, triggers y privilegios; la importación en
un destino nuevo exige ambas tablas vacías. El respaldo/restauración incluye sus
filas y fuentes, conservando la evidencia criptográfica existente. Los loaders
transaccionales reutilizan las revisiones exactas de etapas, fichas y documentos.
La selección de asistentes no usa el filtro de fichas activas de programación.

## 10. Errores

- 400: JSON/Content-Type inválidos (`invalid_json`), query inválida
  (`invalid_query`) o acción/identidad/ruta incongruentes
  (`hearing_result_command_mismatch`); 413 `hearing_result_body_too_large` por body.
- 401 sesión; 403 permiso; 404 recurso del scope no accesible/inexistente, usando
  el patrón actual para no revelar existencia en otro expediente.
- 409: `hearing_result_revision_conflict`, `hearing_result_already_withdrawn`,
  `hearing_result_operation_conflict`, `hearing_result_submission_mismatch`,
  `hearing_result_support_changed`; `case_closed` según error común existente.
- 422: `invalid_hearing_result_value`, `invalid_hearing_result_revision`,
  `hearing_result_revision_exhausted`, `hearing_result_future_time`,
  `hearing_result_invalid_reference` (autorref/raíz previa no admisible),
  `hearing_result_support_too_large`, `hearing_result_support_format_rejected`,
  `hearing_result_support_validation_limit`, `hearing_result_support_digest_mismatch`.
- Resultado no encontrado: 404 `hearing_result_not_found`. Ausencia de
  ancla/antecedente/ficha/soporte del scope: 404 `hearing_result_reference_not_found`;
  expediente inaccesible: `case_not_found`. Corrupción almacenada: 500
  `internal_error`, sin filtrar datos. Cambiar una cabecera histórica no provoca
  por sí solo 409.

Los identificadores y revisiones de audiencias, participantes y documentos
conservan sus códigos comunes. La correspondencia está en
`crates/web/src/error/hearing_result.rs` y `crates/web/src/error.rs`; el cliente
interpreta códigos, no el texto del mensaje.

## 11. Implementación y verificación

El dominio y la aplicación residen en `crates/domain/src/hearing_results/` y
`crates/application/src/hearing_results/`. Infraestructura separa adapter,
codec y catálogo/inventario bajo `hearing_result_postgres/`, `hearing_result_codec/`
y `hearing_result_schema/`; HTTP vive en `crates/web/src/hearing_results/`.
La composición conecta estos puertos sin cambiar las reglas de programación.

Las pruebas de dominio `hearing_result_values`, `hearing_result_time` y
`hearing_result_canonical` fijan normalización, cotas, orden, duplicados y precisión.
Las de aplicación cubren preparación, recibos, soporte, retiro y consultas.
Las pruebas de infraestructura requieren PostgreSQL aislado para comprobar
revalidación, carreras, Clock, rollback, referencias, guards, catálogo, inventario,
importación y restauración. Una suite sin esas dependencias no acredita el adapter.

En HTTP, `crates/web/tests/hearing_results_{http,body,queries}.rs` cubren rutas,
query estricta, objetos anidados, tiempos y límite de bytes. Qadra separa raíces,
historia y detalle mediante `HearingResults`, `HearingResultHistory` y
`HearingResultDetail`; sus helpers de API y envío incierto viven en `web/src/lib/`.
La interfaz conserva selección histórica expresa y conciliación sin reenvío
automático; no construye HRES1 ni HRTX1 en JavaScript.

`scripts/api_hearing_results_demo.py`, invocado por `scripts/api-hearings-demo.py`,
ejercita sesiones, corrección, retiro, continuación y fuentes exactas. El guion
`scripts/api-migration-demo.sh` incluye ambas tablas en la comparación de estado
antes/después del respaldo; el recorrido HTTP compara también respuestas exactas.
Los resultados ejecutados, entorno y limitaciones se registran por separado en
[verification-report.md](verification-report.md). La implementación local no
sustituye el cierre de campaña, integración ni validación de los demás objetivos.

## 12. Decisiones adoptadas y límite de cumplimiento

Se admiten anclas históricas programadas o canceladas; el retiro es terminal.
Se adoptan las cotas capacity100/observation500/reference200 y segundos enteros.
La procedencia es común al relato y los acuerdos, con localizador obligatorio
para antecedente oral o escrito. Rectificar revalida el soporte idéntico; retirar
conserva su admisión histórica. La historia utiliza hasta veinte entradas ligeras
y la continuidad solo navega al antecedente exacto, sin árbol ni consulta inversa.

El seguimiento conserva hechos declarados: no acredita celebración, firmeza,
notificación, cálculos, calendario judicial ni alertas. CU-08/RF-07/RF-08/OE-2
permanecen parciales; sus textos y criterios aprobados se conservan.
