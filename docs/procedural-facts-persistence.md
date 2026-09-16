# Persistencia de hechos declarados de resolucion y notificacion

## Estado y alcance

El backend implementa `PostgresProceduralFactStore`, migraciones, decodificadores
SQL/Rust, autorizacion por expediente y transacciones con la auditoria comun.
Las pruebas PostgreSQL cubren comportamiento, fallos, carreras, fuentes exactas,
catalogo, inventario e importacion/restauracion. El cierre de verificacion local esta
aprobado; los resultados ejecutados se registran en el
[informe de verificacion](verification-report.md), sin atribuir a una prueba
focal el resultado de toda la suite.

La [API HTTP](procedural-facts-api.md) y su composicion estan implementadas con
pruebas focales aprobadas. La comprobacion integrada HTTP con servicios reales
y restauracion tambien esta aprobada localmente; la interfaz Qadra sigue pendiente.
El backend no calcula plazos, selecciona recursos, determina eficacia de
notificaciones ni envia alertas. Conserva declaraciones y su procedencia conforme
al [modelo](procedural-facts.md), la [precision temporal](procedural-time.md),
la [aplicacion](procedural-facts-application.md) y [ADR-0031](adr/0031-declared-procedural-facts.md).

## Esquema e identidad

El conjunto `migrations/0014_procedural_fact*.sql` se instala mediante
`database migrate --runtime-role`, como parte de la migracion completa.
No ejecutar archivos sueltos ni crear revisiones para datos anteriores.

| Tabla | Datos conservados |
| --- | --- |
| `case_procedural_facts` | Familia, UUID, expediente, padre fijo de notificacion y primera revision |
| `case_procedural_fact_revisions` | Valores, fuentes, recibo, accion, motivo, administracion capturada, autor y Clock |

La clave de raiz es `(family,id)`: UUID iguales en familias distintas representan
identidades distintas. Una notificacion fija `parent_resolution_id` y exige una
resolucion previamente existente del mismo expediente. Cada revision conserva
la revision concreta de ese padre; puede cambiarla mediante correccion sin
cambiar su UUID. La FK de primera revision es diferida y exige R1 al confirmar.
La operacion UUID es unica entre ambas familias, incluso entre expedientes.

Alta produce R1 y estado `recorded`. Correccion y retiro requieren la cabeza
esperada y una sucesora representable en u32. `withdraw` produce `withdrawn`,
conserva exactamente valores y fuentes anteriores y termina esa raiz.
No borra la historia ni declara nulidad del acto. Los triggers bloquean UPDATE,
DELETE y TRUNCATE sobre ambas tablas.

## Bytes, proyecciones y recibos

Los parsers SQL independientes generan las vistas JSONB desde los bytes:

| Canon | Intervalo de bytes | Vinculo |
| --- | --- | --- |
| PFRES1 | 27..17700 | Valores declarados de resolucion |
| PFNOT1 | 67..58671 | Valores declarados de notificacion |
| PFSRC1 | 19..36847 | Fuentes inmediatas y vistas legibles |
| PFTXN1, resolucion | 141..4145 | Actor, operacion, caso, target, accion y digests |
| PFTXN1, notificacion | 157..4161 | El mismo recibo, incluido el padre fijo |

Los CHECK comparan SHA-256 y campos del recibo; el trigger contrasta ademas el
padre con la raiz y los valores. Los lectores Rust reconstruyen valores/fuentes
y exigen coincidencia exacta de bytes, digests y proyecciones. Rechazan campos
sobrantes, ausentes, tipos incompatibles y normalizaciones silenciosas.
Los formatos permanecen en [canones de valores](procedural-facts-canonical.md)
y [fuentes y recibos](procedural-facts-receipts.md); no cambian HRES1 ni HRTX1.

## Autorizacion, preparacion y confirmacion

| Cuenta activa | Lectura | Alta, correccion y retiro |
| --- | --- | --- |
| Owner | Todos los expedientes | Todos los expedientes activos |
| Litigator | Expedientes asignados | Expedientes asignados activos |
| Paralegal | Expedientes asignados | Denegado |
| Client | Denegado | Denegado |

El servicio autentica antes del puerto y reautentica al enviar. El adaptador
relee la cuenta, permisos y pertenencia en cada operacion. La falta de asignacion
no expone fuentes de otro expediente. Cerrar un expediente bloquea cambios y
conserva consultas autorizadas, incluso de revisiones retiradas.

`prepare` abre la frontera auditada en READ COMMITTED, comprueba la base,
operacion, administracion y seleccion, carga material acotado y termina mediante
rollback. No reserva una raiz, revision u operacion ni agrega un evento de
preparacion. La admision de archivos ocurre en la aplicacion fuera de esa
transaccion de lectura.

`commit` toma el mismo bloqueo transaccional que la cadena de auditoria y relee
la autoridad actual, la base, las fuentes y los documentos. Rechaza diferencias
respecto de la preparacion, comprueba el recibo y captura administracion, autor
y Clock bajo el bloqueo. Raiz nueva, revision, recibo y evento de auditoria se
confirman juntos. Un fallo no deja una operacion reservada ni una revision sin
su evento. Las lecturas exitosas tambien agregan su evento en la transaccion
que valida el acceso y la integridad.

