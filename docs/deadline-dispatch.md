# Despacho persistente de reevaluación de plazos

## Estado y frontera

El contrato `crates/application/src/deadline_dispatch/` y el adaptador
`PostgresDeadlineDispatchStore` expanden cambios de dependencias en trabajos
persistentes. Cada llamada procesa una página y confirma sus trabajos, el
avance y la auditoría en una transacción. Las migraciones `0019_` y
`crates/infrastructure/src/deadline_dispatch_schema/` definen y verifican esta
frontera. La decisión general está en
[ADR 0037](adr/0037-durable-deadline-reevaluation.md).

El trabajador que consume esos trabajos y confirma revisiones técnicas sigue
pendiente, junto con su servicio, HTTP y Qadra V2. El despachador todavía no
se ejecuta desde `serve` ni expone una ruta HTTP o un comando CLI. Los eventos
persistidos no significan que un plazo ya haya sido recalculado o notificado.

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
históricos se conservan y el futuro consumidor debe decidir sobre la base actual.

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
sentencia antes de iniciar la transacción de despacho. Son presupuestos de
sentencia, no un límite global de tiempo para toda la página o para su apertura.
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

Antes de habilitar el consumidor técnico deben ampliarse las reservas y el
inventario para admitir únicamente la revisión y resultado del trabajo exacto.
El guard humano actual sigue rechazando las escrituras técnicas.
