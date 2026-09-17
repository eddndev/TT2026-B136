# Resolución autorizada de insumos temporales

El componente conecta fuentes persistidas con la [extracción exacta](deadline-triggers.md)
y la [aritmética](deadline-arithmetic.md). Resuelve autorización, recibos y revisiones;
los perfiles normativos y las evaluaciones operativas mantienen su desarrollo pendiente.
La decisión está en [ADR-0034](adr/0034-authorized-deadline-input-resolution.md).

## Servicio, puerto y comprobador

`DeadlineInputService::prepare` recibe sesión y `DeadlineInputRequest`: selección
exacta, exigencia temporal, regla aritmética y referencia opcional de calendario.
El solicitante no proporciona material resuelto ni sus huellas.

`DeadlineInputStore::load` resuelve administración, fuente exacta y cabeza actual,
calendario exacto y cabeza actual. `check_deadline_inputs` comprueba el conjunto
antes de invocar el coordinador de dominio. Sus errores de coherencia y de recibos
se presentan como `DeadlineInputError::Inconsistent`; los errores del puerto
conservan su tipo, incluidos los resultados no encontrados.

`PreparedDeadlineInputs` conserva actor, solicitud, material y cálculo mediante
getters. Su construcción queda reservada al servicio. Es un resultado en memoria,
sin reserva de fuentes ni autorización para una escritura futura.

## Historia y cabezas observadas

Fuente elegida y cabeza deben tener recibos válidos y pertenecer a la misma
identidad, con revisión de cabeza igual o posterior. Si comparten revisión,
los detalles completos deben coincidir, incluidas proyecciones fuera del digest.

Una notificación posterior puede seleccionar otra revisión de la misma resolución
padre. Un resultado posterior puede eliminar el acuerdo elegido históricamente;
la extracción utiliza la revisión seleccionada y comprueba allí la pertenencia
del acuerdo, incluido un identificador UUID cero explícito.

El calendario conserva revisión exacta y cabeza, con identidad, orden, ámbito
inmutable y recibos verificados. Solo los valores seleccionados alimentan el
cálculo. Estados y cabezas observadas se conservan sin inferir elegibilidad ni
sustituir automáticamente las referencias históricas por las actuales.

## Coherencia y bloqueos

El expediente del material coincide con la solicitud. La administración registrada
valida identidad y CADM1 directamente, sin aplicar la prohibición de escritura en
expedientes cerrados. La administración sin revisiones permanece sin revisión o
autor fabricados; el adaptador demuestra su asociación al expediente.

Fuente y cabeza existen juntas para una selección conocida, y ambas están ausentes
para una desconocida. El calendario seleccionado también exige sus dos detalles.
Una fuente desconocida no oculta material adicional o un calendario corrupto.
Un calendario aportado se verifica aunque la regla aritmética no lo necesite.

Los bloqueos de extracción y aritmética se conservan como resultados válidos.
No se completan horas, reinterpretan textos, eligen reglas por el nombre de una
autoridad ni acredita aplicabilidad jurídica por obtener una candidata matemática.

## Autorización y auditoría

`ReadDeadlineInputs` permite Owner, Litigator y Paralegal, y deniega Client.
El servicio autentica antes del puerto. PostgreSQL revalida usuario activo,
permiso y expediente dentro de una transacción auditada: Owner accede a todos;
el resto del personal necesita asignación. Esto rige también sin fuente conocida,
sin calendario o con un calendario global como único insumo seleccionado.
Los expedientes cerrados conservan lectura autorizada.

`PostgresDeadlineInputStore::open` recibe conexión, hasher y reloj. Los cargadores
internos resuelven fuentes, ascendencia y recibos dentro de la misma transacción;
no se llaman operaciones públicas que abrirían otra lectura auditada. El conjunto
se valida antes del evento `deadline.inputs_read`, recurso
`case:<uuid>:deadline_inputs`, con reloj capturado bajo bloqueo. No hay nuevas
tablas, migración, readmisión documental ni escritura de evaluaciones.

Los fallos de autorización, carga o comprobación dentro de la transacción no
confirman el evento. La consulta de una fuente ajena no revela su existencia.
La auditoría sí forma parte de esta lectura: no es una conexión SQL de solo lectura.

Antes de entregar la preparación el servicio vuelve a autenticar. Exige el mismo
actor y rol: sesión inválida propaga su error; un rol denegado produce
`PermissionDenied`; otro actor o rol permitido produce `InvalidSession`.
PostgreSQL y Redis no comparten esta transacción. Si falla la segunda autenticación,
se impide entregar el resultado pero permanece el evento que el puerto confirmó.
Una revocación posterior no revierte bytes ya entregados por una lectura anterior.

## Evidencia y alcance pendiente

Las pruebas de [material](../crates/application/tests/deadline_input_material.rs),
[cabezas](../crates/application/tests/deadline_input_heads.rs),
[servicio](../crates/application/tests/deadline_input_service.rs),
[backend](../crates/infrastructure/tests/deadline_input_backend.rs) y
[permisos](../crates/infrastructure/tests/deadline_input_permissions.rs) distinguen
validación pura, autenticación y datos reales. Los resultados están en
[el informe de verificación](verification-report.md).

El comprobador puro no acredita existencia ni acceso por sí mismo. Faltan perfiles
aplicables, calificación completa, evaluación persistida, atención, reevaluación,
alertas y HTTP/Qadra de plazos. La [investigación normativa](deadline-rule-research.md)
y la [matriz funcional](product-completion.md) conservan ese alcance.