El almacenamiento implementa consultas sincronas tras un mutex por instancia.
Esta entrega no introduce un pool ni clientes asincronos. Mantiene la frontera
[de auditoria compartida](adr/0016-case-document-transactions.md).

## Administracion historica y fuentes exactas

No se exige un perfil penal completo ni una etapa para declarar un hecho externo.
Una base administrativa sin revision se conserva como `Unrevised` (R0 logica):
revision y digest SQL nulos, titulo y referencia originales del expediente.
No se fabrica una revision 0 persistida, autor ni hora administrativa.
Una captura registrada conserva revision/digest y se resuelve contra su fila
historica; no guarda simultaneamente metadatos R0. Las formas parciales se rechazan.

La preparacion no establece un CAS administrativo. Una actualizacion valida de
administracion puede avanzar antes del commit; este captura la vigente si sigue
activa. Los lectores comprueban la captura historica, su no retroceso frente a
la revision anterior y que no preceda o contradiga la administracion de sus
fuentes de resolucion y resultado. Una administracion vigente posterior o
cerrada no sustituye aquella captura.

Las fuentes se seleccionan por expediente, familia cuando corresponde, UUID y
revision exactos. Las fichas manuales conservan su canon y digest; las tipificadas
incluyen el sujeto exacto y su digest, sin revelar por defecto todos sus datos.
Los resultados conservan HRES1, recibo y pertenencia del acuerdo seleccionado.
Los padres retirados, fichas historicas archivadas y resultados retirados siguen
siendo referencias historicas validas; esto no afirma elegibilidad juridica.

`validate_procedural_fact_sources` coteja la union inmediata derivada de los
valores con las fuentes almacenadas y sus proyecciones historicas completas.
No acepta fuentes omitidas, sobrantes o sustituidas. Retener una referencia exacta
conserva su snapshot; elegir otro acuerdo del mismo resultado conserva todos los
datos comunes de esa revision. Ausencia de acuerdo y UUID cero son distintos.

El lote directo tiene cero o una version en resoluciones y hasta dos en
notificaciones. Se deduplica por documento/version conservando localizadores en
los valores. El servicio admite el lote completo no vacio en una llamada por
preparacion, incluida la del envio, con las cotas documentales vigentes.
Los documentos de antecedentes no se agregan a ese lote. El adaptador vuelve a
comparar registros preparados; SQL comprueba caso, version, digest y nombre.
La admision PDF/DOCX corresponde al validador, no a una extension de archivo ni
a la mera existencia de una fila. Retiro no solicita nueva admision.

## Consultas y control del inventario

Los listados seleccionan cabezas antes del filtro de estado y de la paginacion,
usan UUID ascendente con cursor exclusivo y admiten UUID cero. Las notificaciones
se consultan bajo su padre. El historial usa revision descendente y cursor
exclusivo. Se aplican los limites del contrato de aplicacion; el detalle puede
seleccionar una revision exacta y valida tambien su predecesora inmediata.

El arranque valida tablas, columnas, expresiones generadas, restricciones,
cuerpos y propiedades de funciones, triggers y privilegios. El rol
operativo solo puede SELECT/INSERT sobre las dos tablas y ejecutar los helpers
necesarios; no debe poseer objetos ni heredar permisos de modificacion del
historial. Las funciones fijan referencias al esquema y `search_path=pg_catalog`.

El inventario recorre todas las revisiones en paginas de 64, con familia/UUID/
revision como cursor, incluyendo ambas familias con UUID cero. Comprueba primera
revision, continuidad, bytes, digests y cada detalle historico con fuentes y
captura administrativa. Abrir el adaptador no ejecuta DDL ni repara una historia
inconsistente. Catalogo valido y claves foraneas no sustituyen ese recorrido.

## Importacion, respaldo y comprobacion

El primer import legacy exige vacias ambas tablas de hechos, incluso si solo
hay una raiz o una revision huerfana. La conciliacion de un recibo de importacion
existente conserva operaciones posteriores; no reimporta ni regenera hechos.

Respaldar las dos tablas junto con usuarios, expedientes, administracion,
personas/sujetos, resultados, documentos y auditoria. Conservar sus bytes y
capturas, no reconstruirlos desde cabezas actuales. Restaurar tambien roles y
permisos. El procedimiento comun esta en [operaciones de base de datos](database-operations.md).

Las pruebas versionadas incluyen parsers/vectores, fuentes exactas, permisos,
conflictos concurrentes, fallo de auditoria, inventario y `pg_dump`/`pg_restore`.
El ensayo de restauracion compara historia exacta despues de cambios de
administracion y autor, y permite una nueva notificacion sobre un padre retirado
al reabrir el expediente. Es evidencia de backend PostgreSQL; no atribuirla a
un recorrido HTTP o de navegador aun no implementado.
