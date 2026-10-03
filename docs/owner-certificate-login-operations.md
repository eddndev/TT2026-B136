# Composición del acceso Owner con certificado

El binario `serve` puede habilitar explícitamente el primer factor descrito en
[el contrato HTTP](owner-certificate-login-http.md). Permanece deshabilitado por
defecto. Habilitarlo no registra titulares, publica confianza, entrega claves ni
sustituye MFA. Requiere una cuenta Owner activa, su vínculo vigente y confianza
publicada mediante los procedimientos existentes.

## Configuración explícita

`TT_OWNER_LOGIN_ENABLED=true` habilita la composición sólo si se suministran todos
los valores siguientes. Cada variable tiene la opción `--owner-login-...`
correspondiente, visible en `serve --help`.

| Variable | Significado |
| --- | --- |
| `TT_OWNER_LOGIN_START_GLOBAL_MAX` | Cuota positiva global de solicitudes de declaración. |
| `TT_OWNER_LOGIN_START_GLOBAL_WINDOW_SECONDS` | Ventana de esa cuota, de 1 a 86400 segundos. |
| `TT_OWNER_LOGIN_START_OWNER_BINDING_MAX` | Cuota positiva por cuenta y vínculo seleccionado. |
| `TT_OWNER_LOGIN_START_OWNER_BINDING_WINDOW_SECONDS` | Ventana por cuenta y vínculo, de 1 a 86400 segundos. |
| `TT_OWNER_LOGIN_PROOF_GLOBAL_MAX` | Cuota positiva global de presentaciones de firma. |
| `TT_OWNER_LOGIN_PROOF_GLOBAL_WINDOW_SECONDS` | Ventana de presentaciones globales, de 1 a 86400 segundos. |
| `TT_OWNER_LOGIN_PROOF_TOKEN_MAX` | Cuota positiva por huella de token. |
| `TT_OWNER_LOGIN_PROOF_TOKEN_WINDOW_SECONDS` | Ventana por token, de 1 a 86400 segundos. |
| `TT_OWNER_LOGIN_REDIS_CONNECT_MS` | Tiempo positivo y representable para conectar TCP con Redis. |
| `TT_OWNER_LOGIN_REDIS_IO_MS` | Tiempo positivo y representable para operaciones del socket establecido. |

Los valores se validan antes de abrir adaptadores o iniciar servicios. Si la
opción de habilitación está ausente, los valores numéricos guardados no activan
el flujo. El analizador de argumentos sigue rechazando entradas que no puedan
representarse en sus tipos. Los tiempos de socket no constituyen una garantía
de plazo total de autenticación ni de las operaciones iniciales AUTH/SELECT del
controlador Redis; véase la frontera de transporte en
[la descripción interna](owner-certificate-authentication.md).

## Una identidad y un presupuesto

La composición comparte exactamente una instancia de `IdentityService` entre
los puertos de contraseña y certificado. Sus sesiones, MFA y reloj pertenecen al
mismo servicio. Los demás módulos reciben la vista de identidad habitual; el
router recibe además la vista opcional del primer factor. No se crea un segundo
conjunto de permisos HTTP o de trabajadores bloqueantes.

La autoridad consulta PostgreSQL y la confianza publicada; la prueba usa el
verificador RSA del perfil Partner. El runtime Redis aplica los cuatro límites,
conserva capturas de un uso y retiene la procedencia del certificado. El servicio
revalida esa autoridad al admitir MFA, consultar la identidad y renovar actividad.
Cambiar la revisión de confianza o retirar el vínculo rechaza las capacidades
anteriores, sin convertirlas a sesiones de contraseña.

## Evidencia reproducida

El 3 de octubre de 2026 aprobaron siete pruebas de configuración y composición en
0.09 s; Clippy focal terminó en 20.99 s. Un recorrido nativo adicional aprobó en
12.63 s, tras 12.42 s de compilación, con PostgreSQL 16.15 y Redis compatible
Valkey 8.1.10 desechables. Los servicios se retiraron al terminar.

El recorrido abre la composición real, registra un Partner y firma externamente
los 182 bytes con OpenSSL. Comprueba rechazo y consumo de firma alterada,
repetición, MFA TOTP real, procedencia Redis y vigencia de sesión independiente
de los cinco minutos del desafío inicial. Una CRL sucesora y el retiro del
vínculo invalidan capturas, MFA pendientes y sesiones derivadas; las capacidades
de contraseña permanecen independientes y se conserva la cadena auditada.

La prueba usa el router real en proceso. No constituye por sí sola evidencia de
listener HTTP, interfaz de acceso, restauración completa ni activación en VPS3.
Esas fronteras requieren su aceptación propia. No se cambió la configuración del
despliegue activo al ejecutar esta comprobación.

El recorrido CLI completo `scripts/demo.sh` aprobó en 25.997 s. Conservó las
comprobaciones positivas y negativas de cifrado, firma, sello, revocación, OTP,
auditoría y exportación; no reemplaza la aceptación HTTP de este primer factor.

## Capacidades después de restaurar

La restauración desechable invalida sesiones, desafíos MFA y capturas pendientes
del primer factor por certificado. Conserva los límites de contraseña y
certificado, las marcas TOTP ya utilizadas y sus vencimientos absolutos;
restaurar no concede nuevos intentos de autenticación. El guion comprueba el
rechazo de las capacidades anteriores mientras todavía estarían dentro de su
plazo, además de la ausencia de sus claves exactas en Redis.

Un ensayo focal con listener real aprobó en 23.248 s. Después de restaurar SQL
conservó el vínculo, su prueba pública y auditoría; una firma nueva exigió MFA
nueva, y el acceso por contraseña siguió siendo independiente. La lectura de
evidencia SQL se limita a la cuenta seleccionada, incluso cuando hay varios
Owner. No constituye instalación ni restauración del entorno operativo.

El guion de aceptación espera una vez por lote el siguiente intervalo TOTP
antes de solicitar nuevos desafíos. Una anomalía del reloj aborta el lote; una
respuesta MFA rechazada no se reintenta. Tres regresiones controladas aprobaron
en 0.021 s. Esta preparación evita volver a usar el código del intervalo actual
sin borrar sus marcas de uso ni consumir recuperación para alterar revisiones.

La campaña completa `scripts/api-demo.sh` aprobó después en 365.645 s con los
mismos servicios y validadores nativos. Incluyó los módulos existentes antes y
después de restaurar, el vínculo Owner y las nuevas capacidades de certificado.
No sustituye la revisión CI de la revisión publicada ni activa el servidor.
