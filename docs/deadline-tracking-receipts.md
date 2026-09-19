# Recibos y observaciones para seguimiento de plazos

Estado: el modelo distingue evidencia V1/V2, observaciones verificadas y
revisiones sucesoras. La persistencia PostgreSQL admite las decisiones humanas
V2 y reconstruye sus capturas históricas; incluye actualización compatible
desde V1. El [consumidor local](deadline-worker.md) usa la preparación técnica
y persiste sus resultados e intentos. Su composición desde `serve`, API humana
V2 e integración en Qadra siguen pendientes. La decisión está en
[ADR 0037](adr/0037-durable-deadline-reevaluation.md).

## Versiones y estado operativo

`DeadlineReceiptVersion::Legacy` conserva DLTX1 y requiere ausencia de captura
de seguimiento. `Tracked` exige esa captura y añade la huella de observaciones,
las huellas del predecesor y la causa de DLTX2. El autor es una variante
explícita humana o técnica. La verificación rechaza las combinaciones
incompatibles; un recibo V2 corrupto nunca se reintenta como V1.

Cada revisión conserva una sola `DeadlineCalculation`. La captura de
seguimiento contiene políticas de perfil, fuente y calendario, estado y
motivos de revisión, observaciones y administración examinada. Observar una
cabeza nueva no sustituye automáticamente la evaluación histórica.

El vencimiento operativo sólo expone el instante histórico si el plazo está
activo y su revisión está `Accepted`. `Pending`, `LegacyUndeclared` y el retiro
del plazo producen ausencia de fecha operativa. Aceptar no fabrica un instante
si el cálculo está bloqueado. Sin captura de seguimiento, una revisión V1
conserva `LegacyUndeclared`; no se infiere aceptación del hecho de tener fecha.

## Convenciones

En DLTX2 y DLOB1 los enteros son unsigned y big-endian. Un UUID ocupa sus
16 bytes originales; el UUID nil es una identidad presente válida. Una huella
ocupa 32 bytes.
Las opciones se representan con `0` para ausencia o `1` seguido del valor.
Cualquier otro discriminante se rechaza. Los textos llevan longitud en bytes
`u64` y contenido UTF-8. El decodificador exige consumo exacto del marco:
rechaza truncados, bytes sobrantes, versiones desconocidas y tamaños excesivos.

Los textos no se corrigen durante la lectura. El correo capturado debe tener
entre 1 y 320 escalares Unicode, sin controles ni espacios en sus extremos.
El motivo conserva la forma canónica de `FactText`: entre 1 y 1000 escalares,
sin espacios extremos ni controles salvo LF. CR y CRLF sin normalizar se
rechazan. El máximo UTF-8 de cada campo es 1280 y 4000 bytes respectivamente.
Estos límites permiten conservar una identidad histórica; no validan entrega
postal ni prueban autorización actual.

## DLTX2: operación y autoría

| Campo | Representación |
| --- | --- |
| Versión | Cinco bytes ASCII `DLTX2` |
| Expediente, plazo, operación | Tres UUID, en ese orden |
| Acción | `u8`: registro 0, corrección 1, atención 2, retiro 3, reevaluación 4 |
| Revisión esperada | `u32` |
| Estado revisado | Huella de 32 bytes |
| Observaciones | Huella de 32 bytes del marco DLOB1 completo |
| Predecesor | Opción de huella de operación y huella de captura, en ese orden |
| Autor | Variante descrita abajo |
| Motivo | Opción de texto |
| Causa | Variante descrita abajo |

El autor humano usa `0`, UUID de cuenta y correo capturado. El técnico usa
`1`, identificador de servicio `u8` y versión de política `u16`. El servicio
admitido es `DeadlineReevaluator` (0), con versión 1. Nunca se representa como
una cuenta ficticia ni como el usuario que publicó la fuente.

La causa es una de las siguientes:

- `0`: decisión humana. No contiene campos adicionales.
- `1`: evento de fuente. UUID de trabajo, secuencia `u64`, familia `u8`, UUID
  de fuente, revisión `u32`, expediente opcional, audiencia opcional y UUID
  de operación de origen.
- `2`: conciliación inicial del legado. UUID de trabajo y versión de política
  `u16` igual a 1. No inventa un evento de fuente.

La secuencia debe estar entre 1 e `i64::MAX`, compatible con su origen
PostgreSQL. La revisión de fuente debe ser positiva. Las familias son
resolución 0, notificación 1, resultado de audiencia 2, calendario 3 y perfil 4.
Resolución y notificación exigen el expediente del plazo y no llevan audiencia.
Un resultado exige ese expediente y una audiencia. Un calendario es global.
Un perfil puede ser global o pertenecer al expediente. Ninguna otra familia
admite un identificador de audiencia.

