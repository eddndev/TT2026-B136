# Consulta de actividad registrada

## Estado y alcance

Entrega acotada de Auditoría con aceptación local completa: 20 pruebas de
aplicación, 14 PostgreSQL en 37.01 s, 6 HTTP y una del límite compartido,
12 del cliente y 9 escenarios de navegador controlado en 17.1 s. La revisión
visual de escritorio a 1440 px y móvil a 390 px aprobó; Clippy del workspace y
todos los targets aprobó en 2m12s.

La campaña API con respaldo/restauración terminó con salida cero en 376.459967 s.
Los dos recorridos con servicios reales aprobaron: Owner 6.2 s y Litigator 2.4 s,
con 13.7 s de Playwright y 185.7532 s del comando completo. En cada campaña los
3198 archivos del inventario fuente permanecieron idénticos. CI de esta entrega,
integración y activación siguen pendientes; estos tiempos no miden latencia
por registro ni acreditan un despliegue.

La consulta presenta eventos ya registrados. No modifica sus datos, no atribuye
una dirección IP histórica ni convierte el texto del actor en una identidad
UUID comprobada. La operación independiente **Verificar cadena** conserva su
alcance y sus limitaciones. Véase [la decisión de diseño](adr/0061-bounded-owner-audit-query.md).

## Acceso y filtros

`GET /api/v1/audit/events` exige una sesión bearer vigente de Owner. Litigator,
Paralegal y Client no pueden consultar la bitácora global. Cada página valida el
Owner activo persistido y vuelve a comprobar la identidad completa antes de
entregar datos, incluso cuando no hay resultados.

| Parámetro | Contrato |
| --- | --- |
| `from` | Instante RFC3339 obligatorio, incluido en el intervalo. |
| `until` | Instante RFC3339 obligatorio, excluido del intervalo. |
| `actor` | Texto registrado exacto, opcional; 1..254 bytes UTF-8. |
| `action` | Operación exacta, opcional; 1..128 bytes UTF-8. |
| `resource` | Recurso registrado exacto, opcional; 1..1024 bytes UTF-8. |
| `limit` | Entero decimal canónico 1..100; 20 por omisión. |
| `cursor` | Continuación opaca para el cliente, opcional; ASCII de hasta 4096 bytes. |

El intervalo debe ser positivo, durar como máximo 366 días exactos y mantener
sus extremos en años UTC 1..9999. Los instantes se normalizan a UTC sin perder
nanosegundos. Los filtros distinguen mayúsculas y conservan espacios; no admiten
vacíos ni caracteres de control. Omitir un filtro selecciona todos sus valores,
pero enviar una cadena vacía es inválido. Parámetros desconocidos, duplicados o
malformados se rechazan; la consulta codificada no puede superar 16 KiB. La sesión y el límite compartido de solicitudes se
aplican como en las demás rutas privadas; la respuesta usa `Cache-Control: no-store`.

```http
GET /api/v1/audit/events?from=2026-10-01T00%3A00%3A00Z&until=2026-10-02T00%3A00%3A00Z&action=document.sealed&limit=20
Authorization: Bearer <sesion-privada>
```

## Respuesta y continuación

```json
{
  "checked_at": "2026-10-02T10:20:30.123456789Z",
  "snapshot_max_sequence": "120",
  "events": [
    {
      "sequence": "116",
      "timestamp": "2026-10-01T15:45:00.123456789Z",
      "actor": "owner@example.test",
      "action": "document.sealed",
      "resource": "document-reference"
    }
  ],
  "has_more": false,
  "next_cursor": null
}
```

`sequence` y `snapshot_max_sequence` son cadenas decimales canónicas entre
`"0"` y `"9223372036854775807"`; no números JSON sujetos al redondeo de JavaScript.
`snapshot_max_sequence` es `null` si la bitácora estaba vacía al iniciar la
consulta. Los valores del ejemplo son ilustrativos, no evidencia de una acción.

Los eventos están ordenados ascendentemente por instante exacto y, cuando
coincide, por secuencia. La secuencia puede no aumentar entre dos instantes:
el orden global de anexado y el orden cronológico son conceptos distintos.
`checked_at` indica la observación autorizada, no la fecha de verificación
criptográfica de cada registro.

La primera página fija la secuencia máxima existente antes de registrar su
propia consulta. Un cursor mantiene esa secuencia, el último instante/secuencia
y los filtros exactos. Los eventos añadidos después quedan excluidos incluso
si llevan una fecha anterior; **Actualizar** inicia otra consulta para verlos.
El cursor no es una autorización ni una firma. No se debe construir, modificar
ni interpretar en la interfaz: se reenvía literalmente con los mismos filtros.
Puede cambiar únicamente `limit` entre páginas.

`has_more=true` exige otra página y un `next_cursor` no nulo. La ausencia de
continuación indica que terminó esta selección, no que se hayan consultado todos
los eventos de la bitácora. No se devuelve un total global inventado. Cambiar
filtros o cerrar sesión descarta las páginas y continuaciones anteriores. La interfaz
mantiene una sola página: **Cargar siguiente página** reemplaza la anterior para
acotar los datos retenidos en el navegador; no calcula un total a partir de ella.

## Límites y errores

La página contiene como máximo 100 eventos completos y 262144 bytes UTF-8 de
texto combinado de actor, operación y recurso. Los campos históricos no se
recortan para hacerlos caber ni se someten a los límites de entrada de filtros.
Si el conjunto elegido excede esa capacidad, se rechaza completo con `413` y
`audit_query_capacity_exceeded`; el usuario puede reducir `limit` o concretar
los filtros. Un único evento demasiado grande no se vuelve consultable por
truncarlo. El escape JSON puede ampliar el tamaño transmitido, pero sigue
acotado por este presupuesto y por el máximo de metadatos/cursor.

La entrada inválida produce `400`; una sesión ausente o revocada, `401`; un rol
sin permiso, `403`. Los errores de persistencia o de invariantes no entregan una
página parcial. La consulta válida se registra como `audit.events_read` dentro
de su transacción. Una revocación detectada después de esa transacción impide
revelar los datos, aunque el evento de lectura ya registrado permanezca.

La consulta filtrada no verifica por sí misma la cadena completa, no detecta el
truncamiento del final mediante un ancla externa y no acredita tiempos de anexado
inferiores a 500 ms. La consulta de IP e identidad UUID histórica fiable sigue
pendiente; véase [la limitación de anclaje](adr/0007-audit-chain-anchoring.md).
