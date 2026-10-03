# Autenticación interna por certificado de Owner

## Alcance actual

La alternativa de primer factor tiene aplicación y adaptadores con aceptación
local. El [transporte HTTP opcional](owner-certificate-login-http.md) tiene
verificación focal separada. Todavía faltan composición ejecutable, interfaz
pública y activación en VPS3. La vinculación previa de un certificado y su recibo público
son un recorrido distinto: un recibo histórico no permite iniciar sesión.
El contrato binario está en
[ADR-0068](adr/0068-owner-certificate-first-factor.md).

## Admisión y segundo factor

`IdentityService::with_certificate_login` recibe explícitamente autoridad,
almacenamiento temporal, verificador y hash. Las construcciones existentes
mantienen el acceso por contraseña y rechazan una sesión de origen certificado
cuando no disponen de esta autoridad.

El inicio consulta el Owner activo, el vínculo exacto vigente y la confianza
publicada. Emite una declaración con nonce impredecible y una ventana de hasta
300 segundos. El token opaco identifica una captura de esos hechos públicos;
no contiene una contraseña, secreto MFA, clave privada ni sesión autenticada.

La prueba consume esa captura antes de verificar RSA. Debe firmar exactamente
la declaración `OWNAUTH1`; no sirven las declaraciones de registro, retirada o
firma documental. Se consulta de nuevo la autoridad después del trabajo y antes
de devolver una capacidad. Un fallo de auditoría o una admisión final rechazada
retira la capacidad recién creada; una limpieza incierta es un error.

La prueba correcta sólo concede un desafío MFA nuevo. TOTP y códigos de
recuperación conservan su validación existente. La revisión de cuenta puede
avanzar legítimamente al consumir recuperación; la identidad y la generación de
revocación deben seguir coincidiendo. La sesión posterior conserva procedencia
de certificado y vuelve a comprobar cuenta, vínculo y confianza. Una retirada,
cambio de generación, publicación de confianza distinta o expiración impide su
admisión posterior. Esto no cancela retroactivamente un trabajo ya admitido.

## PostgreSQL y prueba criptográfica

La autoridad lee dentro de una transacción con el bloqueo compartido de
escritores auditados. Después de esperar observa la cuenta actual, el UUID y
Owner exactos del vínculo no retirado y la publicación de confianza actual.
No modifica los recibos históricos. Una ausencia conocida se distingue de un
fallo SQL; nunca usa autoridad conservada en memoria como sustituto de una
consulta fallida.

El adaptador RSA conserva el resultado público verificado de su API tipificada.
Al cruzar al puerto de autenticación neutraliza los fallos de credencial,
material o ventana como credenciales inválidas, sin revelar el detalle interno.

## Redis y compatibilidad

El runtime de primer factor exige cuatro presupuestos explícitos de ventana
fija: inicio global y por Owner/vínculo, presentación global y por token.
Son compartidos entre instancias. Una denegación no cobra el otro contador ni
extiende el plazo. Los errores de transporte o permisos no se interpretan como
ausencia o capacidad disponible; una escritura incierta no se reintenta.

Las capturas usan `identity:certificate-login:<sha256 del token>`; los controles
usan `identity:certificate-login-rate:v1:*`. Nonce y token proceden de dos
extracciones independientes del generador del sistema operativo. El DTO privado
es estricto, acotado y rechaza campos desconocidos, duplicados o tipos
incorrectos. Conserva enteros sin convertirlos a números Lua y reconstruye los
182 bytes exactos de la declaración antes de devolverla.

El vencimiento Redis es el instante firmado, no la duración sumada al momento
posterior de transporte. Creación y vencimiento son atómicos; la reclamación
consume antes de decodificar. Una captura corrupta no produce una prueba ni
permanece disponible para repetirla. El invalidador de restauración retira estas capturas con los escritores
cerrados y conserva sus presupuestos. Su prueba nativa de RDB, AOF y reinicio
aprobó; no sustituye la futura aceptación de restauración del recorrido HTTP.

Los registros de contraseña conservan su formato anterior. Los nuevos desafíos
MFA y sesiones de certificado tienen una versión distinta y procedencia explícita.
No se pueden degradar a contraseña. Las sesiones limitan sus plazos absoluto y
de inactividad por la vigencia capturada; el desafío inicial no limita toda la
vida de una sesión posterior. La actividad de certificado no modifica registros
de contraseña ni una procedencia diferente. Las comprobaciones de permisos
Redis preceden las escrituras múltiples para impedir cambios parciales.

## Verificación local del 3 de octubre de 2026

- Aplicación: 22 casos nuevos y 31 del recorrido existente; 0.01 s y 0.00 s,
  respectivamente, tras 4.71 s de compilación. Dos regresiones reprodujeron
  vencimiento idle/absoluto durante una espera de autoridad dentro del mismo
  segundo. La admisión final conserva ahora precisión de milisegundos.
- Verificador: ocho casos en 0.77 s, incluidos dos nuevos del puerto y las
  pruebas existentes de propósito, confianza, revocación y ventana.
- PostgreSQL 16.15 desechable: seis casos en 13.88 s. Incluyen espera del bloqueo,
  cambios de cuenta y CRL, evidencia histórica inmutable y errores de lectura.
- Valkey 8.1.10 compatible con Redis, aislado y autenticado: 11 casos de
  procedencia/sesión en 0.12 s; 23 regresiones de sesiones en 0.07 s; 21 casos de
  captura y presupuestos en 2.71 s. Sólo una suite y un compilador a la vez.

- Restauración: diez casos de comando en 0.016 s y un recorrido nativo en
  7.006 s. El RDB conservó tipo, campos y expiraciones; tras invalidar capturas,
  el AOF limpio impidió que el RDB anterior las resucitara. Los cuatro presupuestos
  de certificado y los controles anteriores permanecieron exactos.

Estas cifras son focales locales; no representan aceptación HTTP completa,
resultados de CI, restauración autenticada ni despliegue. La primera comparación
de no escritura usaba `DUMP`; el diagnóstico comprobó igualdad de campos y plazo
con bytes RDB distintos. La prueba compara ahora tipo, todos los bytes de campos
y expiración absoluta, sin depender del orden interno de serialización.

## Composición ejecutable

El binario comparte una única instancia de identidad para contraseña y certificado,
con habilitación explícita y validación previa de sus límites. La aceptación
nativa del router y las condiciones operativas están en
[composición del acceso](owner-certificate-login-operations.md); sus resultados
no convierten las campañas focales anteriores en una restauración autenticada.