Un registro exige revisión esperada cero, ausencia de predecesor y motivo,
autor humano y causa humana. Las otras operaciones exigen revisión esperada
positiva menor que `u32::MAX`, las dos huellas del predecesor y motivo.
Corrección, atención y retiro son humanos; reevaluación exige autor técnico y
una causa técnica. Estas reglas de forma no prueban que el predecesor exista
ni que sea la cabeza vigente: esas comprobaciones pertenecen al verificador
del sucesor y a la transacción.

El marco válido mínimo ocupa **151 bytes**. El máximo es **5502 bytes** y
corresponde a una operación humana con correo y motivo del máximo tamaño
UTF-8 permitido. DLTX2 no reemplaza ni reinterpreta los bytes de DLTX1. Un
marco corrupto de la nueva versión no debe verificarse con la versión anterior.

## DLOB1: observaciones exactas

El marco comienza con `DLOB1`, UUID de expediente y número de entradas `u8`
entre 1 y 4. Cada entrada contiene:

| Campo | Representación |
| --- | --- |
| Función de la referencia | `u8`: perfil 0, fuente 1, calendario 2, padre de notificación 3 |
| Familia | Discriminante compartido con DLTX2 |
| Identidad y revisión | UUID y `u32` positivo |
| Expediente | Opción de UUID |
| Audiencia | Opción de UUID |
| Padre histórico | Opción de UUID de resolución y `u32` positivo |
| Recibo de origen | Huella de operación de origen |
| Evidencia completa | Huella de la representación histórica reconstruida |

Las funciones son únicas y están en orden estricto; el codec no ordena ni
elimina duplicados. El perfil es obligatorio. Las funciones perfil y calendario
sólo admiten su familia correspondiente; fuente admite resolución, notificación
o resultado. La función de padre sólo admite resolución. Rigen las mismas
reglas de ámbito que para los eventos.

Únicamente una notificación conserva padre histórico obligatorio. La entrada
separada de padre observado, cuando existe, debe identificar esa misma raíz
con revisión igual o posterior. Una notificación R2 puede conservar su padre
R1 y observar la cabeza R9: observar R9 no altera el acto vinculado a R1.
Una referencia a audiencia sólo pertenece a un resultado de audiencia.

El codec permite omitir la cabeza del padre para representar el material
legado; eso no acredita aceptación de una captura nueva. El verificador del
estado completo exige esa observación para una notificación aceptada.
Tampoco una revisión observada decide si se sigue la cabeza o se conserva una
selección fija: esa política se declara aparte.

El máximo válido es **446 bytes**: perfil privado, notificación, calendario
y padre observado. Un manifiesto con perfil global como única entrada ocupa
**111 bytes**.

## DLOE1: evidencia de cada observación

`evidence_digest` compromete `DLOE1`, el discriminante de familia `u8` y la
representación de evidencia correspondiente. Los valores o definiciones se
comprometen mediante sus huellas verificadas; también se incluyen estado,
motivo, operación, autor, correo y momento históricos. En fuentes se conserva
la evidencia administrativa; un resultado de audiencia incluye sus anclas,
continuación, contexto, participantes y soporte capturados. No basta copiar
la huella de operación de la fuente.

Los detalles exactos de esta composición están en
[`entries.rs`](../crates/application/src/deadline_observations/entries.rs),
[`evidence.rs`](../crates/application/src/deadlines/evidence.rs) y
[`evidence_hearing.rs`](../crates/application/src/deadlines/evidence_hearing.rs).
Las fuentes retienen su discriminante interior 1/2/3 después de la familia
DLOE1 0/1/2. Los instantes de esta evidencia conservan nanosegundos Unix
`i128` y desplazamiento en segundos `i32`, ambos big-endian; comparar sólo
instantes UTC perdería parte de la captura.

`build_deadline_observations` verifica recibos de perfil, fuente exacta y
cabeza, calendario exacto y cabeza, y padre de notificación. Comprueba ámbito,
identidad, presencia por pares y orden de revisiones. Si selección y cabeza
tienen la misma revisión, exige igualdad de evidencia, incluidos los
desplazamientos temporales. Para notificaciones exige una resolución padre
completa y compatible con la selección y con la cabeza; no la reconstruye a
partir de una proyección parcial. Construir observaciones permite registrar
dependencias retiradas y expedientes cerrados, sin aceptar su aplicabilidad.

