# Contrato de aplicacion para hechos declarados

## Estado implementado

El modulo `crates/application/src/procedural_facts/` implementa comandos,
consultas acotadas, resolucion de material exacto, canones de fuentes y recibos,
y `ProceduralFactService` como implementacion de `ProceduralFactWorkflow`.
El servicio coordina autenticacion por rol, preparacion, admision documental,
reautenticacion, envio y comprobacion de las respuestas de sus puertos.
El [backend PostgreSQL](procedural-facts-persistence.md) implementa migraciones,
transaccion auditada, autorizacion por expediente y revalidacion bajo bloqueo.
Su cierre de verificacion local esta aprobado. La [API HTTP](procedural-facts-api.md)
y su composicion estan implementadas, con pruebas focales de DTO, proyecciones
y rutas aprobadas. La comprobacion HTTP integrada con restauracion tambien esta
aprobada localmente. El [cliente Qadra](../web/README.md#resoluciones-y-notificaciones-declaradas)
esta implementado y su verificacion integral esta aprobada localmente.
El [informe de verificacion](verification-report.md) distingue las pruebas de
aplicacion de la evidencia integrada de otras capacidades.

Los valores y los canones PFRES1/PFNOT1 siguen el [contrato de dominio](procedural-facts.md).
Los formatos PFSRC1/PFTXN1 se fijan en el [contrato de fuentes y recibos](procedural-facts-receipts.md).
La motivacion esta en [ADR-0031](adr/0031-declared-procedural-facts.md).

## Comandos e identidad

`FactChange<V>` distingue alta, correccion motivada y retiro administrativo.
Alta espera ausencia y produce revision 1. Correccion y retiro exigen revision
positiva y producen su sucesora; u32::MAX devuelve agotamiento, nunca wrap.
Retiro no acepta valores nuevos y es terminal para esa raiz. No acredita
nulidad ni modifica otras declaraciones, personas o documentos.

`NotificationCommand` comprueba que los nuevos valores seleccionen una revision
de su padre declarado. `validate_fact_base` tambien contrasta contra la base
almacenada el expediente y el `FactTarget` completo: familia, UUID y padre fijo.
Esto impide cambiar ambos padres del comando y sus valores para eludir la
identidad almacenada. La comprobacion se aplica igualmente al retiro.
Seleccionar otra revision del mismo padre o corregir personas si esta permitido.

La validacion de base rechaza alta sobre raiz existente, base ausente, revision
obsoleta y una raiz retirada. No consulta almacenamiento ni comprueba recibos,
permisos o unicidad de operacion. El servicio verifica ademas valores, fuentes
y recibo de la base; el adaptador debera repetir la comprobacion contra la cabeza
bajo bloqueo y resolver la existencia real de esa revision.

## Seleccion exacta y material interno

`FactSourceSelection::from_values` obtiene solo referencias declaradas:

| Fuente inmediata | Resolucion | Notificacion |
| --- | --- | --- |
| Padre resolucion | Ninguno | Una revision exacta |
| Fichas personales | Ninguna | Hasta cuatro referencias exactas |
| Resultados de audiencia | Hasta uno | Hasta dos selecciones, con acuerdo opcional |
| Documentos directos | Hasta uno | Hasta dos versiones exactas |

Las cuatro funciones personales son destinatario, receptor, representado y
representante. `Unknown` y `Unlinked` no se convierten en fichas. El orden es
UUID y revision; se deduplica exclusivamente la referencia completa. Se conservan
dos revisiones del mismo participante, dos acuerdos del mismo resultado y dos
versiones del mismo documento, incluso cuando sus digests coinciden. Ausencia
de acuerdo no equivale al UUID cero. Los valores originales conservan todos los
localizadores y funciones cuando la consulta de una fuente se deduplica.

`FactSourceMaterial` contiene snapshots exactos resueltos por el servidor:
resolucion padre opcional, hasta cuatro `ParticipantDetail` y hasta dos
`HearingResultSnapshot`. Dos acuerdos de una misma revision comparten material,
pero se comprueban por separado. La resolucion padre aporta su snapshot, un
resultado opcional y su soporte admitido opcional; no contiene otro padre ni
expande el grafo recursivamente. Los resolutores del servicio comprueban cotas,
presencia y union exacta del material. El adaptador debe acotar las
lecturas antes de decodificar; construir un `Vec` no garantiza ese presupuesto.

El lote `records` contiene exclusivamente los documentos directos. Ningun
antecedente agrega sus documentos a ese lote. En cada preparacion de alta o
correccion, el servicio comprueba identidad, version y digest de cada seleccion directa y admite el lote
completo de una o dos versiones en una sola llamada al procesador y validador,
con limites de 16 MiB por archivo y 32 MiB combinados. Si no hay soportes directos,
no llama al validador. Un documento usado directamente se admite aunque tambien
figure entre antecedentes. Retiro conserva fuentes y admisiones anteriores y
rechaza tanto registros documentales como material de reemplazo.

## Comprobaciones de participantes

`resolve_fact_participants` compara el material con toda la seleccion exacta,
rechaza faltantes, sobrantes, duplicados, expedientes o revisiones diferentes,
y ordena el resultado. Para una ficha manual recomputa su canon y exige ausencia
de sujeto tipado. Para una ficha tipada recomputa el canon de ficha y sujeto,
comprueba expediente, UUID, revision y digest del sujeto, y compatibilidad de
su clase con el perfil. Se permiten revisiones historicas inactivas; su seleccion
no afirma elegibilidad juridica.

La salida combina `FactParticipantSnapshot` con un `ParticipantOverview`
derivado de esos valores. No expone el sujeto completo, CURP ni todos sus datos.
Esta comprobacion no verifica una credencial criptografica, admite archivos ni
prueba existencia en una base de datos. El adaptador debe cargar las fuentes
reales del mismo expediente; el servicio compone esta comprobacion con las de
las demas fuentes antes de construir el borrador.

## Resultados y resolucion padre

`resolve_fact_hearings` exige la union exacta de revisiones de resultado,
comprueba expediente, valores HRES1 y recibo reconstruido, y verifica que cada
acuerdo seleccionado pertenece a esa revision. Dos selecciones de acuerdos de
un mismo resultado comparten un material; cada una conserva su propia vista.
No se exige que la cabeza actual siga registrada ni se readmiten documentos de
los resultados historicos. La comprobacion no reemplaza la carga real del
adaptador ni expande la programacion o continuidad de cada resultado.

`resolve_fact_resolution` comprueba identidad y revision del padre, reconstruye
sus fuentes inmediatas y coteja valores, fuentes y recibo. El soporte historico
conserva la admision capturada; no se procesa de nuevo su archivo en claro.
Un padre retirado puede ser fuente historica exacta. La vista se deriva del
padre verificado: clase, emisor, tiempo y resumen no se aceptan por separado.

`FactSourceViews` contiene las proyecciones legibles derivadas de resoluciones,
participantes y resultados. PFSRC1 vincula esos campos a sus referencias y
rechaza orden, cardinalidad o correspondencia inconsistentes. Dos acuerdos del
mismo resultado deben compartir sus datos comunes. Una correccion puede cambiar
una seleccion, pero no puede alterar silenciosamente el snapshot o la vista de
una referencia exacta que conserva, incluso al elegir otro acuerdo del mismo
resultado.

## Administracion observada

`validate_fact_administration` comprueba expediente y canon de una revision
administrativa registrada. Una base `Unrevised` conserva sus metadatos originales:
no fabrica revision, digest persistido, autor ni hora y no exige perfil penal
completo o etapa para declarar un hecho externo. Como ese valor no contiene UUID
de expediente, el adaptador debe resolverlo dentro del expediente autorizado.

Una administracion actual cerrada impide cambios. Con captura anterior, se
rechaza retroceso de revision o contradiccion de una misma revision inmutable,
incluidos autor y hora. Dos bases sin revision deben coincidir. Se permite pasar
de base sin revision a revision registrada, o avanzar a una revision posterior.
La observacion nunca se convierte en CAS administrativo: una actualizacion
administrativa valida no invalida por si sola el comando. El estado historico
capturado no reemplaza el estado vigente para decidir si se puede escribir.

Esta funcion es una comprobacion de preparacion de cambios, no un verificador
para denegar lecturas historicas de expedientes cerrados.

## Preparacion, envio y recibos

`prepare` autentica y exige `ManageProceduralFact` antes de consultar el puerto.
Valida expediente, identidad fija, revision esperada, estado de base y
administracion observada; resuelve fuentes y admite el lote directo cuando
corresponde. Construye un `PreparedFactChange` interno y reautentica al mismo
actor antes de devolver el borrador. No reserva UUID ni revision.

`submit` vuelve a preparar desde el comando, incluida la admision del lote
directo, y compara el digest de envio esperado
y reautentica al mismo actor antes de llamar a `commit`. Un digest diferente,
sesion revocada o actor distinto impide esa llamada. La respuesta del puerto
se comprueba contra actor, expediente, target completo, operacion, accion,
revision resultante y digest esperado, ademas de valores, fuentes y recibo.
La administracion devuelta tampoco puede retroceder respecto de la observada.

PFSRC1 codifica las referencias compactas, vistas y soportes admitidos. PFTXN1
vincula operacion, actor, expediente, raiz fija, accion, revision esperada,
digests de valores y fuentes y motivo. No introduce CAS administrativo ni
incluye correo o tiempo del servidor en esos canones. El adaptador PostgreSQL
los captura bajo bloqueo y los conserva en su transaccion auditada. Estas
propiedades se verifican en backend; no se deducen solo del recibo autocontenido.

El retiro conserva exactamente valores, fuentes y vistas de la base validada;
no acepta datos nuevos ni repite admision. El servicio mantiene la precision
declarada, sin inferir una fecha actual, orden cronologico ni restriccion general
sobre fechas futuras.

## Lecturas de aplicacion

Las cuatro consultas autentican y exigen `ReadProceduralFact` antes de llamar
al repositorio. Owner y Litigator tienen lectura y gestion por rol; Paralegal
solo lectura y Client ninguna de las dos. Pertenencia al expediente y estado
vigente de la cuenta tambien deberan comprobarse en el adaptador de cada operacion.

Los listados comprueban expediente, padre en ambas referencias de notificacion,
filtro de estado, orden UUID estricto y cursor exclusivo. Conservan UUID cero.
Una pagina tiene como maximo el limite solicitado de 1..100 filas. Si anuncia
mas resultados debe estar llena y devolver como cursor su ultima fila; si no
anuncia mas, el cursor debe estar ausente. El servicio rechaza incoherencias de
tamano y cursor antes de recorrer la pagina. El adaptador debera seleccionar
cabezas antes de filtrar y paginar; un overview aislado no prueba ser la cabeza.

El detalle exige target completo y, cuando se solicita, revision exacta; coteja
valores, union de fuentes y recibo. El historial admite 1..20 revisiones,
comprueba expediente y target, orden descendente estricto, cursor exclusivo,
continuacion coherente y cada recibo. Sus filas ligeras no sustituyen un detalle
ni permiten recomputar sus valores completos. Los expedientes actualmente
cerrados y las declaraciones retiradas conservan consultas autorizadas, con
la administracion historica de la captura verificada por el adaptador.

## Persistencia y cliente implementados

`ProceduralFactStore` fija obligaciones aplicadas por el
[adaptador PostgreSQL](procedural-facts-persistence.md):

- Owner en todos los expedientes, Litigator asignado para lectura y gestion,
  Paralegal asignado solo lectura y Client denegado, sin revelar fuentes ajenas.
- Cargar revisiones exactas y acotar material y documentos antes de decodificar;
  una referencia historica nunca se sustituye por la cabeza actual.
- Confirmar raiz, revision, recibo y auditoria en una transaccion comun. Bajo
  bloqueo, repetir identidad, membresia, expediente activo, cabeza esperada,
  operacion unica y correspondencia de documentos y fuentes preparados.
- Capturar Clock y administracion en esa frontera; la observacion del servicio
  no es un token de revision administrativa ni acredita la escritura atomica.

PostgreSQL, migraciones, inventario y pruebas reales de aislamiento, revocacion,
concurrencia, rollback y restauracion estan implementados y verificados localmente.
La composicion HTTP enlaza el servicio real y su API conserva las mismas
referencias, permisos y recibos. Sus pruebas focales y el recorrido con servicios
reales y restauracion tienen evidencia separada. Qadra incorpora captura,
seleccion historica, lectura y conciliacion explicita; su verificacion integral
esta aprobada localmente.
Las pruebas de puertos de aplicacion no sustituyen las comprobaciones de backend.
No se habilitan calculos juridicos, recursos, alertas o reevaluacion con este
servicio de declaraciones.
