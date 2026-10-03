# Captura privada con hechos de compatibilidad

`ops/deploy/restore_capture.py` conecta la captura existente con las observaciones
locales de PostgreSQL y Redis y produce el descriptor descrito en
[deployment-restore-compatibility.md](deployment-restore-compatibility.md).
`ops/deploy/restore_redis.py` invalida únicamente sesiones y desafíos en un destino
privado explícito. Son entradas internas; no restauran ni publican un despliegue.

## Captura

```python
capture(root, *, controller_directory, targets, tools, descriptor_path)
```

El resultado contiene la ruta del backup y el SHA-256 del descriptor. El recibo
debe ser nuevo y estar bajo el directorio privado `root/receipts`. Su hash necesita
retención independiente para actuar como referencia externa; devolverlo no crea
por sí solo esa independencia.

El llamador suministra identidad y herramientas explícitas:

- PostgreSQL: PID, directorio, base, esquema, propietario y rol runtime.
- Redis: PID, directorio y base lógica cero.
- Rutas resueltas de `postgres`, `psql`, `pg_dump`, `pg_restore`, servidor Redis,
  cliente Redis y comprobador RDB.

El perfil inicial requiere PostgreSQL 16, UTF-8, locale C/libc y los roles y base
provisionados `qadra_admin`, `qadra_runtime` y `qadra`. Rechaza conexiones ajenas,
configuración o vencimientos de rol, membresías y ajustes de base que el descriptor
no representa. Comprueba los atributos restringidos del runtime. El propietario
especial de PostgreSQL `pg_database_owner` se resuelve al propietario de esa base;
el catálogo literal también se compara antes y después. El descriptor registra
el propietario de la base, no todos los atributos posibles del catálogo.

Valkey conserva su identidad y versión propia aunque anuncie además una versión
de compatibilidad Redis. Se registran las huellas de las herramientas definidas
por el descriptor, las versiones observadas y el formato del RDB capturado. Los
hechos deben ser estables entre ambas observaciones, incluido el inventario del
cliente PostgreSQL usado para recogerlos. No se cambia el modo AOF ni se acepta
una reescritura en curso.

La operación toma `deploy.lock` y respeta las dos barreras de mantenimiento. Exige
API/web detenidos; no los detiene ni drena por sí misma. El llamador debe mantener
cerrados sus escritores durante toda la captura. Ausencia momentánea de clientes
y una cabeza auditada estable no prueban exclusión frente a un administrador
concurrente; tampoco crean una transacción entre PostgreSQL, Redis y archivos.

Exporta toda la auditoría ordenada a un archivo temporal privado y ejecuta
`--json audit verify-chain` con el binario del release inventariado. Solo acepta
confirmación completa y el número exacto de filas. Guarda explícitamente una
cabeza original o `null` para una cadena vacía verificada. Las consultas son de
solo lectura, con resolución de catálogo fija y tiempos limitados.

Después de `Runtime.backup` vuelve a comprobar hechos, auditoría e inventario.
Un cambio conserva el backup completo como captura sin descriptor calificado y
devuelve error. No migra, repara, restaura, elimina backups ni sustituye recibos.

Antes de publicar, la admisión existente valida el descriptor temporal contra
los cinco archivos y el release. La publicación usa un enlace exclusivo desde
ese archivo privado ya sincronizado, seguido de `fsync` del directorio del recibo.
Un recibo existente nunca se reemplaza. Si falla la sincronización, no devuelve
éxito aunque el nombre ya sea visible; ese resultado incierto no autoriza una
reescritura automática.

## Límites y credenciales

`restore_commands.py` lee stdout y stderr mediante pipes con límites por flujo y
un plazo monotónico. Ante un fallo termina y recoge el grupo que acaba de crear;
no busca procesos por nombre. No devuelve stderr ni excepciones del proceso.

- Catálogo y Redis: 64 KiB de salida por flujo.
- Versiones y resultado del verificador: 8 KiB.
- Auditoría: 8 MiB y 100000 filas.
- Consultas, versiones y comprobaciones: como máximo 10 segundos.
- Dump SQL y captura RDB: como máximo 300 segundos.
- Recogida tras terminar un proceso fallido: como máximo 5 segundos.

Las credenciales proceden de los ajustes privados existentes o del argumento
interno de invalidación y se entregan por entorno. No aparecen en argumentos de
proceso, recibos ni diagnósticos. Los temporales se retiran solo si conservan el
dispositivo e inodo creados por la operación.

`Runtime.backup` acepta opcionalmente herramientas y un ejecutor explícitos. La
nueva captura utiliza esa costura para ejecutar los binarios inventariados; los
callers existentes conservan sus valores y comportamiento anteriores.

## Invalidación Redis

`invalidate_sessions` exige cliente, host loopback, puerto, contraseña, PID,
directorio y límites explícitos. Selecciona DB0 y RESP2 para las respuestas JSON de comandos estructurados.
La observación `INFO server` usa el formato textual propio de redis-cli y se
valida por separado con los mismos límites de salida. Verifica PING, PID y directorio antes de inspeccionar claves.

