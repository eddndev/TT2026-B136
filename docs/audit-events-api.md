# Consulta de actividad registrada

## Estado y alcance

La consulta base de Auditoría tuvo aceptación local completa, anterior a la
extensión de contexto descrita abajo: 20 pruebas de
aplicación, 14 PostgreSQL en 37.01 s, 6 HTTP y una del límite compartido,
12 del cliente y 9 escenarios de navegador controlado en 17.1 s. La revisión
visual de escritorio a 1440 px y móvil a 390 px aprobó; Clippy del workspace y
todos los targets aprobó en 2m12s.

La campaña API con respaldo/restauración terminó con salida cero en 376.459967 s.
Los dos recorridos con servicios reales aprobaron: Owner 6.2 s y Litigator 2.4 s,
con 13.7 s de Playwright y 185.7532 s del comando completo. En cada campaña los
3198 archivos del inventario fuente permanecieron idénticos. La consulta quedó integrada por PR50 con confirmación de main, registrada en
[el estado del producto](product-completion.md). Estos tiempos no miden latencia
por registro ni acreditan un despliegue.

La consulta presenta eventos ya registrados. La extensión incorpora contexto
observado en los eventos nuevos y mantiene `context: null` en la historia que
carece de él. No modifica los datos anteriores, no atribuye una IP histórica ni
convierte el texto del actor en una identidad UUID comprobada. La operación
independiente **Verificar cadena** conserva su alcance y sus limitaciones.
Véanse [la consulta base](adr/0061-bounded-owner-audit-query.md),
[la decisión de contexto y acceso durable](adr/0077-durable-access-audit-context.md)
y [el catálogo y protocolo de aceptación](access-audit-matrix.md). Las cifras
anteriores no acreditan la aceptación de esta extensión ni su latencia durable.

## Acceso y filtros

`GET /api/v1/audit/events` exige una sesión bearer vigente de Owner. Litigator,
Paralegal y Client no pueden consultar la bitácora global. Cada página valida el
Owner activo persistido y vuelve a comprobar la identidad completa antes de
entregar datos, incluso cuando no hay resultados.

| Parámetro | Contrato |
| --- | --- |
| `from` | Instante RFC3339 obligatorio, incluido en el intervalo. |
| `until` | Instante RFC3339 obligatorio, excluido del intervalo. |
| `actor` | Texto registrado exacto o UUID exacto de `context.user_id`, opcional; 1..254 bytes UTF-8. |
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

El filtro `actor` selecciona la coincidencia exacta con el campo textual `actor`
**o** con el UUID registrado en `context.user_id`. No consulta las cuentas
actuales, no resuelve correos antiguos ni añade un parámetro de identidad. Para
buscar por identidad se envía el UUID canónico tal como aparece en el contexto;
el filtro no normaliza el texto. Un actor histórico que parezca un UUID sigue
siendo texto capturado y no obtiene por ello una identidad comprobada.

