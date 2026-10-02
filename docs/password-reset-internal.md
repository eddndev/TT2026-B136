# Recuperación de contraseña: frontera interna

## Alcance

El núcleo `application::identity::password_reset` separa la capacidad y su
consumo de la activación operativa. Los adaptadores de correo, HTTP y formulario
se describen en [el transporte público](password-reset-public.md); su aceptación
local no habilita recuperación en el despliegue.
La API de recuperación MFA existente sigue consumiendo un código después de
verificar la contraseña; no debe presentarse como recuperación de contraseña.
La [decisión transaccional](adr/0066-atomic-password-recovery.md) explica el diseño.

## Contrato de aplicación

`PasswordResetService` recibe repositorio, entrega, limitador, fuente de entropía,
hasher SHA-256 y hasher de contraseñas. `ResetPolicy` exige duración positiva
representable y cupo positivo; no tiene valor operativo predeterminado.

- `request(email)` normaliza el correo y consulta un límite independiente del
  contador de contraseñas fallidas. Una solicitud admitida puede emitir una
  capacidad de 32 bytes. El resultado no contiene el token ni distingue entre
  cuenta desconocida, inactiva, limitada o sin cupo.
- Sólo el repositorio elige la cuenta y generación vigentes, los tiempos y el
  identificador independiente `ResetId`. El digest tiene propósito exclusivo de
  recuperación. El enlace nuevo no invalida automáticamente otro enlace vigente.
- La entrega devuelve aceptación, rechazo definitivo o incertidumbre. Sólo el
  rechazo definitivo cancela el ID y digest exactos. Un error o incertidumbre no
  demuestra que el destinatario no recibió el mensaje; no se reenvía ni invalida
  automáticamente la capacidad.
- `complete(token, password)` exige 32 bytes y la política existente de
  contraseñas. Inspecciona provisionalmente la capacidad antes de calcular
  Argon2id. Esa inspección no autoriza la escritura: el repositorio revalida todo
  en la transacción de consumo y devuelve `Changed` o `Rejected`.

Los buffers propios de tokens y la entrada concatenada del digest se borran al
liberarse. El comando de consumo borra su hash al destruirse. El servicio no
reclama poder borrar los buffers del llamador ni las copias internas de drivers.
No se registran tokens, contraseñas ni digests en eventos públicos o auditoría.

## Persistencia y aceptación local

`PostgresPasswordResetRepository` abre el esquema validado con el rol restringido
de ejecución. Las operaciones resuelven la cuenta, generación y tiempos en
PostgreSQL. El consumo bloquea la cadena auditada y las filas implicadas antes de
comprobar la vigencia; sustituye el hash, incrementa revisión y generación y
consume la capacidad en la misma transacción que el evento. El recibo diferido
impide confirmar el cambio sin su evento exacto.

El rol de ejecución no lee ni modifica directamente la tabla de capacidades ni
los hashes. El esquema valida cuerpos, firmas, propietarios, privilegios e
inventario de funciones y triggers. Los errores públicos del adaptador no
incluyen tokens, digests, correos ni detalles del servidor.

La primera campaña PostgreSQL aprobó 19 pruebas en 48.11 s con instancias
propias desechables: consumo único y concurrente, cuotas, generaciones,
expiración después de bloqueos, permisos efectivos, rollback ante fallo de
auditoría y rechazo de alteraciones del catálogo. Otra prueba con Argon2id,
TOTP, protección AES, PostgreSQL y Redis reales aprobó en 11.79 s. Verificó que
la contraseña nueva permite completar el MFA existente, mientras la contraseña,
sesión y desafíos anteriores quedan rechazados por su generación. El cambio
preserva roles, asignaciones y material MFA; no borra las claves Redis para
simular su invalidación. Entrega, entropía y limitador de esa prueba son dobles
explícitos. Otros dos casos aprobaron en 5.07 s: vencimiento después de esperar el bloqueo
de la fila de usuario y ejecución con un esquema vacío de sólo USAGE delante
del esquema válido. El segundo reprodujo primero una invocación en el namespace
equivocado; el adaptador ahora obtiene el namespace de la tabla validada.
Estos resultados focales no sustituyen el cierre global de CI.

## Entropía y cupos de transporte

`RandomResetTokenSource` obtiene los 32 bytes del sistema operativo en un buffer
propio que se borra al liberarse. Un fallo después de llenar parcialmente el
buffer no devuelve una capacidad parcial. Los ensayos de inyección comprueban
el contrato, no la calidad estadística de la fuente ni el contenido de memoria
posterior a la liberación.

