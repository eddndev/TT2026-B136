# Informes de estado de expedientes

## Estado y alcance

Implementación local de aplicación, persistencia, protección, renderizado, HTTP,
interfaz y composición supervisada en el servidor. La aceptación integrada y el
cierre global siguen pendientes; esta descripción no acredita un despliegue ni
una campaña global aprobados. Véase la
[decisión de diseño](adr/0060-durable-authorized-case-reports.md).

Una solicitud produce un PDF y un CSV del **estado administrativo observado** de
los expedientes creados en el intervalo elegido, más la carga derivada de esos
mismos expedientes. El intervalo filtra la creación original: no reconstruye el
estado al final del periodo, la actividad histórica ni la efectividad jurídica.
No genera firmas documentales, constancias de PSC ni notificaciones por correo.

## Autorización y captura

Todas las rutas requieren una sesión bearer vigente. Sólo Owner y Litigator
pueden usarlas; Paralegal y Client quedan excluidos. El alcance es `office` para
Owner y `assigned_cases` para Litigator. Cada informe pertenece exclusivamente
a quien lo solicitó: tampoco otro Owner puede consultarlo o descargarlo.

La solicitud conserva identidad completa, revisión de cuenta y generación de
autorización. Cada acceso exige la misma identidad y generación; la revisión
actual debe ser mayor o igual que la guardada. Consumir un código de recuperación
puede aumentar esa revisión sin revocar el informe. Cambiar y restaurar permisos
no elude la comprobación de generación. La revisión guardada no se reemplaza.

La captura conserva filtros, observación, expedientes, revisiones administrativas,
asignaciones y resumen de carga. Antes de publicarla o entregarla se revalida el
acceso a **todos** los expedientes capturados; una pérdida de acceso impide entregar
el conjunto, sin recortarlo silenciosamente. El listado omite informes ajenos o
inaccesibles. La consulta directa de uno ajeno responde `404`; una captura propia
cuyo acceso fue revocado responde `403`.

Captura y artefactos se cifran con protección autenticada ligada al identificador,
tipo de contenido y digest. PDF y CSV proceden de la misma captura inmutable,
incluso al reintentar el renderizado. Los dos se publican juntos con su aviso;
una descarga no vuelve a consultar casos para fabricar otro resultado.

## Rutas y entrada

| Método y ruta | Resultado |
| --- | --- |
| `POST /api/v1/case-reports` | Solicita o recupera por operación: `202` mientras está pendiente; `200` si la operación ya tiene estado terminal. |
| `GET /api/v1/case-reports` | Página de informes propios autorizados, `200`. |
| `GET /api/v1/case-reports/litigators` | Página de litigantes activos autorizados para el filtro, `200`. |
| `GET /api/v1/case-reports/:id` | Detalle vigente de la solicitud, `200`. |
| `POST /api/v1/case-reports/:id/notice-read` | Acuse explícito, cuerpo `{}`, devuelve el detalle, `200`. |
| `GET /api/v1/case-reports/:id/download?format=pdf` | PDF exacto ya publicado, `200`; `format=csv` selecciona CSV. |

`HEAD` de descarga responde `405`. Los cuerpos JSON de solicitud y acuse admiten
como máximo 4096 bytes. Campos o parámetros desconocidos se rechazan; detalle,
solicitud y acuse no admiten parámetros de consulta. Los UUID deben ser no nulos.

```json
{
  "operation_id": "82000000-0000-4000-8000-000000000001",
  "filters": {
    "created_from": "2026-09-01T00:00:00Z",
    "created_before": "2026-10-01T00:00:00Z",
    "status": "all",
    "assigned_litigator": null
  }
}
```

Los instantes son RFC 3339 con desplazamiento UTC cero y año entre 1 y 9999.
El intervalo incluye `created_from`, excluye `created_before`, tiene duración
positiva y no supera 366 días. HTTP admite instantes UTC; el formulario convierte
sus fechas civiles a las 00:00 UTC. `status` es `all`, `active` o `closed`.
`assigned_litigator` es opcional o nulo; si se proporciona, debe identificar un
litigante activo visible para el solicitante. El filtro limita expedientes, no
elimina del resumen a otros litigantes asignados a esos mismos expedientes.

Repetir la misma operación y filtros devuelve la solicitud original. Cambiarlos
con el mismo `operation_id` produce conflicto. Ante respuesta incierta, el cliente
conserva la operación para un reintento explícito; no crea solicitudes automáticas.

El listado admite `limit` entre 1 y 100 (20 predeterminado), `after_id` como UUID
y `unread_only=true|false` (false predeterminado). Ordena por identificador y devuelve
`{checked_at,reports,has_more,next_after_id}`. El cursor sólo se emite si hay otra
página. Una página no representa un conteo total del conjunto.

## Detalle, progreso y avisos

Cada detalle contiene `id`, `operation_id`, `request_digest`, `scope`, `filters`,
`requested_at`, `updated_at`, `state`, `phase`, `retry_at`, `failure`, `ready` y
`notice`. Digests: SHA-256 hexadecimal; instantes: RFC 3339 UTC. Los campos no
aplicables se serializan como `null`, según esta tabla:

| `state` | `phase` | `retry_at` | `failure` | `ready` |
| --- | --- | --- | --- | --- |
| `queued` | null | null | null | null |
| `processing` | `capturing` o `rendering` | null | null | null |
| `retry_waiting` | `capturing` o `rendering` | instante previsto | null | null |
| `ready` | null | null | null | metadatos de ambos artefactos |
| `failed` | null | null | motivo | null |
| `access_revoked` | null | null | `access_revoked` | null |

