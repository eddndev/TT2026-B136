# Respaldo privado de PostgreSQL, Redis y claves

## Alcance y aceptación

El controlador de [despliegue](deployment.md) captura el estado antes de activar
una versión. Esta guía describe la captura y una restauración manual que debe
ensayarse en instancias aisladas. La aceptación focal local aprobó ocho pruebas
de fallos y un recorrido con procesos nativos compatibles Valkey 8.1.10:
RDB, expiraciones absolutas, invalidación selectiva y reinicio desde AOF.
SQL y systemd se simularon en ese recorrido; no fue una restauración completa
del despliegue. Consultar [el informe de verificación](verification-report.md). Un respaldo creado no demuestra por sí solo una
recuperación completa, un tiempo de recuperación ni una copia fuera del servidor.

## Contrato de captura

`ops/deploy/release.py` mantiene el bloqueo exclusivo `deploy.lock`, detiene API
y web y llama a `Runtime.backup`. La captura exige ambos servicios inactivos;
además contrasta el PID de Redis con `qadra-redis.service` y su directorio con
`<root>/data`. Deben permanecer detenidos todos los escritores de esa instancia,
incluidos procesos CLI iniciados fuera del controlador. El bloqueo de despliegue
no bloquea por sí mismo a esos clientes externos.

Cada directorio bajo `<root>/backups` contiene:

| Archivo | Contenido |
| --- | --- |
| `database.dump` | Respaldo PostgreSQL de formato custom, tomado con el rol administrativo. |
| `redis.rdb` | Instantánea Redis, incluidos controles de identidad y sus vencimientos. |
| `private-state.tar.gz` | Configuración y material privado CA/TSA existente al capturar. |
| `COMPLETE` | Marcador publicado al terminar las capturas, validación y sincronización. |