`RedisPasswordResetLimiter` exige cuatro políticas: cupo global y por correo
para solicitar, y cupo global y por digest para completar. Cada cupo es positivo
y cada ventana admite de uno a 86400 segundos; son límites técnicos, sin una
política operativa implícita. Las claves son exclusivas de recuperación y los
identificadores por sujeto usan SHA-256 con propósito separado. Esto evita
persistir correos en claro en las claves; no garantiza anonimato.

Una operación Lua comprueba ambos contadores antes de mutarlos: tipo, campos,
versión, política, números enteros, reloj y expiración absoluta exacta. Rechaza
estado alterado o una política incompatible sin repararlo. Agotar un cupo no
carga el otro ni amplía su vencimiento. Las admisiones concurrentes comparten
los mismos cupos globales. No se modifican sesiones, desafíos, bloqueos de
contraseña ni reclamaciones TOTP. La serialización y validación previas no reservan
memoria ni garantizan rollback ante OOM, caída del proceso o pérdida de las
claves. El transporte requiere conservación del estado y política `noeviction`;
un reinicio vacío abre ventanas nuevas. Un reloj que retrocede antes de la
apertura provoca rechazo; un retroceso dentro de la misma ventana conserva el
cupo y su fecha límite, aunque puede prolongar su duración real.

La conexión conserva autenticación y base Redis seleccionada. Exige límites
explícitos de conexión y de lectura/escritura establecidas. El driver síncrono
realiza autenticación, selección e identificación antes de poder aplicar estos
últimos límites; no se afirma que acoten ese handshake. Un fallo de transporte
puede ocurrir después de consumir el cupo; no se reintenta la operación.

La primera verificación focal aprobó once pruebas en 0.46 s contra Valkey 8.1.10
desechable y dos pruebas unitarias de entropía. Conservó expiraciones, aislamiento,
cuotas entre instancias y errores neutros. Estos resultados no son una
campaña sobre Redis del despliegue ni activan el flujo público. Tres pruebas
adicionales aceptaron ACL de escritura o expiración denegadas sin carga parcial.

Un recorrido combinado separado aprobó 1/1 en 10.98 s con PostgreSQL 18.6,
Valkey 8.1.10, fuente OS, Argon2id y MFA reales. La segunda solicitud limitada
conservó contadores y expiraciones sin otra entrega. El consumo confirmó cambio
auditado de contraseña y generación; credenciales anteriores quedaron rechazadas
y el nuevo acceso conservó TOTP. La entrega fue capturada en memoria mediante un
doble explícito, sin proveedor ni envío externo. Un replay y un intento posterior
limitado no repitieron la mutación SQL. Esta evidencia no sustituye los gates de
la cabeza publicada ni una prueba del recorrido público.

## Invalidación administrativa después de restaurar

`invalidate_restored_password_resets` usa una conexión privada del propietario
administrativo. Recibe UUID estable de operación, base, esquema y cabeza auditada
esperados. Fija un `search_path` seguro antes de resolver relaciones o bloquear;
valida nuevamente identidad, permisos, catálogo e inventario después de adquirir
el bloqueo del esquema real. No ejecuta migraciones ni repara un catálogo alterado.

Tras bloquear la cadena auditada y todas las capacidades pendientes, obtiene el
reloj de PostgreSQL. Cancela sólo las pendientes, incluidas las expiradas, y añade
un único recibo auditado en la misma transacción. Una capacidad emitida en el
futuro provoca rechazo completo. Mantiene contraseñas, generaciones, MFA,
pertenencias y recibos consumidos. Una operación sin pendientes también produce
un recibo. Ante resultado incierto se conserva el mismo UUID y predecesor: un
reintento devuelve el recibo original, pero rechaza un predecesor diferente o
nuevas capacidades pendientes.

Doce pruebas focales aprobaron en 46.46 s, incluido un respaldo y restauración
reales en un esquema desechable. Verificaron recibos, reintentos, bloqueos,
rollback, reloj posterior a esperas, permisos, catálogo, cadena y rechazo del
sombreado de funciones a través del `search_path` recibido.

Es una primitiva interna para un controlador administrativo. El llamador debe
mantener detenidos los escritores y cerrado el acceso durante toda la restauración
SQL/Redis. El [CLI administrativo](database-restore-commands.md) expone la
primitiva con un predecesor e identificador explícitos; el controlador de
despliegue todavía no la invoca ni se ha demostrado la recuperación operacional
completa. Restaurar SQL
puede recuperar una contraseña antigua; cancelar enlaces no corrige ese efecto.

## Activación pendiente

La activación requiere, además, entrega coherente y configurada, limitación del
transporte público, protección de URLs y secretos, comportamiento ante respuestas
perdidas, retención y coordinación de restauración con bloqueo durable del acceso.
La aceptación de contraseña nueva más MFA existente ya es local; no sustituye
la aceptación del recorrido HTTP ni de la entrega real. El estado detallado de pruebas pertenece al
[informe de verificación](verification-report.md).
