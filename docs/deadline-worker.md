# Consumidor durable de reevaluacion de plazos

## Estado y frontera

`PostgresDeadlineWorkerStore` consume los trabajos del
[despachador](deadline-dispatch.md). Confirma revisiones tecnicas, resultados
sin cambios e intentos fallidos mediante PostgreSQL. Las operaciones del puerto
son internas; no reciben un usuario ni permiten que un comando HTTP humano
elija autor tecnico. El contrato esta en
`crates/application/src/deadline_worker/` y la decision arquitectonica en
[ADR 0037](adr/0037-durable-deadline-reevaluation.md).

Este adaptador todavia no se ejecuta desde `serve`. La composicion operativa,
el servicio humano V2, HTTP y Qadra V2 siguen pendientes. Tampoco implementa
activacion de plazos, agenda conjunta, alertas o calificacion juridica de
perfiles. Sus pruebas y las del flujo HTTP V1 tienen evidencias separadas en
[el informe de verificacion](verification-report.md).

La restauración real y las comprobaciones focales de catálogo, permisos y
clasificación de fallos aprobaron. La regresión integral aprobó con los resultados
registrados en el informe de verificación; conserva su alcance separado de
la futura integración HTTP/Qadra V2.

## Puerto y ejecución de un trabajo

`DeadlineWorkerStore` ofrece `run_next()`, `result(job_id)` y
`latest_attempt(job_id)`. La lectura por UUID de intento es un helper interno
para inventario y conciliación; el puerto público no ofrece aún un listado de
intentos. Estas lecturas internas verifican evidencia sin añadir eventos de
consulta, aunque usan la misma frontera transaccional de auditoría.

`run_next()` devuelve uno de estos resultados:

| Resultado | Significado |
| --- | --- |
| `Idle` | No hay un trabajo elegible en ese instante. Puede haber trabajos esperando reintento; no afirma que todo el seguimiento este actualizado. |
| `Completed` | Existe una confirmacion durable y verificada: revision nueva o motivo sin cambios. |
| `Deferred` | La ejecucion fallo y se confirmaron un intento durable y su auditoria. El trabajo sigue pendiente. |
| Error | No se pudo obtener un resultado verificable o registrar el fallo. No equivale a finalizacion ni autoriza a inventar un recibo. |

Se elige el trabajo elegible más antiguo por segundos, nanosegundos de creación
y UUID. Los completados quedan fuera; los pendientes respetan su espera salvo
cuando la cabeza del plazo dejó atrás la base comprobada del intento.

El consumidor comparte el bloqueo transaccional de auditoria con las
decisiones humanas y las mutaciones de fuentes. Usa `READ COMMITTED`, elige un
solo trabajo elegible y verifica su identidad, expediente, operacion, evento
y revision de origen. La secuencia de un evento o una huella aislada no
demuestran su procedencia. Los eventos abarcan resolucion, notificacion,
resultado de audiencia, calendario y perfil; el legado usa causa propia.

Carga la cabeza verificada del plazo y conserva las decisiones humanas que
ya se hubieran confirmado. El cierre administrativo o la baja del responsable
no son filtros de seleccion tecnica. Un plazo retirado produce un resultado
terminal sin otra revision. Un legado ya inicializado tampoco requiere
fabricar cabezas para resolver su resultado sin cambios.

Para las otras rutas, el consumidor carga perfiles, fuentes, calendarios y
padre independiente de notificacion mediante los lectores existentes. El
preparador puro conserva las politicas y calificaciones, y determina si
corresponde revisar o recalcular. La evaluacion historica no se sustituye por
una fecha manual; la revision pendiente no obtiene vencimiento operativo.

## Confirmacion y evidencia historica

Las migraciones `0020_` agregan `deadline_reevaluation_results` y
`deadline_reevaluation_attempts`. Una confirmacion referencia ambas huellas de
su base exacta. Si produce revision, referencia tambien numero y huellas de
esa revision. Su operacion y causa pertenecen al trabajo original.

La revision, el resultado y el evento `deadline.reevaluated` se confirman en
una transaccion. SQL rechaza resultados de tipo revision sin su revision; una
restriccion diferida rechaza revisiones tecnicas sin su resultado al confirmar. La
excepcion al escritor humano exige trabajo, autor, causa, predecesor,
politicas y observaciones correspondientes. Los historiales siguen siendo
inmutables y las operaciones de los trabajos no quedan disponibles para
escrituras humanas.

Los resultados sin cambio usan `deadline.reevaluation_no_change`. `Retired`
y `AlreadyInitialized` conservan solamente la base y causa que justifican la
decision. `AlreadyObserved` y `DependencyNotSelected` conservan DLOB1 y el
compromiso completo de administracion, incluidos sus metadatos historicos.
Pueden examinar otras cabezas posteriores a las observadas por la base; no
se exige que ambos manifiestos sean iguales artificialmente.

`result(job_id)` devuelve ausencia cuando no hay una confirmación guardada;
no significa éxito ni que el trabajo carezca de intentos. Cuando existe,
verifica la procedencia y reconstruye las referencias históricas exactas. Para
un resultado sin cambios, reutiliza el preparador
con los insumos capturados; no sustituye esos insumos por las cabezas de hoy.
La lectura de una revision producida conserva su aritmetica historica y
comprueba recibos, continuidad y enlace al trabajo. Los resultados siguen
consultables despues de una correccion humana posterior.

