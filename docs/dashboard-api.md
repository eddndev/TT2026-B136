# Tablero operativo

## Alcance

`GET /api/v1/dashboard` devuelve una instantánea del despacho para Owner o de
los expedientes asignados para Litigator. Paralegal y Client no pueden consultar
este agregado. No admite filtros ni un ámbito elegido por el navegador.

La lectura conserva el bloqueo de consistencia de las operaciones auditadas,
vuelve a comprobar cuenta y asignaciones en PostgreSQL y verifica de nuevo la
sesión antes de devolver datos. El evento `dashboard.read` pertenece a la misma
transacción que la consulta. Una lectura fallida no entrega totales parciales.

## Respuesta

Todas las respuestas conservan `Cache-Control: no-store`. El objeto contiene:

| Campo | Significado |
| --- | --- |
| `checked_at` | Instante UTC RFC 3339 común a toda la instantánea. |
| `scope` | `office` o `assigned_cases`, determinado por el servidor. |
| `active_cases` | Expedientes autorizados administrativamente activos. |
| `pending_contracts` | Documentos cuya última versión no tiene sello y cuyo tipo actual es `contrato` o `contract`, sin distinguir mayúsculas ASCII. |
| `deadlines_overdue` | Plazos vigentes no atendidos con vencimiento anterior al instante consultado. |
| `deadlines_due_48h` | Vencimientos desde el instante consultado, incluido, hasta 48 horas después, excluido. |
| `deadlines_due_7d` | La misma ventana hasta siete días, incluidos los próximos a 48 horas. |
| `deadlines_unresolved` | Plazos activos no atendidos sin fecha operativa comprobada. |
| `workload` | Lista ordenada por `user_id` con correo y número de expedientes activos asignados por litigante activo. |

El conteo documental usa versiones y clasificación actuales, no nombres de
archivo ni versiones históricas selladas. Incluye expedientes cerrados que el
actor todavía puede consultar. "Pendiente de sello" describe la evidencia
interna del prototipo; no acredita ausencia de firmas externas ni firma personal.

Owner ve a todos los litigantes activos, incluidos quienes no tienen casos.
Litigator sólo ve colegas que comparten sus expedientes activos autorizados;
los totales de cada colega se limitan a esos expedientes. No expone el directorio
ni la carga de trabajo ajena a ese conjunto.

## Vigencia y urgencia

Cada plazo se reconstruye y comprueba contra sus fuentes actuales. Una fecha
histórica no sustituye un resultado bloqueado o pendiente de revisión. Los
plazos retirados o atendidos no cuentan como trabajo pendiente. El cierre
administrativo de un expediente no suspende sus términos: sus plazos todavía
participan en los indicadores. Las ventanas son duraciones exactas en UTC,
no días hábiles ni un nuevo cálculo jurídico.

La interfaz distingue visualmente vencidos y próximos a 48 horas y conserva un
contador separado de revisiones pendientes. La urgencia temporal no asigna una
naturaleza jurídica "fatal" que el catálogo actual no clasifica.

## Límites de lectura

La instantánea admite hasta 1000 expedientes autorizados, 10000 documentos,
1000 plazos y 1000 litigantes visibles. Si se supera un límite, falla la lectura
completa; no devuelve un contador parcial ni omite fuentes silenciosamente.

## Interfaz y aceptación

Qadra consulta el tablero al abrir Inicio y permite actualizarlo explícitamente.
Durante carga o fallo no presenta ceros como resultados comprobados. Descarta
respuestas de una sesión o componente que ya terminó. Mantiene accesos de
navegación para las otras cuentas sin consultar agregados no autorizados.

La evidencia nueva de esta entrega se registra en el informe de verificación;
las mediciones de CI anteriores sólo describen su propia revisión.
