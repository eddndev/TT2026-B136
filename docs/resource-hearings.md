# Audiencias propias de recursos

## Estado y alcance

Implementación local en curso, aún no integrada ni desplegada. El dominio,
`ResourceHearingService` y `PostgresResourceHearingStore` preparan, confirman y
recuperan la audiencia con su asociación inicial y origen durable. La migración
`0030_` y el inventario estricto están implementados. La API genérica de
asociaciones incorpora la familia `resource_hearing`. Las consultas propias
y el router de preparación, envío y lectura están implementados localmente,
incluida su composición en `serve`.

La agenda incorpora localmente esta familia bajo `kind=resource_hearing`, con
su identidad y origen verificados y orden independiente de audiencias ordinarias
y plazos. Su contrato está en [agenda](agenda-api.md). Alertas, Qadra y aceptación
integrada de agenda siguen pendientes dentro de esta entrega. Las pruebas
focales anteriores y las nuevas lecturas de aplicación y PostgreSQL indicadas
al final están aprobadas, incluido el rechazo tipado de evidencia persistida
incompleta, las rutas propias y su composición. No se atribuyen aceptación
integrada del recorrido, CI global ni despliegue.

La decisión está en [ADR-0069](adr/0069-resource-hearing-scheduling.md). Este
contrato complementa los [recursos](procedural-resources-api.md) y sus
[asociaciones con actividades](resource-activities-api.md), sin modificar la
programación ordinaria ni el formato `HEAR1`.

## Clasificación declarada y fuente normativa

| Tipo cerrado | Compatibilidad declarada |
| --- | --- |
| `AppealArguments` / `appeal_arguments` | Recurso `Appeal`, modalidad conocida `Written`. |
| `WrittenRevocation` / `written_revocation` | Recurso `Revocation`, modalidad conocida `Written`. |

