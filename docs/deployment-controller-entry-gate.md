# Cierre persistente de entradas de controladores

## Alcance

`ops/deploy/controller_gate.py` implementa `close_entries`, una primitiva interna
que sustituye cuatro fragmentos atestados por máscaras persistentes de systemd.
Conserva los originales y exige una observación nueva del gestor antes de devolver
el recibo `gated`. Las unidades son `qadra-web.service`, `qadra-api.service`,
`qadra-postgres.service` y `qadra-redis.service`.

El cierre impide futuras entradas por esas unidades una vez que el gestor confirma
las máscaras. Los procesos ya activos pueden continuar: el recibo no acredita
parada, drenaje de solicitudes, ausencia de listeners ni quiescencia. La
[parada acotada de servicios](deployment-restore-quiesce.md) tiene otro contrato.
Esta primitiva no mueve caché, publica fuentes, instala candidatos, reabre
servicios ni modifica configuración privada, datos o PKI.

## Autoridad y precondiciones

```python
close_entries(root, operation_id, *, target_path,
              expected_target_sha256, timeout=30)
```

La raíz y el descriptor son rutas absolutas canónicas. Se requiere un UUID
canónico no nulo y el SHA-256 externo del descriptor `qadra-restore-target`,
guardado en `receipts` de esa raíz. El descriptor liga identidad UID/dispositivo/
inode, huella de configuración, puertos derivados de ella, ejecutable systemctl,
contexto local HOME/XDG/DBUS y huellas de los cuatro fragmentos. La huella controla
integridad; su cálculo por sí solo no aprueba una instalación arbitraria.

La raíz, el directorio de unidades y los directorios propios deben pertenecer al
usuario y tener modo 0700. Los fragmentos originales son archivos regulares
propios 0600, con un solo enlace, en el mismo sistema de archivos que la evidencia
retenida. Se exige un `deploy.lock` existente. Se rechazan enlaces ajenos,
drop-ins, fragmentos discordantes y directorios cuya identidad cambió.

Antes de modificar una entrada se comprueban las cuatro unidades cargadas y sus
procesos por UID, cgroup y tiempo de inicio. El cierre conserva los bytes de sus
comandos sin regenerarlos; un fragmento puede carecer de `ExecStartPre`. No añade
prestarts ni amplía el contrato del renderer de unidades.

La operación retiene un único lock de despliegue, no reentrante. Sólo usa `show`
y `daemon-reload` del systemctl absoluto atestado. Cada comando está acotado a
diez segundos y al presupuesto total explícito, entre uno y sesenta segundos.
El entorno contiene el contexto local validado y `LC_ALL=C`; los errores y los
recibos no incluyen secretos de configuración ni diagnósticos arbitrarios.

## Persistencia y reentrada

El estado propio reside en `maintenance/controller-installation`:

| Archivo | Contenido |
| --- | --- |
| `active.json` | UUID de operación y hash del descriptor |
| `<UUID>/journal.json` | Intención, identidades, originales, máscaras y estado |
| `<UUID>/slots/<unidad>` | Máscara preparada o fragmento original retenido |

La secuencia normal es `prepared`, `masking`, `reload_pending` y `gated`. El diario
preparado captura las cuatro máscaras propias y los originales. El marcador
activo y el estado `masking` son durables antes del primer intercambio.

Cada `renameat2(RENAME_EXCHANGE)` intercambia un original con su enlace propio a
`/dev/null`, preservando el inode, bytes, UID, modo y mtime del original. No existe
un intervalo en el que el nombre de unidad desaparezca. Se sincronizan ambos
directorios padres. Si Linux no ofrece el intercambio, se rechaza sin fallback.

La reentrada acepta sólo los dos pares exactos registrados: original en su nombre
y máscara en el slot, o máscara instalada y original retenido. Una máscara ya
instalada no se intercambia de vuelta. Un enlace sustituto hacia el mismo
`/dev/null` tampoco se adopta. Otro UUID, descriptor u objeto ajeno causa rechazo
sin limpieza ni rollback automático.

Después de observar las cuatro máscaras en disco se registra `reload_pending`,
se recarga el gestor y se exige `LoadState=masked`, `UnitFileState=masked`, ausencia
de drop-ins y ausencia de recarga pendiente para las cuatro unidades. Sólo entonces
se persiste `gated`. Incluso una reentrada con ese estado vuelve a comprobar las
identidades, recarga y observa las cuatro unidades.