Los motivos serializados son `temporarily_unavailable`, `render_unavailable`,
`render_failed`, `capacity_exceeded`, `invalid_capture` y `access_revoked`.
`access_revoked` forma parte del modelo; no habilita la lectura de una captura
revocada. `retry_waiting` conserva fase y fecha, sin presentar un fallo terminal.
La interfaz consulta el progreso por acción explícita, sin porcentajes inventados.

`ready` contiene `snapshot_digest`, `checked_at` y `artifacts`, con dos entradas
`{format,bytes,digest}` para `pdf` y `csv`. `notice` es nulo o
`{kind,created_at,read_at}`, donde `kind` es `ready` o `failed`. Abrir una pantalla o
descargar no marca el aviso como leído. El acuse guarda el primer `read_at` y es
idempotente; sin aviso disponible responde conflicto. Los avisos son internos,
persistentes y distintos de las alertas de plazos o audiencias.

## Descargas y límites

Las respuestas binarias incluyen `Content-Disposition: attachment` con nombre
`report-<id>.pdf` o `.csv`, `Content-Length`, `Cache-Control: no-store`,
`X-Content-Type-Options: nosniff`, `X-Report-Id`, `X-Report-Digest` y
`X-Report-Snapshot-Digest`. Sus tipos son `application/pdf` y
`text/csv; charset=utf-8`. El cliente verifica tamaño, digest e identidad contra
el detalle autorizado antes de ofrecer el archivo. La API no acepta plantillas.

Cada captura admite hasta 1000 expedientes, 10000 pares de asignación y 1000
litigantes en el resumen; su representación canónica no supera 8 MiB. Cada PDF
o CSV admite hasta 16 MiB. Superar los límites falla: no se omiten filas para
aparentar un resultado completo. Conviene reducir los filtros y solicitar de nuevo.

El PDF pagina texto latino/español con fuentes incorporadas; glifos o escrituras
no soportados producen error, sin sustituciones silenciosas. El CSV usa UTF-8,
registros RFC 4180 y un esquema uniforme con `row_type` igual a `capture`, `case`
o `workload`. La fila de captura existe aun sin resultados. Los textos no
confiables llevan apóstrofo inicial para impedir evaluación de fórmulas.

## Selector de litigantes

`GET /api/v1/case-reports/litigators` admite `limit` entre 1 y 100, con 20 como
predeterminado, y `after_id` como UUID no nulo. Rechaza parámetros desconocidos.
Ordena por identificador y devuelve:

```json
{
  "scope": "assigned_cases",
  "checked_at": "2026-10-01T12:00:00Z",
  "litigators": [
    {
      "user_id": "82000000-0000-4000-8000-000000000002",
      "email": "litigante@example.test"
    }
  ],
  "has_more": false,
  "next_after_id": null
}
```

Owner ve todos los litigantes activos, incluso con cero expedientes. Litigator
ve a los litigantes activos con quienes comparte algún expediente, **incluidos
los cerrados**; no se exige compartir un expediente activo. Se usa el mismo
criterio que valida `assigned_litigator` al solicitar el informe. Paralegal y
Client quedan excluidos incluso si la página sería vacía. Cada página revalida
identidad y acceso, registra la lectura y contiene sólo identificador y correo.

El selector es independiente del agregado `workload` del tablero. El intervalo
y el estado seleccionados para el informe pueden producir cero resultados para
un litigante visible. La interfaz carga páginas adicionales por acción explícita;
no presenta una página como directorio completo ni como total de litigantes.
El cursor es el último identificador devuelto sólo cuando `has_more` es verdadero.

## Procesamiento y aislamiento

`serve` compone un consumidor supervisado de la cola persistente. Una solicitud
no ejecuta el renderizado dentro de la transacción HTTP. Los intentos conservan
su captura al reintentar; una concesión vencida no permite que un proceso anterior
publique sobre un intento nuevo. Sólo la publicación conjunta de PDF y CSV
habilita la descarga y el aviso de disponibilidad.

En Linux, cada formato se produce en un proceso hijo del ejecutable confiable:
15 segundos de CPU, 512 MiB de espacio virtual y 20 segundos de tiempo de pared,
con salida limitada al artefacto de 16 MiB y su cabecera de protocolo. La entrada
es una captura en memoria sellada; el hijo no recibe variables de entorno ni
los descriptores abiertos de los servicios. El proceso termina al exceder el
presupuesto o fallar, sin habilitar archivos parciales. Esta separación limita
recursos; no constituye un aislamiento adicional de permisos o red del sistema.
Véase [la operación de la cola y restauración](database-operations.md#informes-durables-y-restauración).

El PDF admite como máximo 512 páginas y un millón de glifos. Los límites de
expedientes, asignaciones, tamaño y renderizado se aplican conjuntamente; alcanzar
el máximo de filas no garantiza que cualquier anchura de texto quepa dentro de
los demás presupuestos. La aceptación debe medir también esos casos límite.

## Errores

Además de los errores de autenticación y permisos, se distinguen
`400 invalid_case_report_request`, `413 case_report_request_too_large`,
`404 case_report_not_found`, `409 case_report_operation_conflict`,
`409 case_report_not_ready`, `422 case_report_capacity_exceeded`,
`403 case_report_access_revoked`, `503 case_report_render_unavailable` y
`500 case_report_render_failed`. Un trabajo asíncrono fallido puede devolver
`200` con `state=failed`: ese estado no equivale a una descarga disponible.