Los artículos 476–477 contemplan una audiencia de alegatos aclaratorios en
apelación. El artículo 466 distingue la revocación oral en audiencia de la
escrita y contempla citación a audiencia por complejidad en esta última.
Esta clasificación no determina procedencia ni calcula el señalamiento.
Fuente consultada el **3 de octubre de 2026**:
[CNPP oficial, páginas 132–136](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf#page=132),
que declara última reforma DOF **28-11-2025**.

La fecha y la necesidad de audiencia se declaran expresamente con su soporte.
El módulo no elige una regla de cómputo del artículo 476 ni deriva una audiencia
separada de la revocación oral. Compatibilidad entre valores declarados no equivale
a calificación jurídica del caso.

## Valores de programación

`ResourceHearingValues::new` recibe los campos siguientes:

| Campo | Representación y límite |
| --- | --- |
| `kind` | Uno de los dos tipos cerrados anteriores. |
| `scheduled_at` | `HearingTime`: instante de segundos enteros, desfase explícito múltiplo de minuto hasta ±14 h; años local y UTC entre 1 y 9999. Conserva el desfase original; no completa una fecha parcial. |
| `modality` | `InPerson` o `Videoconference`. |
| `venue` | `HearingVenue`: texto no vacío, una línea, hasta 500 escalares Unicode; no es una URL ejecutable. |
| `note` | Nota opcional de hasta 1000 escalares, con saltos de línea admitidos. |
| `participants` | De cero a 32 referencias `HearingParticipantRef`, cada una con identidad UUID y revisión positiva. Se ordenan por UUID y se rechaza repetir una identidad, aun con otra revisión. UUID cero sigue siendo una identidad representable. |
| `scheduling_basis` | Declaración obligatoria `HearingNote` y `HearingSupportRef`: documento, versión exacta y digest. No es certificación judicial. |

Los textos recortan espacios exteriores y rechazan controles; el texto multilineal
normaliza CRLF a LF. Los valores no contienen etapa, autor,
recibo ni estado persistido. Tampoco deciden si el instante ya pasó. La familia
dispone de `ResourceHearingId`, `ResourceHearingOperationId` y
`ResourceHearingRevision` propios, separados de los de audiencia ordinaria. La
revisión comienza en uno, rechaza cero y no desborda al agotarse; su existencia
no incorpora todavía un flujo de reemplazo o cancelación.

## Comando y material del puerto

`ResourceHearingCommand` conserva operación, identidad prevista de audiencia,
identidad prevista de asociación, revisión esperada del recurso, captura exacta
del recurso, acto opcional y valores. La identidad prevista no demuestra que se
haya reservado o creado ninguno de esos objetos.

La captura del recurso contiene identidad, revisión y `capture_digest`. El acto
opcional añade su identidad/revisión, la revisión del recurso que lo contiene y
su huella. La revisión esperada corresponde a la cabeza actual; no sustituye la
revisión histórica elegida.

`ResourceHearingStore::prepare(actor, case, resource, command)` debe autorizar
pertenencia y acceso vigentes al expediente antes de la búsqueda. Devuelve una
de dos alternativas:

- `Ready(ResourceHearingMaterial)`: recurso y acto exactos, cabeza actual del
  recurso resuelta por separado, administración observada y revisiones actuales
  de los participantes seleccionados, limitadas a 32. Los soportes proceden de
  las capturas ya admitidas. No modifica datos de negocio; el adaptador confirma
  el evento auditado `resource_hearing.prepare` antes de devolver el material.
- `Replay(ResourceHearingCreation)`: creación original identificada por su
  captura inmutable y su marcador de origen. La existencia aislada de una
  audiencia o asociación no acredita que proceda de la misma operación.

La recuperación conserva la creación original aunque su asociación ya no esté
vinculada; el adaptador conserva esa independencia mediante el marcador durable.
No reconstruye la evidencia histórica desde cabezas nuevas.

`ResourceHearingStore::commit` recibe un `PreparedResourceHearing`. Su contrato
exige, bajo el bloqueo compartido de auditoría, reautorizar expediente y principal
completo, revalidar caso y recurso activos y las cabezas revisadas del recurso y
participantes, y comparar todo el material preparado. Debe escribir audiencia,
asociación inicial, marcador de origen y auditoría en **una transacción**. Un
conflicto no escribe; una carrera de la misma operación exacta devuelve la
creación original, incluida su fecha de registro.

El adaptador PostgreSQL implementa estas obligaciones: autoriza antes de buscar,
carga referencias exactas y cabezas por separado y vuelve a comparar el material
al confirmar. El resultado de su verificación focal se registra separadamente
en el [informe técnico](verification-report.md); las pruebas con dobles no se
presentan como evidencia de transacciones reales.

## Validación y revisión previa

El servicio autentica y admite sólo Owner o Litigator antes de invocar el puerto;
Paralegal y Client se rechazan sin cargar material. La validación de aplicación:

1. Comprueba expediente, identidad del recurso y recibo de su cabeza.
2. Contrasta administración actual con las capturas históricas, sin retroceso ni
   sustitución de una revisión exacta; rechaza el cierre administrativo registrado.
3. Exige cabeza activa y coincidencia con `expected_resource_revision`.
4. Verifica recibos y referencias del recurso y acto seleccionados. Rechaza
   capturas posteriores a la cabeza o distintas de ella bajo la misma revisión.
5. Exige tipo y modalidad escrita compatibles tanto en la selección del recurso
   como en su cabeza. Desconocimiento u oralidad no se completan automáticamente.
6. Localiza el soporte exacto en el recurso seleccionado o en los soportes del
   acto seleccionado. Una versión coincidente con digest o captura distintos se
   rechaza; no basta que exista un documento en el expediente.
7. Resuelve las proyecciones de participantes del mismo expediente, contrasta
   identidad/revisión y huellas de valores y rechaza fichas archivadas.

El resultado `ResourceHearingDraft` incluye comando, recurso, acto, soporte,
proyecciones de participantes, administración observada, referencia de la cabeza,
autor y `submission_digest`. Después de construirlo, el servicio autentica de
nuevo y compara el `Principal` completo. Un cambio de identidad, correo o rol no
permite devolver la revisión anterior; un rol ahora denegado conserva su rechazo.

No se exige ni se fabrica una etapa ordinaria. La comprobación de una referencia
histórica no la convierte en la cabeza ni demuestra que una autoridad haya
confirmado la declaración del usuario.

## Confirmación y recuperación explícita

`prepare_resource_hearing_change` ordena el material de participantes y construye
`PreparedResourceHearing` después de validar el comando y las fuentes. Sus campos
privados conservan revisión, material y principal; no puede construirse omitiendo
esa preparación. `into_creation(recorded_at)` produce y valida la captura inicial
en memoria. No escribe por sí mismo.

`submit`, cuando recibe material `Ready`, vuelve a preparar el comando y compara
`submission_digest` con la huella confirmada por el llamador. Una diferencia impide invocar `commit`. Si coincide,
reautentica el principal completo, realiza una sola llamada a `commit`, verifica
la creación devuelta y exige que su revisión previa sea exactamente la preparada.
Antes de devolverla vuelve a reautenticar. Un error posterior al commit no prueba
que no hubo escritura; el servicio no inicia un reintento automático.

`ResourceHearingCreation` reúne tres piezas:

- `ResourceHearingDetail`: revisión previa, material histórico exacto, revisión
  inicial de audiencia, instante de registro y `capture_digest`.
- `ResourceHearingOrigin`: expediente, recurso, audiencia, operación, asociación
  y huellas de envío y captura. Debe coincidir completamente con el detalle.
- `ResourceActivityDetail`: vínculo inicial R1 real, construido mediante
  `prepare_activity_change`, con destino `ResourceHearing` y la captura exacta
  de la audiencia. Comparte autor, administración, cabeza observada e instante;
  su operación usa el mismo UUID bajo el tipo de operación de asociaciones.

El detalle de audiencia no contiene la asociación. Su verificador independiente
comprueba sólo el detalle; el de creación completa comprueba además asociación
y origen. Así la validación de una fuente no vuelve recursivamente a su creación.

El verificador reconstruye la revisión desde el material histórico y contrasta
sus huellas. El registro debe ser UTC, representable entre los años 1 y 9999 y no
anterior a las fuentes capturadas. El servicio rechaza además una captura fechada
después de su reloj. La validación histórica no concede autorización de acceso.

Cuando el puerto devuelve `Replay`, el servicio comprueba creación, origen,
expediente, recurso, identidad del autor y comando completo. En `submit` exige
además la huella confirmada. Devuelve la captura original sin llamar a `commit`.
El correo histórico se conserva aunque haya cambiado antes de esta nueva llamada;
el principal actual debe permanecer idéntico durante la llamada y mantener un
rol permitido. Una autoridad perdida impide devolver la evidencia.

El llamador puede iniciar expresamente esa conciliación tras una respuesta
incierta usando la misma operación y contenido. El adaptador reconstruye la
audiencia y el vínculo original R1, aunque la cabeza de la asociación sea una
desvinculación R2. Exige el evento original `resource_hearing.registered`, con
marcador `rhl1`, autor, fecha y compromiso de cadena coincidentes. La mera
existencia de objetos con esos identificadores no acredita recuperación.

## Consultas propias y contrato HTTP

`ResourceHearingReadStore` separa `list` y `get` de las operaciones de escritura.
`ResourceHearingReadService` implementa `ResourceHearingReadWorkflow`: autentica
antes de leer, valida la creación completa y reautentica el principal completo
antes de devolverla. Owner puede consultar cualquier expediente; Litigator y
Paralegal necesitan asignación vigente. Client se rechaza antes de invocar el
puerto. El adaptador repite autorización bajo el bloqueo de auditoría y confirma
`resource_hearing.list` o `resource_hearing.read` antes de entregar datos.

La lectura sigue disponible con el expediente cerrado, el recurso archivado o
la asociación desvinculada. Conserva las fuentes históricas sin exigir que el
autor original mantenga hoy sus permisos. Verifica expediente, recurso, audiencia,
revisión exacta, captura, vínculo inicial y origen. El reloj de lectura debe ser
UTC, no retroceder durante la llamada y no preceder la captura devuelta. Una
respuesta corrupta o una auditoría fallida impiden devolver la evidencia.

La base de rutas es
`/api/v1/cases/{case}/procedural-resources/{resource}/activities/resource-hearings`:

| Método y sufijo | Contrato |
| --- | --- |
| `POST /prepare` | Comando explícito; devuelve revisión previa con `submission_digest`, sin crear audiencia. |
| `POST /submit` | `{command, expected_submission_digest}`; devuelve 201 con creación completa o repetición exacta. |
| `GET` base | Página de creaciones históricas del recurso. |
| `GET /{hearing}` | Creación de la cabeza almacenada. |
| `GET /{hearing}/revisions/{revision}` | Revisión exacta; otra revisión positiva ausente no se sustituye por la inicial. |

El comando contiene `case_id`, `resource_id`, `operation_id`, `hearing_id`,
`association_id`, `expected_resource_revision`, `resource`, `act` y `values`.
`act` es obligatorio y puede ser nulo. Los padres deben coincidir con la URL;
el servidor resuelve las fuentes, no acepta material capturado aportado por el
cliente. JSON estricto y acotado a 64 KiB, UUID canónico y huellas hexadecimales
minúsculas. Las rutas requieren bearer y sus respuestas son `no-store`.

La consulta acepta `limit` entre 1 y 20, predeterminado 10, y `after_id`
exclusivo. Ordena por UUID ascendente, sin prometer cronología ni filtrar por
la cabeza del vínculo. Devuelve `case_id`, `resource_id`, `items`, `has_more`
y `next_after_id`. Sólo una página llena puede declarar continuación, y ésta
es el último UUID devuelto; una página vacía no tiene continuación. Cada elemento
contiene `hearing`, `association`, `origin` y `submission_digest`. La asociación
es siempre la inicial; para su estado e historia actuales se utiliza la API
de asociaciones. No se añade `checked_at` ni se afirma vigencia operativa.

No existe ruta adicional de conciliación. Tras una respuesta incierta el llamador
puede repetir **expresamente** `/submit` con el mismo comando y huella. Si ya hay
origen devuelve la creación original; si no hubo commit, el envío puede crear
tras comprobar los requisitos actuales. GET nunca inicia esa escritura y su
resultado no sustituye el cotejo del comando incierto. Tampoco se añaden todavía
reemplazo, cancelación o una historia ficticia de revisiones de audiencia.

Los errores distinguen sesión ausente o vencida (401), permiso denegado (403),
expediente o actividad no encontrados (404), conflicto de la operación (409),
entrada inválida (400), cuerpo excesivo (413) y presupuesto agotado (503).
Una captura existente con origen o asociación inicial incompletos es un error
de integridad almacenada (500 opaco), nunca prueba de ausencia ni permiso
para repetir una creación distinta. Las consultas no sustituyen datos dañados
por una página vacía.

El router usa el presupuesto HTTP compartido. La composición de servidor
comparte el mismo adaptador entre los dos servicios y conserva sus propietarios
fuera del runtime asíncrono; conectar estas rutas no activa agenda ni alertas.

## Persistencia y restauración

La familia `0030_` crea `case_resource_hearings` y
`case_resource_hearing_revisions`, amplía las referencias de asociaciones y
actualiza sus guardas. Conserva valores JSON junto a `RHEAR1` y los bytes
`RHPR1`/`RHCR1` con sus digests; los lectores reconstruyen las fuentes históricas
desde sus tablas exactas. Sólo se admite la revisión inicial de audiencia.

La confirmación escribe captura, asociación y eventos de auditoría bajo el mismo
bloqueo y transacción. La apertura comprueba catálogo, privilegios e inventario
en lotes acotados, sin reparar datos. Verifica ambas direcciones: cada captura
debe tener asociación y origen válidos, y cada origen registrado debe reconstruir
su captura completa. Una restauración parcial, un origen huérfano o una captura
sin origen se rechazan. El rol runtime recibe lectura e inserción por columnas,
sin actualización, borrado, truncado ni facultad de alterar guardas. Véase el
[procedimiento de base de datos](database-operations.md#audiencias-propias-de-recursos).

## Representaciones canónicas

Todos los enteros usan big-endian. UUID y SHA-256 se conservan como bytes crudos.
`RHEAR1` codifica, en este orden: prefijo, tipo u8, segundos Unix i64, desfase i32,
modalidad u8, lugar, presencia de nota y nota si existe, número de participantes
u8 y pares UUID/revisión u32, declaración de señalamiento y soporte como UUID,
versión u32 y digest de 32 bytes. Los textos son UTF-8 con longitud de bytes u32.
Tipos y modalidades usan etiquetas 0/1 según el orden de las tablas anteriores;
la presencia usa 0/1. La lista ya está normalizada por el constructor.

`RHPR1` es la representación interna de la revisión preparada. Vincula expediente,
operación, audiencia, asociación, actor/correo, revisión esperada, captura elegida
y cabeza observada, acto opcional, `RHEAR1`, administración y proyecciones exactas
del soporte y de cada participante. Los bloques variables usan longitud u64.
Soporte y participantes reutilizan la validación y representación `PFSRC1`, sin
ampliar los cupos de fuentes de hechos para acomodar el cupo de esta audiencia.
La implementación está en
[`canonical.rs`](../crates/application/src/resource_hearings/canonical.rs).

`submission_digest` identifica la revisión que el llamador confirma. Por sí solo
no es un recibo durable ni reserva identificadores o un horario.

`RHCR1` vincula esa huella `RHPR1`, la revisión inicial y el instante de registro
con la procedencia histórica de cada participante. Los bloques se ordenan por
UUID y revisión, y cada uno incluye ambos campos para impedir intercambiar
procedencias entre fichas. Distingue ficha manual o tipificada e incorpora los datos de autoría y tiempo no proyectados en la revisión
previa; para fichas tipificadas incluye el envío y la referencia de origen de
credencial, cuando existe, y para sujetos vinculados su autoría y fecha. Esta
vinculación no certifica validez jurídica de la credencial. El detalle conserva
el material necesario para reconstruir y comprobar la revisión. La implementación
del recibo y del marcador de origen está en
[`receipt.rs`](../crates/application/src/resource_hearings/receipt.rs).

La extensión de asociaciones añade la etiqueta 2 de `RASL1` para
`ResourceHearing {id, revision, capture_digest}`. Las etiquetas 0/1 y los bytes
anteriores permanecen intactos. Las funciones públicas
`resource_hearing_submission_bytes(hasher, draft)` y
`resource_hearing_capture_bytes(detail)` exponen los mismos bytes `RHPR1` y
`RHCR1`; no cambian sus formatos. La persistencia los contrasta al reconstruir
el detalle. No se añade una API de importación de canon binario.

## Comprobación y trabajo siguiente

Los casos de [dominio](../crates/domain/tests/resource_hearing_values.rs) incluyen
catálogo cerrado, compatibilidad, cupo y duplicados, vector independiente y
variación de campos canónicos. Las pruebas del
[servicio](../crates/application/tests/resource_hearing_service.rs) usan puertos
controlados para comparar fuentes, soporte, cabeza, permisos y reautenticación.
Los casos de [confirmación](../crates/application/tests/resource_hearing_submission.rs)
contrastan huella, captura completa, origen, reloj y cambios de principal. Los de
[recuperación](../crates/application/tests/resource_hearing_recovery.rs) simulan una
respuesta perdida y una conciliación explícita sin segunda escritura, preservan
el autor histórico y rechazan intentos distintos o autoridad perdida. La
regresión de procedencia altera autoría y orden de participantes sin permitir
que el intercambio conserve una captura válida.
La extensión tiene seis pruebas de dominio de asociaciones, 28 de aplicación
de audiencias de recurso y cinco de consulta inversa aprobadas. El esquema
aprobó cuatro casos en PostgreSQL 16.15 en 19,66 s. El adaptador aprobó nueve
casos en 61,47 s, incluidos rollback, concurrencia real con dos conexiones,
conciliación tras desvinculación, cambios de cabeza de participante y rechazo
de origen huérfano e inventario parcial simulado. No se presenta ese último
caso como una campaña completa de `pg_dump`/`pg_restore`.

El DTO HTTP aprobó cinco casos nuevos, junto con dos de cuerpos y ocho del
contrato existente. Estos resultados no acreditan navegador ni aceptación
integrada. El [informe técnico](verification-report.md) conserva el registro de
cada ejecución, incluidos sus tiempos de compilación.

Las nuevas [lecturas de aplicación](../crates/application/tests/resource_hearing_reads.rs)
aprobaron sus 13 casos en 0,07 s: límites y continuidad, permisos, alcance exacto,
validación de captura y origen, errores de auditoría, reloj y reautenticación.
Las consultas PostgreSQL aprobaron cinco casos en PostgreSQL 16.15 en 38,80 s,
incluidos paginación, permisos, revocación, cierre, archivo, desvinculación,
fallo de auditoría y rechazo de origen perdido. El caso de origen perdido se
refuerza para exigir `StoredInconsistent` tanto en lista como en detalle,
también cuando falta la asociación inicial: rechazar la consulta con cualquier
error no acredita esa clasificación. La comprobación reforzada aprobó 1/1
en 5,66 s, después de reproducir el error. HTTP aprobó 10/10 en 0,04 s y
composición 6/6 en 0,35 s, incluyendo cinco regresiones existentes.

Los siguientes pasos son conectar agenda, alertas y Qadra y cerrar la aceptación
integrada de este recorrido. La asociación inicial ya pertenece a la creación
transaccional; el flujo genérico sólo vincula o desvincula audiencias existentes,
sin crearlas. El soporte sólo puede ser
uno ya admitido en el recurso o acto seleccionado: no se incorpora una citación
nueva y arbitraria mediante esta confirmación.