`build_legacy_deadline_observations` tiene un alcance distinto: verifica V1 y
reconstruye únicamente el perfil seleccionado y las cabezas de fuente y
calendario ya comprometidas. No consulta cabezas actuales, no añade un padre
de notificación que V1 nunca capturó y no declara políticas ni aceptación.
La causa `LegacyBootstrap` tampoco inventa un evento de fuente; su sucesor
técnico conserva el cálculo anterior y deja revisión pendiente con políticas
sin determinar.

`verify_captured_deadline_observations` contrasta un manifiesto persistido con
el perfil, material y padre observado reconstruidos desde revisiones exactas.
Conserva la ausencia histórica legítima de la observación separada del padre;
no la completa con su cabeza actual ni declara aceptación. Verifica el marco,
los recibos y la evidencia completa antes de comparar las observaciones.
El constructor de observaciones nuevas mantiene su exigencia estricta de padre
completo para notificaciones.

La transición técnica V1 a V2 usa ese manifiesto reconstruido como referencia,
tanto en el preparador como en el verificador del sucesor. La falta de un
campo de seguimiento V1 no equivale a ausencia de observaciones. Se conservan
las identidades y el orden de revisiones ya capturados; una revisión igual
exige la misma evidencia completa. Un evento anterior o igual a la cabeza
histórica no justifica otra revisión técnica aunque la cabeza actual haya
avanzado. El bootstrap tampoco permite retroceder una cabeza ni sustituir su
evidencia, incluido el desplazamiento temporal.

## DLRV2 y DLST2: estado revisado y captura

Los codificadores de estado conservan la composición histórica V1 y emplean
el prefijo V2 cuando existe seguimiento. El verificador exige que esa presencia
concuerde con `DeadlineReceiptVersion`. La parte histórica compromete título,
perfil seleccionado, declaraciones de entrada, evidencia exacta del cálculo,
resultado con traza, responsable, atención y estado del plazo. Los bytes y
verificadores V1 permanecen independientes.

Después del estado histórico se añade, en este orden:

| Campo | Representación |
| --- | --- |
| Políticas | Tres `u8`, en orden perfil, fuente, calendario: sin determinar 0, fija 1, seguir 2 |
| Estado de revisión | `u8`: legado sin declarar 0, aceptado 1, pendiente 2 |
| Motivos de revisión | Cantidad `u8`, seguida de pares dependencia/motivo `u8` |
| Observaciones | Huella del DLOB1 completo |

Las dependencias de los motivos son perfil 0, fuente 1 y calendario 2. Sus
motivos son cambio de fuente 0, cambio de perfil 1, dependencia retirada 2 y
política sin determinar 3. Se admiten hasta ocho motivos, únicos y ordenados
estrictamente por dependencia y motivo; no se ordenan ni completan al leer.
`Pending` exige motivos; `Accepted` y `LegacyUndeclared` no los admiten.
Los cambios de fuente/perfil corresponden sólo a su dependencia y a `Follow`.
Una dependencia ausente mantiene política sin determinar y no aporta motivos.
Una dependencia presente aceptada exige política explícita; si está pendiente
y sin determinar, exige su motivo `PolicyUndetermined`, incluso cuando también
está retirada.

DLST2 añade la revisión administrativa observada opcional, la huella CADM1 de
sus valores y la huella de su evidencia administrativa completa. DLRV2 omite
esta observación administrativa, como DLRV1; la administración histórica
interna de una fuente sigue comprometida en ambos. DLST2 conserva además la
administración del cálculo histórico: la observación nueva no la reemplaza.

La observación nunca puede ser anterior a la selección. En un estado aceptado,
`Follow` exige selección y observación iguales; `Fixed` permite conservar una
revisión anterior de esa misma identidad. Esto no permite aceptar un retiro.

## Preparación humana y verificación del sucesor

[`prepare_tracked_deadline_change`](../crates/application/src/deadlines/preparation_tracked.rs)
prepara las cuatro acciones humanas con autor humano explícito. Alta y
corrección requieren políticas y construyen observaciones completas con
aceptación expresa. Atención y retiro no reciben políticas nuevas: conservan
el seguimiento anterior, incluida una revisión pendiente. Sobre V1 reconstruyen
sólo las observaciones legadas y mantienen `LegacyUndeclared`. El recibo V2
vincula ambas huellas del predecesor cuando existe.