Los eventos de auditoria incluyen identidad de trabajo y operacion, base,
huellas, causa, resultado y tiempo. Las salidas sin cambios que examinaron
cabezas incluyen tambien los bytes de observaciones y el compromiso administrativo. El rol de
ejecucion no puede modificar o eliminar resultados o intentos. Esta frontera
no convierte al poseedor de credenciales administrativas de PostgreSQL en un
actor no confiable contenido por los permisos de runtime.

## Fallos, espera y recuperacion

Una ejecucion fallida se revierte antes de intentar registrar su fallo en una
nueva transaccion. Primero se consulta el resultado por identidad de trabajo:
si otra ejecucion ya confirmo o se perdio una respuesta de confirmacion, se
devuelve ese resultado sin crear otra revision o un intento innecesario.

Los intentos tienen UUID propio, numero positivo consecutivo, categoria,
codigo estable, base comprobada opcional, tiempo de fallo y proximo reintento.
La base solo se declara comprobada despues de verificar su recibo. La lectura
de un intento no certifica una causa que precisamente pudo haber fallado.
El último intento sigue consultable después de completar el trabajo. El
inventario verifica también todos los intentos anteriores mediante su lector
interno; no se eliminan del historial.

Los fallos transitorios esperan 1, 2, 4 y sucesivos segundos hasta un maximo de
300, contando los intentos de la misma base. Los fallos de evidencia o trabajo
inconsistentes esperan 3600 segundos. Un avance de la cabeza del plazo deja
sin efecto la espera asociada a una base anterior; el consumidor carga y
respeta la decision humana nueva. Otros trabajos elegibles pueden avanzar.

El contrato distingue evidencia o trabajo inconsistente de fallos transitorios.
`ApplicationError::ClassifiedPort` conserva las categorías neutrales `Busy`,
`Interrupted` y `Unavailable`. El adaptador las obtiene del error nativo antes
de formatearlo; el consumidor guarda respectivamente `LockUnavailable`,
`TransactionInterrupted` y `DatabaseUnavailable`. Los lectores históricos
conservan estas categorías en lugar de convertir un fallo de acceso en evidencia
inconsistente. HTTP mantiene su respuesta interna opaca.

Los errores explícitos de integridad de perfiles, calendarios, hechos, audiencias,
resultados y administración se clasifican como inconsistentes. El código distingue
un trabajo cuya autenticación falló de una dependencia examinada después.
`ExecutionFailed` conserva la categoría genérica cuando no hay información tipada
suficiente. No se deduce SQLSTATE analizando frases ni se copian diagnósticos de
base de datos al código estable o al evento de auditoría.

El intento y `deadline.reevaluation_deferred` se confirman juntos. Si su
respuesta es incierta, se consulta el mismo UUID de intento y el resultado
del trabajo antes de informar exito. Si la conexion no permite confirmar ni
reconciliar, se devuelve error; no se simula un registro durable.

## Apertura, permisos y limites

La apertura verifica esquema, funciones, restricciones, indices, triggers,
permisos y todo el inventario. Comprueba cada trabajo, cada resultado y cada
intento historico, no solo el ultimo. Detecta revisiones tecnicas huerfanas,
operaciones reservadas indebidamente usadas y huecos en la numeracion.
No exige que una revision historica coincida con las cabezas actuales.

Una conexion que se cierra debe reabrirse mediante la misma validacion. La
recuperacion entre reinicios presupone inventario valido; una inconsistencia
persistente impide abrir hasta repararla. La espera de reintentos no debilita
este control. Seguir [operacion de base de datos](database-operations.md) para
migracion y restauracion con roles separados.

El consumidor configura un segundo de espera de bloqueo y cinco segundos de
ejecucion por sentencia antes de iniciar sus transacciones. Esos valores no
acotan el tiempo total de apertura, del trabajo ni del calculo Rust. La
seleccion materializa un candidato, pero puede examinar mas trabajos
pendientes. Los limites y las mediciones se deben comunicar por separado.

## Verificación focal y límites pendientes

La comprobación focal de [catálogo](../crates/infrastructure/tests/deadline_worker_catalog.rs)
aprobó tres pruebas de funciones, restricciones y permisos. La
[recuperación](../crates/infrastructure/tests/deadline_worker_recovery.rs)
aprobó restauración real con resultado e intento pendientes y crecimiento de
reintentos hasta 300 segundos. La reapertura posterior a `pg_restore` no ejecuta
una migración reparadora y compara datos y recibos exactos antes de continuar.

La restauración detectó que `pg_dump` y su posterior parseo podían aplanar una
conjunción producida por `BETWEEN`, alterando el texto de `pg_get_expr`. Las
restricciones `0020_` usan comparaciones `>=`/`<=` explícitas para estabilizar ese
texto. El verificador conserva la comparación estricta de los once CHECKs.
No se relajaron los guards para hacer pasar la prueba.

Tres regresiones adicionales verifican un perfil posterior inconsistente y los
fallos nativos de bloqueo e interrupción durante la auditoría de confirmación.
Exigen la categoría correcta, reversión de revisión/resultado, intento durable,
espera y reapertura. La regresión global también aprobó y registra estos casos dentro de su total.
Estos resultados focales tampoco acreditan ejecución desde `serve`, transporte
HTTP/Qadra V2, tiempos máximos de toda la operación o entregas de correo.
