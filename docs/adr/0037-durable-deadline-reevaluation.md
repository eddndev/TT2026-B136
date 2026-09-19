# ADR 0037: Reevaluacion durable y vigencia operativa de plazos

## Status

Propuesto. El registro humano descrito en
[ADR 0036](0036-persisted-deadline-evaluation-and-attention.md) esta implementado.
Este documento describe su ampliacion y distingue la implementacion local de
su integracion operativa completa. El estado de cada frontera aparece debajo.

El modelo de aplicación V2, las observaciones verificadas y el verificador de
sucesores ya están implementados como contratos puros. Existen preparadores
humano y técnico con seguimiento explícito. La persistencia PostgreSQL admite
decisiones humanas V2 y reconstrucción histórica compatible con V1. El
[despachador persistente](../deadline-dispatch.md) expande eventos y legado en
trabajos, con cursores y auditoría atómicos. El [consumidor local](../deadline-worker.md)
implementa confirmación técnica, resultados sin cambios e intentos durables.
La restauración real sin migración reparadora y las comprobaciones focales de
catálogo y permisos aprobaron. Las regresiones focales de clasificación de
fallos también aprobaron; la regresión integral de esa entrega tiene su propio
registro en el informe de verificación. El servicio humano y HTTP exponen V2 y
consultas de vigencia con verificación focal. Qadra V2 dispone de pruebas
unitarias y de navegador con HTTP controlado. La composición desde `serve` está
implementada localmente y su planificación y supervisión tienen pruebas focales.
La aceptación real de Qadra completó 25 escenarios, incluidos los recorridos
de reevaluación y revisión humana en escritorio y móvil. El recorrido HTTP
conservó R1 a R5 tras reinicios y restauración. La regresión final de la revisión
publicada y su integración siguen pendientes; el ADR permanece propuesto hasta
cerrar esa entrega. El alcance y las limitaciones de las comprobaciones están
en [el informe de verificación](../verification-report.md).

## Context

El registro V1 conserva calculos reproducibles, fuentes exactas, responsable
y atencion. Los cambios de fuentes, calendarios y perfiles emiten eventos
inmutables. Al proponer esta ampliacion no existia un consumidor que aplicara
esos cambios a cada plazo dependiente ni una distincion persistida entre un
vencimiento historico y uno habilitado para seguimiento actual. El estado
de implementacion de esta ampliacion se mantiene en Status.

Una nueva fuente o regla puede invalidar las declaraciones con las que se
calculo un plazo. Reutilizarlas por coincidir identificadores de condiciones
no demuestra aplicabilidad. El cierre administrativo tampoco suspende un
termino juridico. Los comandos humanos requieren una cuenta y un expediente
activos, por lo que un trabajador no debe aparentar ser un Owner ni reutilizar
la correccion humana para eludir esas comprobaciones.

## Decision

### Historia y politicas explicitas

Mantener una sola historia consecutiva por plazo. Agregar una transicion
tecnica de reevaluacion con autor tecnico, causa y trabajo durables propios.
La API de decisiones humanas no admite autores tecnicos. Conservar los bytes
y verificadores de los recibos existentes; las revisiones nuevas usan una
version explicita que compromete tambien politicas, observaciones y estado
operativo. El formato se selecciona por version, nunca por aceptar otro
verificador cuando falla el primero.
Los contratos DLTX2, DLOB1, DLOE1, DLRV2 y DLST2 se describen en
[los recibos de seguimiento](../deadline-tracking-receipts.md). El modelo
selecciona la evidencia y la autoría mediante variantes explícitas; los codecs
y verificadores puros no sustituyen la resolución de revisiones persistidas.

Declarar por dependencia si se sigue su cabeza, se conserva una revision fija
o aun no se determina esa politica. La igualdad entre revision seleccionada
y observada no demuestra seguimiento; su diferencia no demuestra fijacion.
El legado conserva esa falta de declaracion y requiere conciliacion explicita.