La preparación exige perfil seleccionado y cabeza publicados. Una política
de perfil fija permite seleccionar una revisión anterior publicada, pero no
evita comprobar la cabeza. Para una nueva calificación también se rechazan
fuentes, calendarios y padres observados retirados. **`Fixed` no acepta
implícitamente un retiro**: la conciliación excepcional de esa situación sigue
pendiente de un contrato explícito. La autorización y la resolución de cabezas
actuales corresponden al servicio y deben repetirse al confirmar en almacenamiento.

[`deadline_successor_matches`](../crates/application/src/deadlines/successor.rs)
verifica los recibos adyacentes, identidad de expediente/plazo, revisión
consecutiva, operación distinta y ambas huellas del predecesor V2. Rechaza
sucesores de un plazo retirado y el regreso de V2 a V1. La corrección conserva
la atención; atención y retiro conservan la evidencia del cálculo y el
seguimiento. Convertir un seguimiento manual V1 a V2 no equivale a aceptarlo.

La administración observada tampoco retrocede. Para V2 se compara con la
observación del seguimiento anterior; al pasar desde V1, con la administración
del cálculo histórico. Puede pasar de la base original sin revisión a una
revisión registrada, pero nunca volver a la base ni reducir la revisión.
Conservar una revisión exige la misma captura completa, incluidos autor y
desplazamiento temporal; dos bases originales también deben coincidir.
Este control se añade a las restricciones de cada acción: no autoriza que
atención o retiro cambien la administración que deben preservar.

Una reevaluación técnica no cambia políticas, borra motivos pendientes ni
eleva una revisión no aceptada a aceptada. Sus observaciones no retroceden ni
cambian identidad; una revisión observada igual conserva la misma evidencia.
Cada observación de perfil o fuente que avanza con `Follow` exige estado
`Pending` y un motivo de su ámbito: `ProfileChanged` para perfil y
`SourceChanged` para fuente. También se admite `DependencyRetired` del mismo
ámbito. La cabeza del padre de notificación pertenece al ámbito fuente y usa
su política; incorporarla por primera vez también exige esa revisión si la
fuente tiene `Follow`. El control considera todas las dependencias avanzadas,
aunque sólo una haya originado el evento, y conserva los motivos anteriores.

DLOB1 no declara el estado de retiro: el adaptador debe verificarlo con la
evidencia exacta; un motivo por sí solo no demuestra ese estado. `Fixed` no
genera un motivo por un cambio ordinario, y las políticas sin determinar
mantienen su exigencia de `PolicyUndetermined`. El calendario seguido conserva
su regla específica de recálculo; no utiliza `SourceChanged` ni `ProfileChanged`.

El evento puede ser anterior a la cabeza observada durante su preparación.
Para una dependencia ya observada, una revisión técnica nueva exige
`observada_anterior < evento <= cabeza_nueva`, con identidad y ámbito iguales.
La causa conserva la revisión y operación originales del evento; no se
reescribe para aparentar que el evento corresponde a la cabeza nueva. Si ambas
revisiones coinciden, el preparador exige que la operación del evento coincida
con el recibo de esa cabeza exacta. Para un evento anterior, su operación se
debe comprobar contra la revisión durable correspondiente, no contra el recibo
de la cabeza posterior.

El cambio de cálculo permitido es el de un calendario seguido y publicado,
con estados anterior y nuevo aceptados y selección/cabeza exactas. Puede
observar a la vez avances ordinarios de fuentes o perfiles `Fixed`, conservando
sus selecciones y calificaciones históricas, incluso si uno de esos avances
originó el trabajo. Las demás observaciones seguidas no cambian en esa
excepción. Si avanza una fuente o perfil `Follow`, queda `Pending` y se conserva
el cálculo anterior, aunque también haya avanzado el calendario. Un calendario
`Fixed` conserva su selección y resultado al observar una revisión ordinaria.
El verificador compara capturas canónicas y no repite la aritmética histórica.

## Preparación técnica y resultados sin cambio

[`prepare_technical_deadline_change`](../crates/application/src/deadline_technical/mod.rs)
recibe una base exacta, un identificador de operación y causa técnica, junto
con la cabeza del perfil, el material seleccionado con cabezas observadas y
la cabeza completa del padre cuando corresponde. Verifica la base, la forma
de la causa, la conservación del material seleccionado y las observaciones
exactas antes de producir una revisión. Conserva políticas, responsable,
atención y calificación humana; las nuevas causas de revisión se combinan con
las anteriores en orden canónico, sin borrarlas ni aceptar por el usuario.

El resultado distingue una revisión preparada de cuatro motivos tipificados
sin cambio:

