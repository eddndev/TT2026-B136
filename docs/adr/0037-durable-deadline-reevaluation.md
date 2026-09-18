# ADR 0037: Reevaluacion durable y vigencia operativa de plazos

## Status

Propuesto. El registro humano descrito en
[ADR 0036](0036-persisted-deadline-evaluation-and-attention.md) esta implementado.
Este documento describe su ampliacion; no acredita que el trabajador, su
persistencia o la interfaz nueva esten terminados.

El modelo de aplicación V2, las observaciones verificadas y el verificador de
sucesores ya están implementados como contratos puros. Existe un preparador
humano con seguimiento explícito; su conexión al servicio persistido sigue
pendiente. Persistencia V2, trabajador durable, API V2 y Qadra aún no integran
esta ampliación. El ADR permanece propuesto para ese conjunto de trabajo.

## Context

Los plazos conservan calculos reproducibles, fuentes exactas, responsable y
atencion. Los cambios de fuentes, calendarios y perfiles emiten eventos
inmutables. Todavia no existe un consumidor que aplique esos cambios a cada
plazo dependiente ni una distincion persistida entre un vencimiento historico
y uno habilitado para seguimiento actual.

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

DLRV2 compromete la única evaluación histórica, las políticas, el estado y
los motivos canónicos de revisión, junto con la huella del manifiesto DLOB1.
DLST2 compromete además la administración observada con su evidencia completa,
sin reemplazar la administración histórica del cálculo. El vencimiento
operativo sólo existe si el plazo está activo, la revisión está `Accepted` y
la evaluación histórica contiene un instante. El legado sin declaración,
la revisión pendiente y el retiro no habilitan una fecha operativa.

### Preparacion humana y sucesores

El preparador de seguimiento humano admite las mismas cuatro acciones y
requiere autor humano. Alta y corrección declaran políticas y aceptación;
atención y retiro conservan el seguimiento anterior, incluso si está pendiente.
Una atención o retiro sobre V1 puede adquirir recibo V2 conservando
`LegacyUndeclared`; ese cambio de formato no acepta ni fija políticas.

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
publicado, con revisión aceptada antes y después, evento exacto y conservación
de las demás evidencias. El verificador no ejecuta de nuevo la aritmética;
preparar ese resultado técnico y confirmar su origen durable sigue pendiente.

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

La estrategia de carga y calculo debe decidirse con mediciones del material
maximo admitido y de la latencia de operaciones concurrentes. Una transaccion
corta por trabajo simplifica la recuperacion; si el calculo exige separarla,
la reclamacion necesita vencimiento y un token que invalide confirmaciones de
trabajadores reemplazados. No se ejecutan llamadas externas bajo el bloqueo
comun de auditoria.

### Verificacion

Probar recibos antiguos y nuevos, alteracion de politicas y causas, rechazo de
autores tecnicos en HTTP humano, proyeccion operativa nula cuando falta revision,
las cinco familias, expansion paginada, dos despachadores y trabajadores,
huecos de secuencia, rollback, respuesta incierta, restauracion y concurrencia
con decisiones humanas. La interfaz debe mostrar autor, causa, historia y
revision requerida, manteniendo aislamiento, conflictos y borradores.

Los contratos de aplicación ya permiten comprobar V1/V2, observaciones y
sucesores sin almacenamiento. El adaptador conserva temporalmente V1 y el HTTP
rechaza los registros V2 que no puede proyectar completos, en lugar de omitir
autoría o revisión. Esto es una frontera de compatibilidad provisional, no la
entrega de la API nueva. Los resultados ejecutados y sus límites se registran
en [el informe de verificación](../verification-report.md); las pruebas puras
no acreditan trabajador, concurrencia, restauración V2 ni entregas externas.

## Consequences

La representacion HTTP y sus lectores estrictos deben evolucionar juntos con
la persistencia. La conservacion de recibos anteriores no implica que un
cliente antiguo entienda acciones o autores nuevos. El instalador, inventario,
permisos y restauracion deben conocer las nuevas transiciones y tablas.

Esta decision no califica juridicamente un perfil, no crea plazos aplicables
a partir de texto libre y no acredita una entrega de correo. La activacion
desde actos, el corpus juridico, la agenda y las alertas siguen requiriendo sus
flujos y evidencia de aceptacion propios.