El directorio tiene modo `0700` y los archivos `0600`, independientemente del
umask del operador. Redis se captura con `redis-cli --rdb`, autenticación por
`REDISCLI_AUTH` y dirección loopback; la contraseña no se coloca en argumentos.
La copia debe ser no vacía y aprobar `redis-check-rdb`. Un fallo de captura,
validación o persistencia impide declarar completo el respaldo y aborta la
activación. No se sustituye por un RDB anterior ni se omite Redis para continuar.
El controlador intenta recuperar la versión previa; si no existe o falla su
recuperación, la entrada permanece detenida. El modo de captura procede de la
[interfaz oficial de Redis CLI](https://redis.io/docs/latest/develop/tools/cli/#remote-backups-of-rdb-files).

Las copias antiguas sin `redis.rdb` sólo cubren SQL y material privado, aunque
tengan `COMPLETE`. No reinterpretarlas como copias completas de identidad. La
primera captura precede a la inicialización de PKI: es necesaria otra captura,
con API/web detenidas y el mismo bloqueo, que incluya las claves ya inicializadas.

Copiar el conjunto fuera del servidor mediante SSH con clave de host verificada,
a almacenamiento restringido ajeno al repositorio y a artefactos públicos.
Registrar tamaños y SHA-256 antes y después de transferir; no imprimir contenidos
privados. El marcador no es una firma ni protege contra modificación posterior.
No se borran automáticamente respaldos o versiones anteriores.

## Restauración manual antes de reabrir tráfico

1. Bloquear ingreso y detener todas las instancias que escriban en PostgreSQL o
   Redis. Mantener el bloqueo de despliegue durante mantenimiento. Verificar los
   archivos, hashes de transferencia, versión de Redis y revisión del binario.
   Ensayar primero sobre procesos, puertos y directorios desechables propios.
   Conservar intactos los datos originales hasta aceptar la restauración.
2. Restaurar SQL, configuración y claves del mismo conjunto, con sus propietarios,
   privilegios y esquema. Un `pg_dump` de una base no incluye la creación de roles
   del clúster: preparar los roles necesarios antes de restaurar. Seguir
   [la guía de base de datos](database-operations.md); no desactivar guardas,
   regenerar claves ni modificar la huella de migraciones para conseguir arranque.
3. Si Redis sigue íntegro, conservar sus controles actuales e invalidar únicamente
   sesiones/desafíos como indica el paso siguiente. No reemplazarlo por una copia
   antigua sólo porque se haya restaurado SQL. Si Redis se perdió, cargar el RDB
   verificado en un directorio nuevo y privado con la misma versión compatible,
   loopback, autenticación y `appendonly no` inicialmente. No reutilizar un
   directorio con AOF existente ni copiar archivos sobre un Redis en ejecución.
4. Antes de borrar claves, verificar PID, directorio, puerto y base lógica `0`
   de la instancia recuperada. Enumerar con `SCAN`, con límite de recorrido y
   lotes acotados, y eliminar sólo claves completas que coincidan con
   `identity:session:[a-f0-9]{64}` o `identity:challenge:[a-f0-9]{64}`. Son patrones
   de validación, no expresiones que Redis interprete como regex. Rechazar claves
   coincidentes de formato inesperado, errores o recorridos incompletos; comprobar
   finalmente que no quedan sesiones ni desafíos. No usar `FLUSHDB`/`FLUSHALL`.
   No borrar `identity:password-failures:*`, `identity:totp-used:*` ni otros datos.
5. Conservar valores y vencimientos de los controles que aún estén vigentes.
   Redis persiste expiraciones absolutas: el tiempo transcurre mientras está
   apagado. No asignar de nuevo el TTL original ni resucitar claves expiradas;
   comprobar sincronización de relojes. Véase
   [persistencia de expiraciones](https://redis.io/docs/latest/commands/expire/#expires-and-persistence).
6. Para una instancia reconstruida desde RDB, habilitar AOF sobre los datos ya
   cargados y depurados mediante `CONFIG SET appendonly yes`. Antes de reiniciar,
   exigir en `INFO persistence` que `aof_rewrite_in_progress` y
   `aof_rewrite_scheduled` sean `0`, y `aof_last_bgrewrite_status` sea `ok`;
   aplicar un plazo acotado y mantener cerrado el ingreso ante fallo. La
   configuración de arranque también debe conservar `appendonly yes`. Reiniciar
   sólo esa instancia y comprobar que no reaparecen sesiones/desafíos y que los
   controles no expirados se conservan. En una instancia AOF conservada, comprobar
   igualmente que la invalidación se persiste antes de reabrir ingreso.
7. Validar catálogo, inventarios, privilegios, cadena de auditoría y confianza
   con la versión correspondiente. Comprobar rechazo de tokens anteriores y nuevo
   acceso mediante contraseña/MFA; incluir rechazo por límite de intentos y por
   reutilización TOTP en el ensayo poblado. Reabrir tráfico sólo tras aceptar esos
   resultados. El guion `scripts/api_identity_restore.py` ilustra invalidación
   limitada para fixtures desechables; sus variables no autorizan ejecutarlo sin
   adaptación sobre una instancia desplegada.

**Precedencia de AOF:** con AOF y RDB presentes, Redis reconstruye desde AOF.
Copiar `redis.rdb` o cambiar únicamente la configuración y reiniciar puede perder
la restauración. La conversión sobre el proceso recuperado y la comprobación de
reescritura siguen la [documentación de persistencia](https://redis.io/docs/latest/operate/oss_and_stack/management/persistence/).
No usar comandos de respaldo de versiones Redis posteriores sin verificar que
existen en la versión desplegada.

## Prueba obligatoria de persistencia Redis

`scripts/tests/test_deployment_redis_snapshot.py` captura claves sintéticas en
procesos desechables, comprueba valores y expiraciones del RDB restaurado e
invalida sesiones y desafíos antes de generar un AOF. El reinicio con un RDB
anterior presente debe conservar esa invalidación y los demás controles.
Requiere Redis servidor y CLI 7.4 o posterior y falla explícitamente si no
están disponibles; sustituye PostgreSQL y systemd por fixtures.

CI ejecuta esta prueba en el job `Deployment backup`, en `tt-ci-live-primary`.
Coverage depende de su resultado y de los tests Rust; Deploy version reutiliza
el mismo gate. La prueba no acredita restauración PostgreSQL, datos reales ni
la actualización de los controladores instalados en el servidor.

## Límites del punto de recuperación

Una copia histórica no contiene intentos fallidos, reclamos TOTP ni otros cambios
posteriores a su captura. Cuando ese estado más reciente se ha perdido, no afirmar
que restaurar RDB conserva las restricciones que estaban vigentes al fallar el
servidor. Mantener autenticación cerrada hasta disponer de ese estado o hasta que
transcurra la mayor ventana aplicable desde la última autenticación posible.
Actualmente `crates/application/src/identity/service.rs` define 900 segundos para
fallos de contraseña y 90 para reclamos TOTP; volver a contrastarlos con la revisión restaurada. Esta
restricción operativa no cambia los TTL del producto ni recupera datos perdidos.

SQL y Redis se capturan secuencialmente con escritores detenidos; no comparten
una transacción distribuida. La captura Redis puede necesitar memoria adicional
para generar la instantánea: un límite alcanzado debe producir fallo, no una copia
parcial marcada como completa. Una instalación vacía no acredita capacidad ni
recuperación de una instalación con usuarios y datos.

El rollback de aplicación conserva SQL y Redis actuales; no restaura estas copias.
Los controladores instalados bajo `<root>/tools` también deben corresponder a la
revisión aceptada: publicar una etiqueta no actualiza automáticamente esos archivos.