Un calendario seguido permite una nueva evaluacion con su revision exacta.
Un cambio de fuente o perfil seguido requiere revisar la calificacion humana.
Una referencia fija conserva la seleccion ante cambios ordinarios. Un retiro
nuevo requiere revision incluso para referencias fijas. Un evento ya observado
no retrocede la seleccion ni provoca otra revision por si solo.

`Fixed` conserva una selección ante cambios ordinarios; no constituye una
aceptación implícita de una dependencia retirada. El preparador humano rechaza
una nueva calificación si las cabezas de fuente, calendario o padre están
retiradas, y exige que perfil seleccionado y cabeza sigan publicados. La
conciliación excepcional que pudiera justificar otra decisión necesita un
contrato explícito todavía pendiente.

### Calculo historico y estado operativo

Preservar el calculo anterior y sus referencias aunque una revision posterior
requiera calificacion. La revision pendiente no expone el vencimiento anterior
como vencimiento operativo para agenda o alertas. La ausencia de un instante
calculado tampoco se completa por aceptar una revision. La atencion declarada
y el responsable capturado se conservan; ni un trabajador ni una nueva
correccion pueden reiniciarlos silenciosamente.

Las observaciones nuevas identifican las cabezas realmente examinadas, incluida
la del perfil. Una notificacion conserva su propio padre historico; un cambio
del padre no la reencadena. Si un acuerdo desaparece de un resultado nuevo,
se requiere revision y no se selecciona otro acuerdo por posicion.

El constructor estricto comprueba recibos, ámbito, identidad y orden de
revisiones; cada observación compromete la evidencia completa mediante DLOE1,
además del recibo de origen. La igualdad de revisión exige la misma evidencia,
incluidos los desplazamientos temporales capturados. La cabeza del padre de
una notificación requiere una resolución completa verificada por separado.

El constructor de observaciones legadas sólo recupera el perfil seleccionado
y las cabezas de fuente y calendario ya capturadas en V1. No consulta cabezas
actuales ni inventa el padre que falta. La conciliación inicial técnica usa
causa `LegacyBootstrap`, conserva la evaluación histórica y deja políticas sin
determinar y revisión pendiente, sin fabricar un evento de fuente.

El preparador y el verificador del primer sucesor técnico V2 reconstruyen ese
manifiesto V1 para comprobar la causa y el avance de observaciones. La ausencia
de seguimiento explícito no borra las cabezas históricas: una revisión igual
conserva su evidencia exacta, y ni un evento ni el bootstrap pueden hacerla
retroceder. Un evento ya observado en V1 tampoco justifica otra revisión
técnica por el mero hecho de que la cabeza actual haya avanzado.

DLRV2 compromete la única evaluación histórica, las políticas, el estado y
los motivos canónicos de revisión, junto con la huella del manifiesto DLOB1.
DLST2 compromete además la administración observada con su evidencia completa,
sin reemplazar la administración histórica del cálculo. El vencimiento
operativo sólo existe si el plazo está activo, la revisión está `Accepted`,
la evaluación histórica contiene un instante y una lectura autorizada ha
comprobado la vigencia de sus dependencias. El legado sin declaración,
la revisión pendiente y el retiro no habilitan una fecha operativa.

### Comprobación actual separada de la captura

La lectura actual resuelve la cabeza del plazo y las dependencias pertinentes
bajo una misma transacción autorizada con el bloqueo común de auditoría. Verifica
la evidencia histórica seleccionada, la continuidad administrativa y las cabezas
completas; compara todas las observaciones con sus políticas sin ejecutar otra
vez el cálculo. La cabeza del padre de una notificación se resuelve de forma
independiente. El instante de comprobación se obtiene después de esas lecturas.

`DeadlineCurrent` mantiene juntos el detalle verificado y `DeadlineOperational`,
con una vinculación privada a identidad, revisión y captura. El resumen copia
esos campos y conserva la vinculación; el servicio y el proyector rechazan una
sustitución de identidad, fecha, desfase o metadatos resumidos. La lista aplica
paginación y filtro antes de cargar cabezas y entrega datos sólo después de
confirmar su auditoría. No se recupera de una inconsistencia devolviendo vigencia
sin comprobar o un resultado histórico como si fuera actual.