Recorre por completo ambos espacios antes del primer `DEL`. Solo admite las
claves `identity:session:` e `identity:challenge:` seguidas de 64 dígitos
hexadecimales minúsculos. Deduplica observaciones y comparte el presupuesto de
SCAN entre los recorridos previos y finales. Los máximos son 1000 SCAN, 10000
claves únicas, lotes de 100 y 10 segundos por comando; cada llamada proporciona
valores positivos dentro de esos límites.

Después del borrado exige un recorrido vacío de ambos espacios. Un error o una
clave nueva impide devolver éxito; puede haber ocurrido borrado parcial y el
reintento es seguro. No elimina límites de contraseña/reset, protección TOTP ni
datos ajenos, y no modifica vencimientos, AOF o servicios. Persistencia y reinicio
siguen siendo responsabilidad del ensayo o controlador llamador.

## Verificación y alcance pendiente

La verificación local del 3 de octubre de 2026 aprobó ocho recorridos de captura
e invalidación tras reproducir la ausencia de ambas entradas, más veinte pruebas
de compatibilidad del backup. Tres pruebas con procesos reales verificaron límites
de stdout/stderr, plazos, recogida del grupo propio y conservación de un proceso
ajeno. Los dobles de motor y verificador de las ocho primeras siguen declarados.

La revisión contra el esquema real encontró que el exportador trataba la fecha
auditada como entero aunque PostgreSQL guarda texto RFC 3339. Dos regresiones
reprodujeron ese rechazo y la permanencia del archivo lateral del verificador.
La corrección conserva el texto exacto, incluidos nanosegundos, y retira sólo el
`.lock` creado por la operación. Ambas regresiones y cuatro casos de captura
aprobaron después del cambio. La CLI conserva la verificación de fechas y cadena;
el exportador no redondea ni recompone los instantes.

## Ensayo nativo poblado y desechable

El ensayo explícito `scripts/tests/native_restore_acceptance.py` aprobó un caso
completo en 27.424 s, incluidos preparación y limpieza, el 3 de octubre de 2026.
Usó PostgreSQL 16.15, Redis 7.4.11, un binario de desarrollo del código actual y
un paquete interno con inventario y migraciones exactos. Ese paquete no se publicó
como release de despliegue. La preparación del compilador y dependencias queda
separada del tiempo del caso.

La prueba creó por HTTP dos usuarios, un expediente y dos asignaciones, con
Argon2id y MFA reales. Sembró capacidades consumida, vencida, cancelada y pendiente
mediante las funciones SQL reales; el consumo y su evento canónico compartieron
transacción. Capturó quince eventos y verificó toda su cadena con la CLI.

Tras restaurar el dump sin migraciones ni reparaciones, el comando `database check`
validó el esquema y el runtime. Una cabeza falsa se rechazó sin cambiar filas.
La invalidación del predecesor original canceló dos capacidades y añadió exactamente
un evento; repetir la misma operación devolvió el mismo recibo sin otro efecto.
Conservó el conjunto completo de capacidades, usuarios y asignaciones y verificó
otra vez la cadena. Configuración, CA y TSA se extrajeron con inventario y hashes
exactos y reemplazaron las copias de trabajo antes del arranque posterior.

Redis recuperó los tipos, valores y vencimientos absolutos. La comparación de
hashes Redis ordena sus campos: el orden de serialización `DUMP` no forma parte
del valor lógico. Se retiraron dos sesiones y un desafío, conservando siete
controles y claves ajenas. El AOF regenerado prevaleció sobre una copia antigua
del RDB al reiniciar. Las cuotas de recuperación siguieron aceptando consumo con
el Lua real, sin renovar su vencimiento. La API rechazó las credenciales previas;
un login nuevo y el MFA existente aprobaron con la PKI restaurada.

Sólo se simuló la observación de unidades systemd: PostgreSQL, Redis, API, PKI,
serialización, cadena de auditoría y comandos administrativos fueron reales. No
hubo correo externo ni ingreso público; la barrera de restauración permaneció
activa. El ensayo no emite ni consume un nuevo enlace de recuperación mediante
HTTP después de restaurar: ese flujo conserva su aceptación separada. El resultado
final incorpora también los fallos de limpieza, y los procesos son propiedad
del ensayo.

Para reproducirlo se requieren Python con `psycopg2`, OpenSSL, herramientas
PostgreSQL 16, Redis 7.4.11 y un paquete interno vigente íntegro. El binario debe
incluir los comandos de validación e invalidación de recuperación. No reutilizar
un paquete anterior con un inventario SQL distinto.

```bash
python3 -B scripts/tests/native_restore_acceptance.py \
  --release /private/current-test-release \
  --postgres-bin /private/postgresql16/bin \
  --redis-bin /private/redis7411/bin \
  --result /private/restoration-result.json
```

El directorio temporal es exclusivo y se retira al terminar; los diagnósticos
acotados se conservan con permisos privados junto al resultado. Nunca se usa una
base, sesión o clave de la instalación activa.

## Pendiente operacional

Una restauración operacional requiere además parada y drenaje, staging y promoción
durables, reconciliación tras fallo, confianza PKI y contadores vigentes,
instalación verificada y decisión sobre la fuente Redis y controles perdidos.
Estas entradas no autorizan instalar, habilitar, limpiar la barrera ni activar
correo de recuperación.
