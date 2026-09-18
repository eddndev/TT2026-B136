# Registro y atención de plazos

Estado: modelo, aplicación, persistencia PostgreSQL y HTTP implementados. Están
aprobadas las pruebas focales de esquema, transacciones, concurrencia y
revalidación, la restauración PostgreSQL y el ensayo integrado de API con
servicios reales. Qadra de plazos sigue pendiente: no existe todavía un flujo de
alta de plazos en la interfaz. El cierre global de esta entrega se registra por
separado en el [informe de verificación](verification-report.md). La decisión está en
[ADR-0036](adr/0036-persisted-deadline-evaluation-and-attention.md).

## Modelo

Una identidad de plazo conserva su expediente. La definición contiene título,
perfil y revisión exactos, selección temporal, calendario opcional, cantidad
ordenada opcional, calificación expresa y responsable. El cálculo conserva el
perfil completo, material exacto, cabezas observadas y resultado con su traza.

| Operación | Resultado |
| --- | --- |
| Register | R1 activa, evaluación completa o bloqueada, atención pendiente. |
| Correct | Nueva evaluación explícita; conserva la atención anterior. |
| SetAttention | Nueva declaración de actuación o regreso explícito a pendiente; conserva el cálculo anterior. |
| Retire | Retiro terminal; conserva cálculo, responsable y atención. |

Las operaciones posteriores exigen revisión esperada y motivo. El retiro no
elimina evidencia. La atención conserva fecha, minuto, segundo o desconocimiento
según lo declarado; no fabrica una hora de presentación. El instante de
vencimiento, cuando los insumos permiten obtenerlo, procede del cálculo y queda
capturado. Determinar si ya transcurrió comparándolo con el reloj es una cuestión
separada de la atención y del retiro; esa clasificación no se persiste como un
estado adicional en este corte.

La selección de un perfil retirado o de una revisión anterior a su cabeza impide
un cálculo nuevo. Sus capturas anteriores siguen disponibles para consulta,
atención y retiro. Las fuentes procesales históricas pueden seleccionarse
expresamente; su diferencia frente a la cabeza observada se conserva. Un estado
organizativo de retiro no determina por sí solo su efecto jurídico.

## Capturas y verificación

DEVI1 conserva entradas antes de cualquier decisión dependiente del perfil. Su
lector exige forma canónica, identificadores de condición distintos, de cero a
16 respuestas y tamaño entre 48 y 98 897 bytes. La cantidad ausente no se
sustituye por el máximo ni por una regla ficticia.

DRES1 conserva requisito y resultado de extracción, regla opcional, aritmética,
traza completa, bloqueos ordenados y vencimiento opcional. Tiene límite de
3 000 000 bytes. Las trazas de calendario conservan fecha, origen semanal o
excepción, clasificación, explicación, referencias y acumulado. Leer una captura
no ejecuta nuevamente el cómputo ni reclasifica fechas con otro calendario.

DLRV1 y DLST1 vinculan los valores mediante sus huellas canónicas e incluyen
literalmente los metadatos históricos que los recibos previos no cubren. Esto incluye administración de las
fuentes, nombres de asistentes y soportes de audiencia. La administración
observada del expediente se agrega solamente a DLST1: un avance administrativo
activo no altera lo confirmado, mientras que cierre y retroceso se rechazan.

DLTX1 usa el prefijo ASCII, cuatro UUID de 16 bytes en orden actor, expediente,
plazo y operación, acción de un byte, base U32BE, huella revisada de 32 bytes y
presencia del motivo. Un motivo presente usa longitud UTF-8 U64BE y sus bytes.
Las acciones Register, Correct, SetAttention y Retire usan respectivamente
0, 1, 2 y 3. UUID cero es un valor explícito. El recibo va de 107 a 4115 bytes.

## Persistencia y frontera transaccional

El [adaptador PostgreSQL](../crates/infrastructure/src/deadline_postgres/mod.rs)
comprueba autorización actual incluso para una consulta vacía. Owner administra
todos los expedientes; Litigator administra los asignados; Paralegal consulta los
asignados; Client está denegado. Las lecturas siguen disponibles con el expediente
cerrado y filtran por expediente antes de paginar; las escrituras exigen que siga
activo. Preparar no reserva operación ni instante. Confirmar reproduce los
insumos bajo READ COMMITTED y el bloqueo común de auditoría, comprueba las huellas
y registra revisión, dependencias, recibo y auditoría en una sola transacción.
Los ensayos de fallo de auditoría y concurrencia comprueban rollback completo y
un único sucesor de una misma revisión esperada.

Atención y retiro cargan las referencias y las cabezas históricas capturadas en
la revisión base, sin sustituirlas por las actuales ni volver a ejecutar la
aritmética. El responsable histórico puede haber perdido acceso sin impedir que
otro gestor autorizado registre la actuación. Alta y corrección comprueban de
nuevo su cuenta activa, rol de personal y acceso actual al expediente; una
asignación como responsable nunca concede ese acceso. La entrega futura de
alertas tendrá que revalidarlo nuevamente.

### Esquema y capturas acotadas