`Current` significa que las cabezas no exigen actualizar el seguimiento capturado
según sus políticas; `Changed` impide usar la fecha histórica. `NotChecked`
identifica consultas exactas, confirmaciones, legado sin políticas y retiro.
Un seguimiento pendiente puede estar `Current` respecto de sus observaciones y
seguir sin fecha operativa porque falta aceptación humana. Los cambios ordinarios
`Fixed` pueden conservar vigencia; un retiro nuevo exige revisión. La cola, el
cursor y un resultado sin cambios no se usan como prueba de vigencia. La lectura
no modifica trabajos ni revisiones. Cada futura agenda o entrega de aviso debe
revalidar acceso y vigencia para su propio instante de uso.

### Preparacion humana y sucesores

El preparador de seguimiento humano admite las mismas cuatro acciones y
requiere autor humano. Alta y corrección declaran políticas y aceptación;
atención y retiro conservan el seguimiento anterior, incluso si está pendiente.
Una atención o retiro sobre V1 puede adquirir recibo V2 conservando
`LegacyUndeclared`; ese cambio de formato no acepta ni fija políticas.

La conversión manual conserva exactamente el manifiesto reconstruido de V1
y la administración histórica del cálculo. No añade un padre de notificación
observado ni una cabeza posterior, aunque esa evidencia sea auténtica. Atención
y retiro sobre V2 conservan los bytes de seguimiento y observaciones anteriores.
La incorporación de observaciones nuevas corresponde a otra transición con
su propia causa y validación.

El verificador de sucesores exige identidad de expediente y plazo, revisiones
consecutivas y ambas huellas del predecesor V2. Rechaza volver de V2 a V1 y
continuar después del retiro. Compara evidencia canónica para conservar
atención, responsable y cálculo según la acción, incluidos los desplazamientos
temporales. Una reevaluación técnica conserva políticas y motivos pendientes,
no acepta por sí sola una revisión y no hace retroceder observaciones.

La observación administrativa respeta el orden de revisiones: una revisión
registrada no puede regresar a la base original ni a una revisión anterior.
Si la revisión no cambia, debe conservar la evidencia completa, incluido el
desplazamiento temporal. El primer sucesor V2 toma como referencia la
administración del cálculo V1; los siguientes usan la observación de seguimiento.
Estas comprobaciones no eliminan la preservación adicional exigida por atención
y retiro.

Cada perfil o fuente que avanza con `Follow` exige revisión pendiente y un
motivo de cambio o retiro de su propio ámbito. La cabeza del padre de
notificación se revisa como fuente, también cuando se incorpora por primera vez.
Se comprueban todas las dependencias avanzadas de la captura, no sólo la que
originó el evento, conservando los motivos anteriores. El estado real de retiro
debe comprobarse con la evidencia persistida; DLOB1 no lo declara. El calendario
seguido conserva su regla específica de recálculo.

La excepción de cambio de cálculo corresponde a un calendario seguido y
publicado, con revisión aceptada antes y después. Puede combinar avances
ordinarios de fuentes o perfiles fijos, conservando sus selecciones y
calificaciones históricas, incluso cuando el evento procede de una de esas
dependencias. Un avance de fuente o perfil seguido exige revisión pendiente
y conservación del cálculo anterior. El calendario fijo también conserva su
selección y resultado ante cambios ordinarios. El verificador no ejecuta de
nuevo la aritmética histórica; el preparador técnico calcula el nuevo resultado
de calendario cuando se cumplen esas condiciones.

El evento procesado puede preceder a la cabeza examinada. Para una dependencia
ya observada, una revisión nueva exige el intervalo
`observada_anterior < evento <= cabeza_nueva`, sin cambiar identidad ni ámbito.
La causa mantiene la operación original del evento. Cuando evento y cabeza
coinciden en revisión, el preparador contrasta su operación exacta; si la cabeza
es posterior, el adaptador debe verificar la operación en la revisión durable
del evento.

