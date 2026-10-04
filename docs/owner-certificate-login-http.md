# Transporte HTTP del primer factor Owner

## Alcance

Estas rutas opcionales exponen la [admisión interna](owner-certificate-authentication.md).
El constructor `api_router_with_authentication_budget` recibe un
`AuthenticationHttp` con el flujo de certificado opcional y comparte el presupuesto
existente de solicitudes y trabajo bloqueante. Los constructores previos y la
opción ausente conservan 404 en ambas rutas. El binario incorpora una [composición explícita y deshabilitada por defecto](owner-certificate-login-operations.md).
La interfaz de acceso requiere su propia aceptación; exponer el router no la activa.

No se recibe clave privada, contraseña ni secreto MFA. El inicio por certificado
es una alternativa al primer factor; su prueba correcta entrega únicamente un
nuevo desafío MFA. La sesión se obtiene por las rutas MFA existentes.

## Disponibilidad del método

`GET /api/v1/auth/certificate-login/availability` devuelve únicamente
`{"enabled": true}` o `{"enabled": false}`, derivado de la composición efectiva.
No consulta cuentas, vínculos o certificados. No exige bearer, admite sólo cuerpo
vacío y ninguna query; su respuesta usa `no-store`. Comparte la admisión HTTP y
no necesita permiso de trabajo bloqueante para informar esta configuración.
Los constructores anteriores devuelven `false`; inicio y prueba siguen devolviendo
404 cuando el flujo está deshabilitado. No existe otra opción de frontend que
pueda habilitar un canal ausente en el servidor.

## Solicitar declaración

`POST /api/v1/auth/certificate-login/start`, sin bearer ni query, recibe un objeto
JSON con exactamente dos cadenas: `owner_id` y `binding_id`. Deben ser UUID
canónicos en minúsculas y distintos del UUID nulo. El límite real del cuerpo es
1024 bytes, independientemente de `Content-Length`.

La respuesta 200 contiene exactamente:

| Campo | Contenido |
| --- | --- |
| `challenge_token` | Token opaco de un solo uso; no es una sesión. |
| `statement_base64` | Los 182 bytes canónicos `OWNAUTH1`, en base64 estándar. |
| `expires_in_seconds` | Ventana de presentación concedida por la aplicación. |

La firma externa debe corresponder exactamente a esos bytes y al certificado
previamente vinculado. Ni un recibo histórico ni la firma de registro o retirada
sirven como prueba de inicio. Véase [ADR-0068](adr/0068-owner-certificate-first-factor.md).

## Presentar prueba

`POST /api/v1/auth/certificate-login/proof`, sin bearer ni query, recibe exactamente
`challenge_token` y `signature_base64`. El token es base64url canónico sin padding
para 32 bytes; la firma es base64 estándar canónico para 384 bytes RSA-3072.
El límite real del cuerpo es 2048 bytes.

La respuesta 200 conserva el contrato MFA existente: `challenge_token` y
`expires_in_seconds`. No contiene `access_token` ni `Set-Cookie`. La aplicación
consume la captura antes de verificar la prueba admitida. Un rechazo sintáctico
HTTP no afirma haber consumido esa captura. No hay reenvíos automáticos ante
error o resultado incierto.

## Rechazos y límites

Se rechazan arreglos posicionales, campos desconocidos o duplicados, nulos,
ausencias, tipos incorrectos, contenido posterior al objeto y cualquier query.
La lectura conserva el contrato JSON y de tipo de contenido compartido por la API.
Todos los resultados, incluidos errores y rutas deshabilitadas, usan `no-store`.

| Estado | Significado |
| --- | --- |
| 400 | Solicitud inválida o material no canónico; no se invoca el flujo. |
| 401 | Credenciales rechazadas, sin detallar cuenta, vínculo ni causa criptográfica. |
| 413 | El cuerpo excede el límite de la ruta. |
| 429 | Presupuesto de autenticación denegado. |
| 500 | Fallo interno opaco; no equivale a ausencia de cuenta o permiso. |
| 503 | Capacidad HTTP o de trabajo agotada. |

La admisión de solicitudes precede la lectura del cuerpo. Una prueba y las demás
operaciones de la API comparten el presupuesto bloqueante. Si se abandona la
respuesta HTTP, el trabajador iniciado conserva su permiso hasta terminar; no
se permite trabajo adicional por haber desaparecido el cliente.

## Frontera de aceptación

Las pruebas de transporte usan un flujo controlado y los constructores reales.
Comprueban proyecciones, entradas estrictas, límites de cuerpos, errores,
compatibilidad con contraseña/MFA/recuperación y conservación del presupuesto
ante cancelación. Su flujo controlado no prueba RSA, PostgreSQL, Redis ni restauración.
La aceptación nativa adicional de la composición ejecutable sí ejercita RSA,
MFA y ambos almacenes mediante el router real; mantiene separadas las fronteras
de listener HTTP, restauración e interfaz. Los resultados ejecutados se registran por separado en el
[informe de verificación](verification-report.md).

Tres regresiones de disponibilidad reprodujeron primero la ausencia de la ruta.
La suite HTTP posterior aprobó 22 casos en 0.29 s, incluidos los 19 anteriores.
