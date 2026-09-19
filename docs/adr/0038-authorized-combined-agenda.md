# Agenda combinada con fechas operativas verificadas

## Status

Accepted.

## Context

La programación de audiencias conserva su fecha declarada. Un plazo conserva
su cálculo histórico incluso cuando una dependencia cambió y exige revisión.
Reunir ambos listados en el navegador podría mostrar esa fecha histórica como
vigente, aplicar límites antes de autorización o mezclar observaciones tomadas
en transacciones independientes.

## Decision

Un puerto de aplicación `AgendaStore` reúne las dos familias. PostgreSQL carga
la última revisión autorizada de cada raíz bajo una sola transacción y el
cerrojo común de auditoría. Revalida actor, membresías y administración; comparte
con la lectura actual de plazos la resolución verificada de dependencias.
El servicio comprueba nuevamente la identidad completa antes de responder.

Un plazo entra únicamente con recibo V2, seguimiento aceptado, estado activo,
dependencias vigentes y fecha operativa vinculada a su captura. El cálculo
histórico sirve como prefiltro SQL; nunca autoriza por sí solo la inclusión.
El cierre administrativo y una atención declarada conservan lecturas permitidas.

Cada página verifica como máximo 100 candidatos. El orden conserva segundo,
nanosegundo, familia e identificador; audiencias preceden a plazos empatados.
El cursor exclusivo avanza hasta el último candidato examinado, incluso si fue
omitido. Una página vacía puede ser parcial. La continuación está ligada al
intervalo y filtros y no concede permisos. Véase [el contrato](../agenda-api.md).

Qadra muestra día, semana y mes, además del rango personalizado, en un desfase
UTC explícito. Acumula páginas bajo demanda, identifica actividades por familia
e identificador y conserva la revisión más reciente recibida. Abrir una tarjeta
revalida el expediente y consulta la revisión exacta mostrada.

## Consequences

La consulta no depende de que el consumidor durable haya vaciado su cola y no
calcula fechas sustitutas. Usa el presupuesto HTTP y los límites PostgreSQL
existentes; el adaptador síncrono se conserva fuera del runtime asíncrono.

Cada página representa una observación distinta. Cambios concurrentes pueden
mover actividades entre páginas; actualizar reinicia la consulta. No hay un
snapshot persistente ni totales globales. El límite de candidatos acota su
verificación, no el costo total de seleccionar y ordenar encabezados SQL.

La agenda no activa plazos ni entrega alertas. No determina aplicabilidad
jurídica, zona horaria jurisdiccional ni efectos de una notificación.