El preparador técnico devuelve una revisión preparada o un motivo tipificado
sin cambio: plazo retirado, evento ya observado, dependencia no seleccionada o
legado ya inicializado. `Retired` y `AlreadyInitialized` dependen de la base y
la forma de la causa verificadas; se resuelven antes de examinar cabezas que
no influyen en esas decisiones. En las demás rutas, el preparador comprueba
la continuidad y evidencia exacta de la administración antes de devolver
`AlreadyObserved` o `DependencyNotSelected`. Un evento repetido o ajeno no
permite ocultar retrocesos administrativos ni reescribir evidencia de la
misma revisión, incluido el desplazamiento temporal.

La revisión preparada retiene base e insumos examinados, enlaza ambas huellas
anteriores, conserva datos humanos y usa el servicio
`DeadlineReevaluator` con política 1. La persistencia elige el momento de
registro y debe comprobar otra vez la base y las cabezas pertinentes antes de
confirmar, también si el resultado no requiere revisión. Para `Retired` y
`AlreadyInitialized` no se requieren cabezas irrelevantes para la decisión;
se revalida la base, además de los controles durables de servicio, causa y
trabajo que corresponden al adaptador. El núcleo no reserva revisiones ni
completa trabajos. Esa frontera pertenece al consumidor PostgreSQL: autentica
el trabajo durable y confirma su resultado junto con la revisión y auditoría
cuando corresponde. El despachador conserva la asignación y sus cursores.

### Despacho y ejecucion

Persistir un cursor de eventos y, durante una expansion paginada, su posicion
exclusiva de plazo. Insertar trabajos unicos por evento y plazo junto con el
avance del cursor. Cubrir resolucion, notificacion, resultado de audiencia,
calendario y perfil; filtrar identidad tipificada, expediente y dependencias
de la cabeza vigente antes de paginar.

Confirmar la nueva revision, auditoria y resultado del trabajo atomicamente.
Un reintento concilia una operacion estable ya confirmada. Las correcciones,
atenciones y retiros concurrentes obligan a comprobar otra vez la cabeza; un
trabajo atrasado no restaura datos humanos anteriores. El retiro del plazo
es terminal. La reevaluacion tecnica puede continuar en expedientes cerrados
y conserva responsables revocados como evidencia, sin autorizar futuras
entregas protegidas a esas cuentas.

El consumidor actual ejecuta un trabajo por transacción bajo el bloqueo común
de auditoría. No necesita una reclamación con vencimiento ni roles nuevos.
La revisión técnica y su resultado se exigen mutuamente al confirmar; los
resultados sin cambios conservan causa, base y evidencia examinada. Sus lectores
reconstruyen referencias históricas, sin consultar cabezas actuales ni duplicar
los verificadores existentes. No se hacen llamadas externas bajo ese bloqueo.

Una ejecución fallida revierte antes de registrar un intento y su auditoría en
otra transacción. Los reintentos transitorios usan espera creciente de 1 a 300
segundos para una misma base; las inconsistencias esperan 3600 segundos. Una
base nueva invalida la espera anterior. `Deferred` sólo significa que el intento
se confirmó; no convierte el fallo en éxito. Si la respuesta es incierta, se
concilian las identidades originales del resultado y del intento.

La apertura valida todo el inventario. La recuperación entre reinicios exige
inventario válido; una corrupción persistente impide abrir hasta repararla o
restaurar. Una conexión ya abierta puede registrar el fallo y permitir otros
trabajos si puede confirmar su intento y auditoría, sin eludir ese control.
Un fallo que impide también esa confirmación sigue siendo un error.

Los presupuestos de bloqueo y sentencia no acotan el tiempo total de apertura,
la búsqueda entre todos los trabajos pendientes ni el cálculo Rust. Su coste y
la latencia de operaciones concurrentes requieren mediciones propias. Si en el
futuro se separa el cálculo de la transacción, esa nueva frontera necesitará
reclamación y protección contra confirmaciones de trabajadores reemplazados.