| Variante `NoChange` | Significado |
| --- | --- |
| `Retired` | La base verificada ya es un plazo retirado. |
| `AlreadyInitialized` | Se solicitó `LegacyBootstrap` y la base ya no está en `LegacyUndeclared`. |
| `DependencyNotSelected` | La identidad y el ámbito del evento no corresponden a una dependencia observada del plazo. |
| `AlreadyObserved` | La revisión del evento es igual o anterior a la ya observada. |

`Retired` y `AlreadyInitialized` se deciden después de verificar el recibo de
la base y la forma de la causa, antes de examinar las cabezas nuevas. No
necesitan leer ni validar cabezas irrelevantes para esas salidas. Las rutas
que examinan dependencias comprueban primero la administración exacta contra
el seguimiento anterior o, en V1, contra la administración del cálculo.
Rechazan un retroceso de revisión o una sustitución de evidencia de la misma
revisión, incluidos autor, momento y desplazamiento temporal. Esta comprobación
precede a `AlreadyObserved` y `DependencyNotSelected`: ninguno puede ocultar
una administración incompatible en los insumos recibidos.

Una revisión preparada conserva la base y los insumos examinados para su
revalidación. Captura observaciones, políticas y motivos en V2, enlaza ambas
huellas del predecesor y registra autor técnico `DeadlineReevaluator`, política
1. Sólo ejecuta una evaluación nueva cuando procede el calendario seguido;
en ese caso conserva el perfil, fuente y calificación históricos. Antes de
devolverla verifica recibo y continuidad. `record(recorded_at)` construye el
snapshot con el momento elegido por la persistencia; no confirma una escritura.

También un resultado `NoChange` requiere comprobar transaccionalmente que la
base y las cabezas pertinentes siguen siendo las examinadas antes de dar por
concluido el trabajo. Para `Retired` y `AlreadyInitialized`, la comprobación de
continuidad se centra en la base que justifica la salida, sin exigir cabezas
que no participaron en esa decisión. El adaptador debe autenticar servicio y
evento durable, resolver la operación exacta de un evento anterior a la cabeza
y repetir la preparación si cambian sus insumos. El preparador puro no reserva revisiones,
no marca trabajos como terminados ni prueba por sí solo que un evento exista.

## Persistencia y fronteras pendientes

Las migraciones `0018_` guardan por separado `tracking_canonical`, el sufijo
existente de DLST2, y `observations_canonical`, el marco DLOB1. Las proyecciones
generadas enlazan administración observada y secuencia de causa. El cálculo
anterior mantiene sus columnas y bytes. Los campos de usuario admiten NULL
para representar autoría técnica sin cuentas ficticias. El puerto humano
rechaza esa autoría; la ampliación `0020_` permite únicamente la transición
correspondiente a un trabajo durable auténtico y exige su resultado al confirmar.

El commit humano reautoriza cuenta, expediente y responsable cuando corresponde,
vuelve a preparar las cabezas y confirma revisión y auditoría conjuntamente.
Registro y corrección requieren seguimiento aceptado con observaciones vigentes;
`Fixed` permite conservar una selección histórica publicada. Atención y retiro
preservan la captura. Un upgrade manual V1 conserva exactamente sus observaciones
históricas y políticas no declaradas.

Los parsers SQL comprueban marcos, proyecciones y compromisos. El guard contrasta
referencias y recibos de cada revisión; el lector reconstruye también DLOE1 y
la evidencia administrativa completa desde revisiones exactas. La coincidencia
de hashes por sí sola no autentica ese material. La lectura y el inventario
verifican continuidad y rechazan referencias inexistentes o evidencia alterada.

El despachador persiste trabajos y cursores mediante `0019_`; el consumidor
local autentica servicio técnico, evento y trabajo mediante `0020_`. Confirma
revisión, resultado y auditoría atómicamente, o conserva un motivo sin cambios
con base y observaciones exactas. Sus intentos fallidos tienen historia separada;
no se consideran resultados satisfactorios. Véase [el contrato](deadline-worker.md).

El servicio humano y HTTP actuales mantienen V1; falta evolucionar su contrato
junto con Qadra para exponer políticas, motivos, causa y autoría y componer ambos
adaptadores en `serve`. Agenda y alertas deben consumir solamente vencimientos
operativos admitidos. La verificación local del consumidor aprobó; sus resultados no acreditan
esa integración operativa todavía pendiente.

La evidencia ejecutada y sus límites están en
[el informe de verificación](verification-report.md). El almacenamiento humano
no acredita procesamiento automático ni entrega de avisos.
