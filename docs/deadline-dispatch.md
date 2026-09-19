# Despacho persistente de reevaluación de plazos

## Estado y frontera

El contrato `crates/application/src/deadline_dispatch/` y el adaptador
`PostgresDeadlineDispatchStore` expanden cambios de dependencias en trabajos
persistentes. Cada llamada procesa una página y confirma sus trabajos, el
avance y la auditoría en una transacción. Las migraciones `0019_` y
`crates/infrastructure/src/deadline_dispatch_schema/` definen y verifican esta
frontera. La decisión general está en
[ADR 0037](adr/0037-durable-deadline-reevaluation.md).

El [consumidor local](deadline-worker.md) confirma revisiones técnicas,
resultados sin cambios e intentos durables. El despachador y el consumidor
se componen en `serve` mediante un único bucle bloqueante serial, con parada y
supervisión. El ejecutable aprobó 31 pruebas unitarias y cuatro de ayuda CLI;
las 26 de bucle, parada y supervisión están incluidas en las 31. La campaña API
real comprobó reevaluación, cierre TERM/INT, reinicios y restauración de R1-R5.
Qadra V2 aprobó 25 recorridos con backend real, incluidos dos Follow de escritorio
y móvil, además de sus pruebas con HTTP controlado. No se expone una orden
humana de reevaluación por HTTP: el cliente puede consultar revisiones técnicas,
pero no elegir su autor ni enviar comandos de trabajador. La repetición final
API tras corregir su comprobación de token, la nueva regresión global y la
integración en `main` permanecen pendientes; véase
[el informe de verificación](verification-report.md).

Persistir un evento o asignar un trabajo no significa que el plazo ya haya sido
reevaluado ni que se haya entregado una notificación. El detalle actual y el
listado comprueban vigencia a partir de cabezas verificadas; no usan la posición
del cursor o el estado de la cola como prueba de frescura. Véase
[el contrato de seguimiento](deadline-tracking-api.md).

## Dos recorridos independientes

`Events` consume el siguiente evento realmente almacenado después del último
completado. Conserva evento activo y UUID exclusivo cuando una página deja
candidatos; los borra al completar el evento. Un hueco de la secuencia por
rollback no bloquea el avance. Un evento sin candidatos también se completa.
No se supone que el siguiente número sea el anterior más uno.

`LegacyBootstrap` busca plazos activos V1 o con estado `LegacyUndeclared` que
todavía no tengan trabajo de conciliación de política 1. Su cursor es separado
y vuelve al inicio cuando termina una pasada. Así encuentra registros V1
añadidos después con UUID menor que el último recorrido, sin duplicar trabajos
ni declarar aceptación humana. V1 y el seguimiento humano V2 pueden coexistir.

Los candidatos se seleccionan por la revisión actual del plazo:

| Cambio | Relación que selecciona el plazo |
| --- | --- |
| Resolución | Fuente directa o padre de la notificación seleccionada, en el mismo expediente. |
| Notificación | Identidad y familia de la fuente, en el mismo expediente. |
| Resultado de audiencia | Resultado, audiencia y expediente coincidentes. |
| Calendario | Identidad del calendario seleccionado. |
| Perfil | Identidad del perfil y su ámbito global o de expediente. |

Una coincidencia únicamente histórica no selecciona la cabeza actual. UUID
iguales en familias diferentes no intercambian causas. El cierre administrativo
del expediente y la pérdida de acceso del responsable no eliminan el seguimiento
técnico. Los plazos retirados quedan fuera de nuevas expansiones; sus trabajos
históricos se conservan y el consumidor decide sobre la base actual verificada.

## Transacción, identidad y recuperación

El adaptador obtiene el bloqueo común de auditoría bajo READ COMMITTED antes
de leer el cursor y los candidatos. Reconstruye la captura de cada candidato
mediante el lector verificado, sin retener todas las capturas de una página.
Inserta UUID de trabajo, UUID de operación, plazo, expediente, causa y fecha
UTC exacta. Un conflicto en la clave lógica conserva la fila ya existente.

La unicidad es por evento/plazo o por plazo/política de conciliación. Los UUID
de operación también se reservan frente a las operaciones humanas: una colisión
en cualquiera de los sentidos devuelve `OperationConflict`. La asignación de
trabajos y el movimiento de cursores generan `deadline.dispatch_advanced` con
autor técnico, causa, cantidades y posiciones anterior/nueva. Una consulta
ociosa no fabrica progreso ni un evento de auditoría.