### Composicion y cierre del servidor

`serve` abre y valida despachador y consumidor antes de aceptar tráfico. Un
único consumidor bloqueante alterna eventos y conciliación del legado. Cada
ciclo despacha una página e intenta como máximo el mismo número de trabajos,
en serie, seguido siempre por una pausa positiva. `--deadline-page-limit`
admite 1 a 100, con 20 por omisión; `--deadline-poll-ms` admite un entero positivo
de 32 bits, con 1000 por omisión. Un resultado `Deferred` consume una unidad
del lote y permite continuar con otros trabajos elegibles. No se drena una
cola completa antes de ceder ejecución.

Los errores de puerto retornados permiten otro ciclo después de la pausa;
los errores explícitos de configuración o integridad detienen el consumidor
y el servidor. La reconexión de ambos adaptadores repite la apertura con
validación completa del inventario y reinstala los presupuestos por sentencia
y bloqueo. Ningún reintento ejecuta trabajo sobre una conexión no validada.

El supervisor registra SIGINT y SIGTERM en Unix antes de iniciar el trabajo.
Una señal, el fin del HTTP o el fin del consumidor solicitan la parada y el
cierre gradual del HTTP. Se espera tanto el fin de las solicitudes como el
consumidor bloqueante; no se aborta ni se abandona una transacción en curso.
La espera entre ciclos se interrumpe por parada, pero una notificación sin
parada no reduce la pausa mínima. Un fallo o fin inesperado del consumidor
no deja al servidor funcionando sin procesamiento. Los diagnósticos de esa
frontera usan categorías estables y conservan fallos de ambos lados al cerrar.

El propietario final del router y de los adaptadores síncronos permanece fuera
del contexto asíncrono. Esto permite liberar las conexiones PostgreSQL después
del cierre sin ejecutar su limpieza bloqueante dentro de Tokio. La política
acota llamadas por ciclo, no el tiempo total de apertura, consulta, cálculo o
cierre: los presupuestos por sentencia no constituyen un plazo global.

### Verificacion

Probar recibos antiguos y nuevos, alteracion de politicas y causas, rechazo de
autores tecnicos en HTTP humano, proyeccion operativa nula cuando falta revision,
las cinco familias, expansion paginada, dos despachadores y trabajadores,
huecos de secuencia, rollback, respuesta incierta, restauracion y concurrencia
con decisiones humanas. La interfaz debe mostrar autor, causa, historia y
revision requerida, manteniendo aislamiento, conflictos y borradores.

Los contratos de aplicación comprueban V1/V2, observaciones y sucesores. El
adaptador persiste decisiones humanas V2 y reconstruye las revisiones exactas
observadas; sus migraciones conservan filas y recibos históricos V1. El HTTP
escribe decisiones humanas V2 y conserva historia V1/V2 con autoría y causa
explícitas. El contrato de [seguimiento y vigencia](../deadline-tracking-api.md)
separa cálculo conservado y fecha operativa. La escritura técnica local exige la
frontera durable de trabajo, resultado y causa; el puerto humano sigue rechazando
la autoría técnica. Los resultados ejecutados y sus límites se registran en
[el informe de verificación](../verification-report.md). Las pruebas puras no
acreditan concurrencia o recuperación, y las pruebas del consumidor no acreditan
su composición en servidor, interacción Qadra ni entregas externas.

## Consequences

La representacion HTTP y sus lectores estrictos deben evolucionar juntos con
la persistencia. La conservacion de recibos anteriores no implica que un
cliente antiguo entienda acciones o autores nuevos. El instalador, inventario,
permisos y restauracion deben conocer las nuevas transiciones y tablas.

Esta decision no califica juridicamente un perfil, no crea plazos aplicables
a partir de texto libre y no acredita una entrega de correo. La activacion
desde actos, el corpus juridico, la agenda y las alertas siguen requiriendo sus
flujos y evidencia de aceptacion propios.