```http
GET /api/v1/audit/events?from=2026-10-01T00%3A00%3A00Z&until=2026-10-02T00%3A00%3A00Z&limit=20
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
      "resource": "document-reference",
      "context": null
    },
    {
      "sequence": "117",
      "timestamp": "2026-10-01T15:46:00.123456789Z",
      "actor": "00000000-0000-0000-0000-000000000001",
      "action": "access.granted",
      "resource": "GET /api/v1/cases",
      "context": {
        "user_id": "00000000-0000-0000-0000-000000000001",
        "ip_address": "192.0.2.4",
        "source": "http",
        "request_id": "00000000-0000-0000-0000-000000000002"
      }
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

Cada evento incluye `context`. Su valor es `null` cuando no existe contexto
registrado, o un objeto con exactamente estos cuatro campos:

| Campo de contexto | Contrato |
| --- | --- |
| `user_id` | UUID de cuenta observado tras admitir la sesión, o `null`. |
| `ip_address` | Literal IPv4 o IPv6 observado en el transporte, o `null`; no incluye puerto, zona ni nombre de host. |
| `source` | Uno de los textos exactos `http`, `cli` o `internal`. |
| `request_id` | UUID de correlación asignado por el servidor, o `null`. |

Esta representación nueva (V2) no añade un campo `version` al JSON. El contexto
presente se cubre en la cadena mediante la extensión canónica `AUDCTX01`;
el contexto ausente no añade bytes a la representación histórica. UUID, IP y
origen no se reconstruyen desde las cuentas actuales. Un objeto con origen
`cli` o `internal` y los otros campos en `null` distingue un origen registrado
de la ausencia histórica completa. El cliente rechaza campos adicionales,
orígenes desconocidos e identificadores o direcciones malformados.

La IP se toma del par de conexión por omisión. Solo la configuración explícita
de proxy local admite un `X-Forwarded-For` con una única IP literal recibido de
un par loopback; Nginx sobrescribe ese encabezado. Los valores ausentes,
duplicados o inválidos conservan la IP del par. Esto registra una observación
del transporte y no acredita la identidad de una persona detrás de la IP.

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

La página contiene como máximo 100 eventos completos y 262144 bytes UTF-8,
sumando actor, operación, recurso y el JSON compacto completo de cada contexto
presente, incluidas sus claves, valores y sintaxis. `context: null` añade cero
bytes a ese presupuesto de contexto. No es un límite del cuerpo HTTP completo:
los demás metadatos, la sintaxis de la página y los escapes del texto también
ocupan bytes en la respuesta. Los campos históricos no se
recortan para hacerlos caber ni se someten a los límites de entrada de filtros.
Si el conjunto elegido excede esa capacidad, se rechaza completo con `413` y
`audit_query_capacity_exceeded`; el usuario puede reducir `limit` o concretar
los filtros. Un único evento demasiado grande no se vuelve consultable por
truncarlo. El tamaño transmitido sigue acotado por este presupuesto y por el
máximo de eventos, metadatos y cursor.

La entrada inválida produce `400`; una sesión ausente o revocada, `401`; un rol
sin permiso, `403`. Los errores de persistencia o de invariantes no entregan una
página parcial. La consulta válida se registra como `audit.events_read` dentro
de su transacción. Una revocación detectada después de esa transacción impide
revelar los datos, aunque el evento de lectura ya registrado permanezca.

## Resultados de acceso y su interpretación

El registro general de acceso añade un evento independiente `access.*` después
del resultado del manejador admitido y confirma su persistencia antes de
entregar la respuesta. No sustituye el evento de actividad de negocio ni añade
un campo `outcome` al contrato: el resultado se obtiene de la acción.

| Estado HTTP observado del manejador | Acción | Etiqueta en la interfaz |
| --- | --- | --- |
| 2xx | `access.granted` | Concedido |
| 401 o 403 | `access.denied` | Denegado |
| Otros estados que no sean 5xx | `access.rejected` | Rechazado |
| 5xx | `access.failed` | Fallo |

Un `404` conserva la ambigüedad entre recurso inexistente y recurso oculto por
autorización; no se convierte en denegación demostrada. Tampoco un `409` o `422`
demuestra por sí solo falta de permisos. Una respuesta pública exitosa no
acredita la existencia de una cuenta, la finalización de MFA, la entrega de
correo ni una modificación de negocio. Los eventos históricos sin contexto y
las demás acciones no reciben una clasificación retroactiva de acceso; la
interfaz muestra **No disponible** para esos resultados y metadatos ausentes.

Si falla el anexado durable del acceso, se devuelve el `500` genérico en lugar
de la respuesta del manejador. Puede existir ya una modificación de negocio
confirmada: ese `500` no prueba reversión ni autoriza un reintento automático.
No se declara un evento durable que el almacén no haya confirmado. Una petición
rechazada por capacidad con `503` antes de su admisión no ejecutó una operación
admitida y queda fuera de esta población de registros durables.

El alcance finito corresponde a las rutas de producto reconocidas por el
servidor. Salud, archivos estáticos y rutas arbitrarias no reconocidas quedan
fuera; los rechazos de método, incluidos los `HEAD` explícitos, conservan su
significado de transporte. El contexto no modifica permisos: Client sigue sin
acceso a la bitácora global y la decisión humana sobre documentos Client
permanece separada. Los detalles están en [la matriz](access-audit-matrix.md).

La consulta filtrada no verifica por sí misma la cadena completa, no detecta el
truncamiento del final mediante un ancla externa y no acredita tiempos de anexado
inferiores a 500 ms. La medición del acceso empieza antes de esperar el escritor
y termina tras su commit durable; incluye espera y anexado, no el tiempo HTTP
total previo. Deben conservarse muestras individuales, máximo, distribución,
host, volumen de historia y concurrencia. La media o el p95 no sustituyen el
criterio por operación. La evidencia de esta extensión se registra por separado
en la matriz; la IP y el UUID históricos ausentes continúan sin reconstruirse.
Véase también [la limitación de anclaje](adr/0007-audit-chain-anchoring.md).