La confirmación precede al resultado entregado al llamante. Perder esa respuesta
permite reabrir y continuar desde la posición persistida. Si falla la escritura
de auditoría, toda la página revierte. Dos despachadores comparten el mismo
orden de bloqueo y no pueden confirmar páginas contradictorias.

Una conexión cerrada se recupera desde el mismo adaptador. Cada reconexión
vuelve a comprobar esquema, permisos e inventario completo antes de despachar,
y restablece los presupuestos de bloqueo y sentencia. Un esquema alterado o un
cursor ausente impiden continuar hasta reparar o restaurar la evidencia; no se
reconstruye progreso supuesto. Cuatro pruebas de reconexión verifican continuación
sin duplicados, rechazo y recuperación. La regresión focal incluye además cinco
pruebas de atomicidad y dos de presupuestos: once aprobadas en total. Véase
[el informe de verificación](verification-report.md).

SQL rechaza saltos de evento, regresiones, posiciones que no identifican un
trabajo, intervalos con candidatos sin asignar y cambios simultáneos de ambos
recorridos. Los trabajos son inmutables. Los cursores sólo admiten cambios de
posición; no permiten eliminar o volver a insertar su identidad.

Estas comprobaciones SQL complementan al adaptador auditado. No afirman que
cualquier sentencia SQL arbitraria del rol operativo añada automáticamente
su evento de auditoría. Las credenciales de base de datos pertenecen al servicio.

## Paginación y presupuestos

La API admite límites de 1 a 100, con valor predeterminado 20. Solicita una fila
adicional para saber si queda otra página. El helper SQL aplica intervalo UUID,
filtro de trabajos pendientes y límite antes de materializar el resultado:
retorna como máximo 101 filas para una página y una fila para cada comprobación
de existencia. Las validaciones de un trabajo usan el intervalo exacto de su UUID.
UUID nulo como valor y ausencia de cursor son estados distintos.

El adaptador configura un segundo para esperar bloqueos y cinco segundos por
sentencia en cada conexión, incluida su recuperación, antes de iniciar la
transacción de despacho. Son presupuestos de sentencia, no un límite global
de tiempo para toda la página o para su apertura.
Una página puede examinar muchas raíces si las coincidencias son dispersas;
no se garantiza tiempo constante ni memoria total constante del ejecutor SQL.
Dimensionar el servicio con evidencia representativa de sus datos.

La campaña reproducible crea 240 plazos y consume tres eventos usando límites
1, 20 y 100. Comprueba 720 trabajos exactos y registra duración por página y
planes `EXPLAIN ANALYZE`. Los valores medidos, su entorno y sus límites se
reportan en [verificación](verification-report.md), separados de la corrección
funcional. No son una garantía de rendimiento de producción.

## Migración y operación

Aplicar `database migrate --runtime-role` mediante credenciales administrativas,
con escritores detenidos y respaldo coherente, como indica
[operación de la base](database-operations.md). La migración crea el singleton
sólo al crear su tabla. Si una tabla existente perdió su cursor, repetir la
migración conserva el fallo y el arranque lo rechaza: no inventa una posición.

El rol operativo puede leer y añadir trabajos y actualizar únicamente las cuatro
posiciones del cursor. No puede reescribir trabajos, cambiar identidad, borrar,
truncar, modificar triggers ni ejecutar directamente funciones de guard.
La apertura comprueba columnas, restricciones, índices, funciones, triggers,
permisos efectivos e inventario. También rechaza privilegios excesivos por
PUBLIC o pertenencia a roles.

El inventario autentica eventos, raíces, ámbitos, operaciones y posiciones.
No exige que la dependencia de un trabajo histórico siga seleccionada en la
cabeza actual: una corrección o retiro posterior es legítimo. Respaldar juntos
eventos, trabajos, cursores, plazos, dependencias y auditoría. Nunca reparar un
cursor perdido eliminando trabajos ni volver a crear progreso supuesto.

Las migraciones `0020_` amplían las reservas y el inventario para admitir
únicamente la revisión técnica y el resultado del trabajo exacto, con su causa
autenticada. La operación reservada no puede reutilizarse desde el puerto humano.
El consumidor conserva cierre, retiro y cambios humanos posteriores; sus
resultados, intentos y límites de recuperación están en
[el contrato del trabajador](deadline-worker.md). Su verificación es independiente
de las campañas históricas del despachador.