Un fallo de recarga, respuesta perdida, plazo agotado o fsync final incierto no
devuelve éxito. Puede dejar un diario `gated` visible, pero ese archivo no sustituye
la reobservación. Una preparación interrumpida antes de su primer diario durable
se conserva para inspección; no autoriza intercambios ni adopción de temporales.
La API no retira el marcador, las máscaras ni los originales.

## Evidencia focal y aceptación nativa pendiente

El 3 de octubre de 2026, seis pruebas reprodujeron la ausencia del módulo. Tras
implementar el contrato aprobaron **6/6 en 0.963 s**. Cubren cierre durable bajo
lock, caída de un hijo después de dos intercambios, reentrada exacta, originales
retenidos, rechazo de drop-ins y objetos sustitutos, recarga fallida, pérdida de
confirmación, plazo agotado y sincronización final incierta. Usan archivos y
operaciones Linux reales; el gestor y los observadores de procesos son dobles.

```bash
python3 -B -m unittest discover -s scripts/tests -p 'test_deploy_controller_gate*.py'
```

La primera ejecución de `scripts/tests/native_controller_gate_acceptance.py`
completó las comprobaciones del cierre, pero falló durante cleanup: tras cargar
las máscaras, systemd informó `Result=success` y `ExecMainCode=0` para los cuatro
workers ya ausentes. Ese valor no conserva un estado de salida del proceso. Los
cuatro recibos de cierre registraron SIGTERM. El coordinador retiró sólo las
máscaras de identidad comprobada y restauró el modo original del directorio;
conservó la evidencia sintética. La aceptación completa sigue pendiente de repetir.

El ensayo requiere opt-in y exclusivamente la
cuenta no privilegiada `tt-runner`, con las cuatro unidades inexistentes y sin
fragmentos, procesos, drop-ins ni enlaces de dependencia previos. Aborta si el
directorio de unidades no es propio 0700; el script no cambia permisos globales.
Si un coordinador necesita ajustar temporalmente ese modo, debe fijar antes su
inode y restaurar el permiso anterior sólo tras comprobar la misma identidad y
la conservación del inventario ajeno. Esa adaptación es externa al ensayo.

```bash
TT_CONTROLLER_GATE_NATIVE=1 python3 -B scripts/tests/native_controller_gate_acceptance.py -v
```

El ensayo crea cuatro workers inocuos con memoria y vida limitadas, comprueba
una caída parcial y una confirmación perdida después de una recarga real, y
reentra mediante intérpretes nuevos. Exige que PID, tiempo de inicio y listeners
de los workers sigan iguales durante el cierre. El cleanup verifica identidades
antes de parar sus procesos y retirar sus propios archivos; una divergencia
conserva la evidencia y los objetos ajenos. No se ejecuta en la cuenta del
despliegue ni reinicia el gestor o el host.

Cada recibo cooperativo de cierre liga PID, UID, tiempo de inicio y cgroup al
worker observado. Cleanup exige ese recibo después de cerrar el socket, ausencia
de procesos y listeners propios, estado inactivo con cgroup vacío y
`Result=success`. Rechaza metadata que declare una terminación anormal, pero no
interpreta `ExecMainCode=0` como prueba de salida normal del sistema operativo.
La recepción del recibo y la desaparición del proceso acreditan cooperación y
ausencia; el ensayo no recupera un código de salida que el gestor dejó de exponer.

Su alcance es persistencia en disco, `daemon-reload` y reentrada con systemd real.
No acreditará comportamiento tras reboot, instalación de controladores en VPS3
ni reapertura. La [publicación](deployment-controller-publication.md), la
[aprobación de generación](deployment-controller-approval.md) y el
[launcher](deployment-controller-launcher.md) conservan sus fronteras separadas.


### Aceptación nativa corregida

El recorrido desechable posterior aprobó 1/1 en 1.091 s (1.219 s incluyendo
preparación externa). Conservó los cuatro trabajadores durante el cierre de
entradas, sobrevivió a la muerte del intérprete tras dos intercambios y a la
pérdida de confirmación del reload, y aceptó dos reentradas nuevas. La limpieza
retiró únicamente sus unidades y procesos, preservó el inventario ajeno y
restauró el modo previo del directorio del usuario. No hubo reinicio del host,
parada del gestor de CI ni cambio de los servicios del usuario de despliegue.
