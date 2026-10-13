# Activacion operativa del acceso Owner

Este procedimiento usa el acceso construido y aceptado, descrito en
[operacion Owner](../owner-certificate-login-operations.md). La cuenta designada
es `hatziryvh@gmail.com`. El operador de CA `eddndev@gmail.com` no recibe sus
credenciales, clave privada ni rol. Activar el primer factor no sustituye MFA.

## Perfil preparado

Para una instancia con una titular Owner de demostracion, preparar estos
valores sin modificar la politica existente de sesiones o inactividad:

| Variable | Valor |
| --- | --- |
| `TT_OWNER_LOGIN_START_GLOBAL_MAX` | `60` |
| `TT_OWNER_LOGIN_START_GLOBAL_WINDOW_SECONDS` | `60` |
| `TT_OWNER_LOGIN_START_OWNER_BINDING_MAX` | `5` |
| `TT_OWNER_LOGIN_START_OWNER_BINDING_WINDOW_SECONDS` | `300` |
| `TT_OWNER_LOGIN_PROOF_GLOBAL_MAX` | `30` |
| `TT_OWNER_LOGIN_PROOF_GLOBAL_WINDOW_SECONDS` | `60` |
| `TT_OWNER_LOGIN_PROOF_TOKEN_MAX` | `1` |
| `TT_OWNER_LOGIN_PROOF_TOKEN_WINDOW_SECONDS` | `300` |
| `TT_OWNER_LOGIN_REDIS_CONNECT_MS` | `250` |
| `TT_OWNER_LOGIN_REDIS_IO_MS` | `1000` |

La cuota por cuenta/vinculo admite cinco comienzos en cinco minutos; cada
captura admite una presentacion. Una firma rechazada requiere un desafio nuevo,
conservando el comportamiento de un solo uso ya implementado. Los presupuestos
globales limitan la carga del primer factor; son superiores al uso esperado de
una sola titular sin convertir el endpoint publico en trabajo ilimitado.
Los tiempos corresponden a Redis local: acotan conexion y operaciones del socket,
no prometen un plazo total de autenticacion. Estos valores son una configuracion
operativa propuesta para esa instancia, no resultados de una medicion de carga.

No habilitar `TT_OWNER_LOGIN_ENABLED=true` hasta comprobar el acceso alternativo,
certificado Partner y vinculo vigente de la titular. El flag ausente o `false`
conserva el estado anterior. Este documento no acredita que se haya activado.

## Aplicacion y aceptacion

1. Registrar la release exacta, disponibilidad observada y hashes de los archivos
   de configuracion. Conservar una copia privada de la configuracion anterior,
   incluido `<root>/data/.env`, fuera de Git y con permisos `0600`.
2. La titular genera y conserva su clave fuera del servidor y entrega solo el
   CSR publico si necesita un Partner. El operador revisa/emite en la CA. La
   titular registra el vinculo y firma externamente su prueba de posesion en
   Qadra; guardar el recibo publico y comprobar la confianza vigente.
3. Preparar los once valores, incluido el flag, en el archivo privado usado por
   el lanzador existente. Validar con el binario aceptado y su configuracion;
   no copiar tokens, claves o codigos de autenticacion al registro de evidencia.
4. Aplicar en una ventana controlada con el respaldo y retorno anteriores. Tras
   iniciar la API exacta, comprobar `certificate-login/availability` con `enabled:true`,
   salud y conservacion de los accesos del despacho.
5. La titular descarga un desafio nuevo, lo firma externamente, completa MFA y
   confirma su UUID/correo Owner. Cierra la sesion y prueba el acceso alternativo
   por contrasena y MFA. No reutilizar capturas ni pedir secretos por chat.
6. Registrar revision, instante, configuracion no secreta y resultado real en
   el recibo de entrega. El material privado sigue bajo control de la titular.

Si falla la aceptacion, recuperar exclusivamente los valores anteriores del
primer factor y reiniciar la misma release. Comprobar `enabled:false` cuando ese
fuera el estado previo y que contrasena/MFA permanecen disponibles. No retirar
vinculos, borrar auditoria, revocar certificados ajenos ni regenerar la CA para
realizar ese retorno.