Las cinco migraciones `0017_deadline_*.sql` añaden `case_deadlines` y
`case_deadline_revisions`. La raíz fija el expediente y exige R1 mediante una
clave foránea diferida. La historia es inmutable, consecutiva y terminal al
retirarse; el UUID de operación es único entre todos los plazos.

Cada revisión conserva el perfil exacto, DEVI1, DRES1, CADM1 de la administración
observada, responsable capturado, atención JSON estricta, los tres cánones de
recibo, sus huellas, autor e instante de registro. R0 se expresa con revisión
administrativa ausente y bytes CADM1 de la metadata original de `cases`; nunca se
fabrica una R1. Las dependencias tipificadas tienen claves foráneas a revisiones
exactas de hechos, resultados, calendarios y perfiles. La revisión del padre de
una notificación seleccionada puede diferir de la capturada en su cabeza; ambas
se conservan. El acuerdo seleccionado permanece en DEVI1.

`due_at_seconds` y `due_at_nanoseconds` son una proyección conjunta, presente o
ausente, del instante en DRES1. El lector compara ambos campos con el resultado
capturado. Estos campos preparan consultas posteriores; no implementan la agenda
conjunta ni una regla de alertas.

| Captura | Límite persistido |
| --- | --- |
| DEVI1 | 48–98 897 bytes; lectura completa antes de proyectar referencias. |
| DRES1 | Hasta 3 000 000 bytes; decodificación estructural acotada en Rust. |
| CADM1 | 17–16 384 bytes; reconstrucción desde R0 o la revisión exacta. |
| DLRV1 / DLST1 | Hasta 524 288 bytes cada uno; reconstrucción exacta en Rust. |
| DLTX1 | 107–4115 bytes; proyección SQL estricta y comprobación Rust. |

Las cotas de CADM1 y de los dos cánones de estado son conservadoras. CADM1 admite
3260 escalares en sus campos máximos y 77 bytes de estructura: hasta 13 117 bytes
UTF-8. Para DLRV1/DLST1, dos fuentes con hasta 32 asistentes, una reserva de 4096
bytes por proyección de asistente y 8192 por el resto de cada fuente, más tres
bloques de 8192 para perfil/calendarios y dos de 16 384 para contexto exterior y
administración, suman 335 872 bytes. Los payloads completos DEVI1, DRES1, DPRF1,
JCAL1, HRES1 y PFSRC1 se vinculan por huella dentro de esos cánones; no se concatenan
completos en ellos. Estas son cotas del formato actual, no máximos medidos de una
campaña de rendimiento.

El arranque valida columnas, nulabilidad, restricciones, expresiones generadas,
cuerpos y atributos de funciones, triggers y permisos. El rol operativo puede
leer y agregar filas; no posee objetos ni puede reescribir o borrar historia.
El inventario recorre todas las revisiones por lotes, incluido UUID cero, verifica
recibos y reconstruye las capturas sin recalcular con reglas actuales. Los CHECK
SQL verifican forma, huellas y proyecciones; no duplican el evaluador aritmético.
Una captura DRES1 estructuralmente coherente no acredita por sí sola su origen:
la preparación autorizada, su recibo, la transacción y la auditoría completan esa
frontera de confianza.

### Respaldo y alcance pendiente

El respaldo debe incluir ambas tablas y todas sus fuentes, usuarios, expedientes,
membresías, perfiles, auditoría y secuencia de eventos de fuentes. El primer
import legacy rechaza un destino con cualquiera de las dos tablas ocupada, incluso
si una restauración parcial dejó una raíz sin auditoría. La conciliación de un
recibo de importación existente conserva las revisiones posteriores.

La [aceptación de restauración](../crates/infrastructure/tests/deadline_restore.rs)
pasó con `pg_dump`/`pg_restore` reales. Compara filas completas, bytes, recibos,
fuentes, auditoría y expresiones literales de CHECK y columnas generadas antes y
después de restaurar. Incluye repetición de la migración con historia, retiro
posterior de fuente y perfil, revocación de autor y responsable, y consulta y
atención por otro Owner que conserva las capturas anteriores. La regresión de
siete pruebas de esquema también pasó tras ese ajuste.

El [ensayo HTTP integrado](../scripts/api-deadlines-demo.py) pasó con servicios
reales: días, meses y horas, cuatro roles, aislamiento, preparación y confirmación,
conflictos, atención, retiro y revocación. Después de la restauración global
comparó 14 respuestas completas de plazos idénticas, incluidos resultados
capturados. Esta evidencia cubre API y backend, no interacción de navegador ni
validez jurídica de los perfiles sintéticos. Véanse las
[operaciones de base de datos](database-operations.md).

La cola de reevaluación, sus cursores y reintentos, la sustitución atómica de
alertas, el correo y la agenda conjunta conservan el alcance del
[ciclo de vida](deadline-lifecycle.md). El catálogo de eventos de fuentes existente
no tiene aún un consumidor que actualice automáticamente estos plazos. La
interfaz Qadra y la aceptación de esos recorridos completos requieren sus propias
campañas; el ensayo HTTP aprobado no acredita interacción de usuario ni entrega
de alertas.
