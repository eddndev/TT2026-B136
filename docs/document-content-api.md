# Contenido documental exacto e incidentes de integridad

Estado: implementación en curso. La evidencia de aceptación se registra en
[el informe de verificación](verification-report.md). La decisión está en
[ADR-0042](adr/0042-verified-document-content-and-integrity-incidents.md).

## Descargar contenido

`GET /api/v1/cases/{case_id}/documents/{id}/versions/{version}/content`

Requiere bearer y versión positiva explícita, sin ceros iniciales ni parámetros
de consulta. No selecciona automáticamente la cabeza. Owner consulta cualquier expediente; Litigator y
Paralegal requieren asignación vigente. Client queda denegado. Un expediente
cerrado conserva esta lectura. Pertenencia, cuenta y rol se comprueban de nuevo
antes de confirmar el acceso auditado.

La ruta acepta contenido pendiente de sello o sellado. Descifra con AES-GCM,
liga identidad y versión mediante AAD y contrasta SHA-256. No firma, solicita un
sello de tiempo, evalúa firma/certificado/TSA ni cambia el estado de evidencia.
Tampoco vuelve a admitir el formato de un archivo histórico.

Respuesta satisfactoria: `200` con los bytes originales completos y:

| Cabecera | Valor |
| --- | --- |
| `Content-Type` | `application/octet-stream` |
| `Content-Disposition` | `attachment`, con nombre saneado |
| `Content-Length` | Tamaño exacto de los bytes entregados al transporte |
| `Cache-Control` | `no-store` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Case-Id` | UUID del expediente solicitado |
| `X-Document-Id` | UUID del documento solicitado |
| `X-Document-Version` | Versión decimal canónica |
| `X-Document-Digest` | SHA-256 hexadecimal minúsculo de 64 caracteres |

`Range` se ignora: se devuelve el archivo completo tras validarlo. No se emiten
ETag, 304 ni contenido parcial. `HEAD` responde 405 sin invocar el caso de uso.
La capacidad de descargas usa el máximo configurado de peticiones y conserva
el cupo hasta liberar la última referencia a sus bytes. Si se agota, responde
503 antes del procesamiento. El límite de esta ruta es 16 MiB de contenido; los históricos mayores no se
eliminan ni se califican como alterados por excederlo.

Los errores usan el sobre JSON habitual, nunca un adjunto ni un fragmento del
archivo:

| Estado / código | Significado |
| --- | --- |
| 401 / autenticación | Sesión ausente, inválida o revocada |
| 403 / `permission_denied` | Rol sin permiso |
| 404 / documento no encontrado | Identidad, versión o ámbito no disponibles |
| 409 / `document_content_validation_failed` | La comprobación de contenido o del snapshot falló |
| 413 / `document_content_too_large` | Supera la cota de lectura |
| Error técnico | Fallo de servicios, auditoría o registro de incidente; no se afirma una notificación confirmada |

Qadra contrasta las cabeceras con la ficha exacta y descarta respuestas tardías
al cambiar de versión, expediente o sesión. Sólo inicia la descarga tras esas
comprobaciones. El mensaje de éxito expresa el inicio de la descarga, no que el
navegador haya guardado el archivo en disco.

## Buzón interno de Owner

`GET /api/v1/document-integrity-incidents?limit=50&after_id={uuid}`

El límite admite 1–100. `after_id` es exclusivo y el orden es UUID ascendente,
no cronológico. Los parámetros desconocidos o repetidos se rechazan. Respuesta:
`{incidents, has_more, next_after_id}`; el cursor es la última identidad emitida
cuando hay otra página y `null` al terminar. No es un total del despacho.

`GET /api/v1/document-integrity-incidents/{id}` devuelve una fila directamente:

```json
{
  "id": "UUID",
  "observation_id": "UUID",
  "case_id": "UUID",
  "document_id": "UUID",
  "document_version": 1,
  "requester_id": "UUID",
  "failure": "authentication_failed",
  "detected_at": "2026-09-19T12:00:00Z",
  "recorded_at": "2026-09-19T12:00:00Z",
  "expected_digest": "64 lowercase hex characters",
  "observed_snapshot_digest": "64 lowercase hex characters"
}
```

El ejemplo describe la estructura; los identificadores y huellas abreviados no
son datos válidos para un ensayo. Las categorías son `malformed_vault`,
`authentication_failed`, `digest_mismatch` y `snapshot_changed`. No identifican
al causante ni prueban por sí solas un ataque. La autenticación del cifrado puede
fallar también por una KEK incorrecta.

Cada lectura exige Owner activo y auditoría confirmada. Los otros roles no
reciben datos; una degradación o revocación vigente se comprueba otra vez. El
detalle ausente usa `document_integrity_incident_not_found`. No hay POST público
para fabricar observaciones, ni rutas de reconocimiento, reparación o borrado.

El aviso es interno y persistente. Qadra consulta su existencia al iniciar
sesión y permite abrir o actualizar el buzón. Un error de consulta no se muestra
como ausencia de incidentes. Abrir una versión revalida el acceso; no descarga
automáticamente contenido rechazado. No se afirma envío de correo ni atención
humana del incidente.
