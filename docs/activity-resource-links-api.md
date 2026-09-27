# Recursos relacionados con una actividad

La lectura inversa descubre las asociaciones de recursos desde una audiencia o
plazo del expediente. Reutiliza las capturas verificadas de
[asociaciones existentes](resource-activities-api.md); no crea vínculos,
revisiones, alertas ni cambios de lectura. La decisión está en
[ADR 0059](adr/0059-activity-resource-navigation.md).

## Rutas y autorización

- `GET /api/v1/cases/{case_id}/hearings/{hearing_id}/resource-associations`
- `GET /api/v1/cases/{case_id}/deadlines/{deadline_id}/resource-associations`

Ambas requieren bearer y devuelven `Cache-Control: no-store`. Comparten los
cupos existentes de peticiones y trabajo bloqueante. No hay una consulta global
que permita buscar asociaciones sin identificar un expediente autorizado.

| Principal vigente | Lectura |
| --- | --- |
| Owner | Todos los expedientes |
| Litigator asignado | Su expediente |
| Paralegal asignado | Su expediente |
| Client | Denegada, incluso con asignación |

La aplicación comprueba identidad y permisos antes de consultar. El adaptador
revalida cuenta y pertenencia antes de buscar el destino, incluidas las páginas
vacías. Destino, cabezas de asociaciones, capturas y auditoría se comprueban en
una transacción. La respuesta sólo se entrega tras confirmar la lectura auditada
y reautenticar al principal completo. Los expedientes cerrados y recursos
archivados conservan sus reglas de lectura; esta consulta no los reactiva.

Una alerta personal no concede acceso adicional al expediente. Su autorización
por destinatario permanece en el servicio de alertas y cada consulta posterior
comprueba sus propios permisos vigentes.

## Consulta estricta y paginación

| Parámetro | Contrato |
| --- | --- |
| `limit` | Entero decimal canónico de 1 a 100; predeterminado 20. |
| `after_id` | UUID canónico de asociación, exclusivo; se omite en la primera página. |
| `status` | `linked` predeterminado; `unlinked` o `all` explícitos. |

Los UUID de ruta también deben ser canónicos. Se rechazan parámetros
desconocidos, duplicados o vacíos y valores no admitidos. El tipo del destino
procede de la ruta; `kind` no es un filtro de esta consulta. `all` elimina el
filtro de estado; no incluye expedientes ni destinos adicionales.

Se selecciona la cabeza actual de cada asociación y se filtra su estado antes
de ordenar y paginar por UUID ascendente. Cada continuación conserva expediente,
tipo, identidad de destino y filtro. Cambiar cualquiera inicia una nueva
consulta. `after_id` no es una autorización ni un token de instantánea: cada
petición observa el estado actual de forma independiente y una modificación
concurrente puede alterar las páginas sucesivas.

## Respuesta

Ejemplo de una página autorizada sin asociaciones:

```json
{
  "case_id": "00000000-0000-0000-0000-000000000001",
  "target": {
    "kind": "hearing",
    "id": "00000000-0000-0000-0000-000000000005"
  },
  "checked_at": {
    "unix_seconds": 1790467200,
    "nanosecond": 0,
    "offset_seconds": 0
  },
  "associations": [],
  "has_more": false,
  "next_after_id": null
}
```

`target.kind` es `hearing` o `deadline` y `target.id` conserva exactamente la
identidad solicitada. `checked_at` usa el objeto UTC existente: segundos Unix,
nanosegundo y desplazamiento cero. Está presente incluso cuando la página está
vacía y coincide con la observación de todas sus vistas.

Cada elemento de `associations` es la vista completa existente:

- `association`: cabeza de la asociación con identidad, revisión, estado,
  selección histórica, recurso y acto capturados, destino capturado, autoría,
  contexto administrativo y recibo.
- `checked_at`: mismo instante que la página.
- `current_target`: proyección actual de la audiencia o plazo, separada de la
  revisión histórica incluida en `association.sources.target`.

`has_more` verdadero exige una página llena y `next_after_id` igual al UUID de
su última asociación. Cuando no hay continuación, el cursor es nulo. Las filas
son estrictamente ascendentes, sin duplicados y posteriores al cursor recibido.
Una respuesta del puerto con ámbito, destino, estado, observación, orden,
continuación o capturas incoherentes se rechaza sin entregar una página parcial.

La revisión abierta de una actividad o conservada por una alerta puede diferir
de la revisión capturada por una asociación. La lista describe asociaciones
actuales, no las relaciones existentes al emitirse la alerta. No prueba que
un recurso causara el aviso ni modifica su origen histórico. Abrir la asociación
permite consultar su revisión exacta y el recurso capturado; consultar la cabeza
actual del recurso permanece como acción explícita independiente.

## Errores y límites

| HTTP | Situación |
| --- | --- |
| 400 | UUID, límite, cursor o consulta no válidos. |
| 401 | Bearer ausente, sesión inválida o vencida. |
| 403 | Rol denegado o acceso revocado. |
| 404 | Expediente o destino inexistente en el ámbito autorizado; se conservan `hearing_not_found` y `deadline_not_found`. |
| 500 | Contrato inconsistente del puerto, captura inválida o fallo de persistencia/auditoría; sin detalle interno. |
| 503 | Presupuesto compartido no disponible, conforme al runtime existente. |

Los errores de acceso o validación no se convierten en una lista vacía. La
consulta no marca alertas como leídas, crea notificaciones, modifica recibos,
activa plazos ni acredita efectos jurídicos. Su aceptación se registra por
separado en [el informe de verificación](verification-report.md).
