# Parada observada antes de restaurar

## Alcance

`ops/deploy/restore_quiesce.py` añade una entrada interna para cerrar admisión,
detener los cuatro servicios propios y observar que ya no tienen procesos ni
listeners. No restaura ni promueve datos, no inicia servicios y no retira la
barrera. Un resultado `stopped` no acredita restauración completada ni autoriza
reabrir. La instalación del controlador en VPS3 conserva su aceptación separada.

## Identidad explícita

`quiesce(root, operation_id, target_path=..., expected_target_sha256=...,
timeout=240)` recibe un UUID canónico no nulo y un descriptor privado fijado por
SHA-256. El administrador debe preparar e inspeccionar ese descriptor bajo el
bloqueo de despliegue. El digest fija bytes; no autentica por sí solo su origen.

El descriptor liga la ruta, propietario, dispositivo e inode del root; el hash y
tamaño de `config/settings.json`; el ejecutable absoluto de systemctl; los
fragmentos de las cuatro unidades, directorio de trabajo y cgroups esperados.
También liga HOME, el directorio runtime y el bus de usuario local actual.
Los cuatro puertos se derivan de los settings, no de una selección arbitraria.
Los datos sensibles de los settings no se copian al descriptor ni al diario.
Se rechazan redirecciones, permisos inseguros, identidades distintas, drop-ins,
unidades que necesitan recargarse y observaciones incompletas.

## Secuencia y reentrada

1. Adquirir una vez `deploy.lock` y mantenerlo durante toda la operación.
2. Validar la identidad fijada y observar unidades y procesos propios antes de
   detener nada.
3. Publicar y sincronizar `maintenance/restore/active.json` y el diario
   `maintenance/restore/<operation_id>/journal.json` en estado `closing`.
4. Detener web y API; exigir estado inactivo, terminación normal, cgroups vacíos
   y ausencia de listeners antes de detener PostgreSQL y Redis.
5. Reobservar los cuatro servicios y sus puertos, y publicar `stopped` mediante
   sincronización de archivo y directorio.

El presupuesto monotónico total admite de 90 a 300 segundos. Una parada sólo
comienza si quedan al menos los 90 segundos de gracia de las unidades. Cada
consulta y observación consume ese mismo presupuesto. Un timeout, señal forzada,
salida anómala o error al sincronizar conserva la barrera y devuelve fallo.
El controlador nunca busca procesos para matarlos ni adopta listeners ajenos.

Reentrar con el mismo UUID y descriptor vuelve a observar el estado real,
incluso si el diario ya dice `stopped`. Una unidad propia que volvió a arrancar
se detiene de nuevo siguiendo el protocolo. Otro UUID o digest no adopta el
recibo. No se deshace la parada mediante un rollback de aplicación.

## Frontera de observación

El lector Linux comprueba cgroup v2, PID, UID, inicio del proceso y pertenencia
antes y después de la lectura. Rechaza cambios, inventarios ambiguos y límites
excedidos. Los listeners IPv4 e IPv6 se observan en el namespace de red del
proceso actual; las rutas nativas usan su PID numérico para evitar los alias
normales de `/proc/net` y `/proc/self`. No se abren conexiones a los servicios.

Estas observaciones no son una frontera contra un administrador Unix concurrente
ni prueban que cada cliente recibió su respuesta antes del cierre. La parada
normal acredita salida de los procesos observados, no una política de recuperación
de datos ni un punto de recuperación autorizado.

## Evidencia y pendientes

Los 13 casos del protocolo reprodujeron primero la ausencia del controlador;
los ocho del lector reprodujeron la ausencia del observador. Los 21 aprobaron
en 0.223 s, con archivos, locks y sincronización reales y fronteras de comandos
y observación controladas. Las seis regresiones existentes de la barrera
aprobaron en 0.261 s. No se modificaron los casos existentes ni sus aserciones.
Una aceptación nativa adicional aprobó 1/1 en 0.215 s usando una unidad de
usuario desechable con 64 MiB y vida máxima de 60 s. Verificó PID, inicio, UID,
cgroup, un proceso hijo y listeners IPv4/IPv6 reales. La parada normal confirmó
salida del hijo, recibo de terminación y desaparición de cgroup y puertos. Ese
ensayo comprueba los observadores con systemd real; no ejecuta el controlador
completo sobre los cuatro servicios de la instalación ni restaura bases.

Para reproducir únicamente esa aceptación aislada:

```bash
TT_RESTORE_OBSERVER_NATIVE=1 python3 -B scripts/tests/native_restore_observe_acceptance.py -v
```

Requiere Linux, cgroup v2, bus de usuario local y systemd. Crea sólo una unidad
con UUID propio; no opera sobre unidades `qadra-*` existentes.

Siguen pendientes staging privado, validación de los datos restaurados,
promoción durable, reconciliación de esa promoción, decisión de fuente Redis y
condiciones de reapertura. Este controlador no implementa esas etapas ni modifica
SQL, Redis, PKI o configuración de la instalación. Véanse la
[barrera de admisión](deployment-restore-fence.md), la
[captura consistente](deployment-restore-capture.md) y la
[compatibilidad del respaldo](deployment-restore-compatibility.md).
