# Operacion de los runners de CI

## Reparto sin maquinas alojadas por GitHub

CI y Web usan exclusivamente runners del repositorio. Actions conserva la
orquestacion y los artefactos; no aumentar limites de gasto para habilitar
maquinas de GitHub. El reparto es:

| Servidor | Runners y etiquetas | Trabajo | Presupuesto compartido |
| --- | --- | --- | --- |
| VPS3 | Uno, `tt-ci-dedicated` | Rust y generacion de cobertura | 8 CPU, MemoryHigh 28 GiB, MemoryMax 30 GiB |
| VPS3 | Uno, `tt-ci-live-primary` | Tercera particion real | Incluido en el presupuesto de VPS3 |
| VPS1 | Uno, `tt-ci-vps1` | Checks nativos y agregados de CI/Web | 3 CPU, MemoryHigh 4608 MiB, MemoryMax 5 GiB |
| VPS1 | Dos, `tt-ci-mock` | Dos shards de navegador simulado | Incluido en el presupuesto de VPS1 |
| VPS2 | Tres, `tt-ci-live` | Dos particiones reales; un runner de reserva | 6 CPU, MemoryHigh 6.5 GiB, MemoryMax 7 GiB |

Los runners adicionales comparten la cuenta `tt-runner`, pero cada uno tiene
su propio directorio de instalacion y trabajo. Asignar sus unidades systemd
a `user-$(id -u tt-runner).slice`, igual que Docker rootless. No multiplicar
los limites por el numero de servicios. El runner original de VPS2 conserva
ademas `tt-ci-vps2` para las tareas manuales y el modo Rust alternativo.
Evitar solapar una campana manual con la regresion completa.

En VPS2, usar `CPUWeight=1000` en el slice compartido de `tt-runner` para
priorizar CI frente a otros usuarios cuando compitan por CPU. Configurar
`Nice=0` en los tres servicios: el runner original no debe conservar una
prioridad inferior a los adicionales. `MemoryHigh=6656M` deja 512 MiB entre
el umbral de reclamacion y `MemoryMax=7G`; no aumenta el limite duro ni la
cuota de seis CPU. Los pesos reparten capacidad disponible, no garantizan
un tiempo de ejecucion. Aplicar cambios y reiniciar runners cuando no haya
jobs activos; comprobar propiedades efectivas y medir la siguiente campana.

Preparar VPS1 con compilador C, pkg-config, OpenSSL de desarrollo, Git, curl,
Python 3.12+, jq, Rust stable/rustfmt/clippy y Rust 1.88. Los jobs seleccionan
la version Rust requerida. El navegador simulado corre en
`mcr.microsoft.com/playwright:v1.63.0-noble` mediante Docker rootless, con
1.5 CPU, 1536 MiB y 512 MiB de memoria compartida por contenedor. Descargar
la imagen con el usuario del runner antes de habilitarlo. Al actualizar
Playwright en `web/package-lock.json`, actualizar tambien la imagen y
comprobar Chromium; el workflow rechaza una imagen incompatible.

Los runners reales de VPS2 (Ubuntu 24.04) y VPS3 (Ubuntu 22.04) requieren
PostgreSQL 16 con `initdb`/`pg_ctl`, Redis 7,
Python 3.12+, jq, unzip, OpenSSL y las dependencias nativas de Chromium.
Instalar estas dependencias una vez como administrador con la version de
Playwright fijada por el lockfile. Instalar Chromium con el usuario del
runner; el workflow reutiliza esa cache y no ejecuta APT con sudo. Node 24
se selecciona por job con setup-node. Comprobar que el navegador abre una
pagina antes de publicar cambios de infraestructura.

La preparacion manual del checkout, sus directorios `output/` y `output/tmp`
y las pruebas focales deben ejecutarse como `tt-runner`. Si el administrador
crea directorios, asignar propietario y grupo al crearlos, incluidos todos
los padres. Antes de habilitar el runner, comprobar con esa cuenta que puede
crear y eliminar recursivamente un directorio de prueba dentro de `output/`
y que todos los directorios del checkout permiten escritura y recorrido.
Un padre propiedad de root puede impedir la limpieza de `actions/checkout`
aunque `output/tmp` tenga el propietario correcto. Corregir solamente los
permisos afectados, conservar los targets externos y retirar resultados
obsoletos de pruebas focales antes de iniciar la primera campana de Actions.

En Ubuntu 22.04, instalar Redis 7 desde el [repositorio APT oficial de Redis](https://redis.io/docs/latest/operate/oss_and_stack/install/install-stack/apt/)
y conservar la serie mayor con una preferencia APT. El paquete Redis 6.0 de
la distribucion no ofrece `GETDEL`, usado por la autenticacion. Comprobar un
login MFA contra servicios desechables antes de habilitar un runner nuevo;
que el servidor responda PING no valida ese contrato.

`scripts/prepare-ci-workspace.sh` configura un compilador por job y conserva
`~/.cache/tt-ci/target` para los checks secuenciales. Cada runner real usa
`~/.cache/tt-ci/web/$RUNNER_NAME` para evitar compartir un target entre tres
compilaciones simultaneas. Las herramientas y los caches permanecen fuera
del checkout; los backends temporales usan `output/tmp` en disco. Conservar
espacio para la primera compilacion de cada target y medirla separadamente
de las siguientes ejecuciones calientes. La migracion no acredita por si
sola una reduccion de tiempo. Ver [ADR 0052](adr/0052-owned-ci-runners.md).

Un check fallido solicita la cancelacion de los jobs restantes de CI/Web de
la misma revision, rama y evento, despues de conservar diagnosticos. Las
suites dejan de programar pruebas al primer fallo. Una ejecucion cancelada
no satisface los gates ni acredita el inventario completo; corregir y medir
la siguiente cabeza. Ver [ADR 0053](adr/0053-ci-failure-cancellation.md).

Los tres jobs reales usan `TT_WEB_LIVE_SHARD=1/3`, `2/3` o `3/3` para elegir
archivos y preparar sus familias de datos. No combinar esa variable con
`--shard` de Playwright: dividiria dos veces y omitiria pruebas. Sin variable,
`scripts/web-demo.sh` conserva la suite y preparacion completas. Un archivo
nuevo sin clasificar recibe una particion y todos los fixtures como respaldo.
La tercera particion reune administracion y participantes en VPS3; la primera
conserva etapas y plazos en VPS2. Los fixtures siguen a sus escenarios, sin
duplicar su preparacion ni aumentar la cantidad de runners.
Ver [ADR 0054](adr/0054-browser-fixture-partitions.md).

## Servidor dedicado

El modo dedicado requiere Linux x86_64 con 8 vCPU y 32 GB de RAM, una cuenta
exclusiva del runner y Docker con servicios desechables. Registrar un solo
runner de este repositorio con la etiqueta `tt-ci-dedicated`. Un unico proceso
Nextest reparte las pruebas individuales entre dieciseis slots; no instalar
cuatro runners que compilen simultaneamente el mismo workspace. El runner
adicional `tt-ci-live-primary` usa un target distinto para la tercera
particion real. Preparar sus dependencias nativas y navegador como se indica
arriba. Se conservan al menos 2 GiB fuera del limite de CI para el sistema;
recalcular este presupuesto antes de alojar servicios de despliegue.

Limitar conjuntamente el usuario del runner y su Docker rootless a 8 CPU y
30 GiB de RAM, con `MemoryHigh=28G`, incluidos Rust y el navegador real.
Este presupuesto utiliza las ocho vCPU del host dedicado; revisar el reparto de CPU antes de desplegar servicios
adicionales. Mantener una cuenta independiente para despliegues, sin
pertenencia a los grupos ni acceso a las credenciales del runner. El workflow
limita PostgreSQL a 6 GiB y Redis a 128 MiB,
incluidos en el presupuesto anterior. Usar almacenamiento persistente para
`~/.cache/tt-ci` y `output/tmp`; no colocar esas rutas en tmpfs. El espacio de
compilacion debe dimensionarse con la primera ejecucion completa, conservando
los caches compatibles y retirando generaciones obsoletas fuera de trabajos
activos.

En un host configurado con el usuario `tt-runner`, aplicar el presupuesto
conjuntamente a sus procesos y servicios rootless mediante la slice del UID:

```bash
sudo systemctl set-property "user-$(id -u tt-runner).slice" \
  CPUQuota=800% MemoryHigh=28G MemoryMax=30G
```

Cambiar recursos entre campanas. El numero de slots, distinto del numero de
compiladores, se configura en el workflow. Verificar los contadores de CPU,
memoria y conexiones al concluir la siguiente campana.

Preparar Python 3.12+, PostgreSQL **servidor y cliente 16** (`postgres`,
`initdb`, `pg_isready`, `psql`, `pg_dump`, `pg_restore`), Rust stable con `llvm-tools-preview`, las dependencias de
`scripts/setup-document-formats.sh`, Docker y las herramientas habituales de
compilacion. El workflow instala cargo-llvm-cov 0.9.1 y cargo-nextest 0.9.146.
Verificar el acceso a Docker con el usuario del servicio, sus limites de
cgroup y los prerrequisitos antes de habilitarlo. El servidor no necesita
publicar puertos de PostgreSQL o Redis fuera de loopback.

El job dedicado inicia PostgreSQL nativo mediante una unidad transitoria de
`systemd --user`, dentro de la slice presupuestada del usuario. Verificar que
`systemctl --user` funciona en el entorno del servicio runner y que
`XDG_RUNTIME_DIR` apunta al directorio de ese UID. El helper exige PostgreSQL16
con limite efectivo de 6GiB, usa credenciales SCRAM nuevas y tres bases por
slot, y elimina el cluster al terminar o cancelar. El paso de limpieza final
atiende un marcador residual; el limite de vida de la unidad cubre una muerte
abrupta del supervisor. No habilitar un servidor PostgreSQL persistente para
CI. Tanto las pruebas SQL previas del calendario como Nextest invocan el
supervisor, cada uno con su propio cluster nuevo. Se conserva Redis en
contenedor y el modo no dedicado sin cambios. Ver
`docs/adr/0055-native-ci-postgres.md`.

La variable del repositorio `TT_CI_DEDICATED=true` activa este modo. Mientras
este ausente o tenga otro valor, siguen operando los dos runners actuales.
Cambiarla entre ejecuciones, despues de validar el host y antes de iniciar la
siguiente campana. No modificarla a mitad de una ejecucion: determina tanto
el destino de las pruebas como el numero de artefactos exigidos por Coverage.

Validacion focal del circuito de cobertura con servicios reales, un compilador
y un slot local, tras preparar `output/tmp` en disco y las herramientas:

```bash
TMPDIR="$PWD/output/tmp" CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
  bash scripts/test-backends.sh python3 -B scripts/tests/check_nextest_pipeline.py
```

Despues, ejecutar la regresion completa en el servidor dedicado. Comparar
compilacion, ejecucion y reporte con cache frio y caliente; descargar
`test-durations` para localizar las pruebas que dominan el tiempo sin sondear
continuamente Actions. Una campana verde acredita correccion; el objetivo de
10-20 minutos solo se acredita con una medicion completa. Aumentar los slots
requiere revisar CPU, memoria, conexiones PostgreSQL y los resultados de esa
campana. Dieciseis slots con ocho CPU aprobaron Rust y cobertura en 11m54s,
con el mismo limite de memoria. La estabilidad del conjunto de checks se
confirma para cada cabeza publicada. El navegador reparte sus suites
entre dos jobs para API simulada y tres para servicios reales; cada job conserva un solo worker
y servicios aislados. Todos los jobs de cada suite deben aprobar
para que apruebe el check agregado. Ver
[ADR 0051](adr/0051-browser-ci-shards.md).

Ver [ADR 0050](adr/0050-isolated-test-slots.md).

### Comprobacion de slots simultaneos

En el servidor, con PostgreSQL y Redis desechables ya preparados y sus URLs
exportadas, ejecutar:

```bash
python3 -B scripts/tests/check_nextest_pipeline.py --slots 4
```

La comprobacion espera que los cuatro slots esten activos a la vez y toma la
misma clave de bloqueo consultivo en cada base. Si comparten base, el limite
de espera provoca un fallo. Verifica tambien cobertura fresca y resultados
JUnit. Su duracion no representa la regresion del proyecto. En la computadora
local mantener el valor predeterminado de un slot.

## Python

Los runners Linux requieren Python **3.12 o posterior**. La selección debe
funcionar tanto en los pasos normales como dentro de `/bin/sh` con el entorno
vacío: el supervisor de formatos elimina las variables del worker antes de
ejecutarlo. Un alias, un entorno virtual o modificar solamente el PATH del
servicio del runner no satisface esa condición.

En AlmaLinux 9, instalar la versión compatible y exponerla en la ruta local
de programas administrados por el operador:

```bash
sudo dnf install -y python3.12
test ! -e /usr/local/bin/python3 && test ! -L /usr/local/bin/python3
sudo ln -s /usr/bin/python3.12 /usr/local/bin/python3
```

El enlace se crea una sola vez. Si ya existe, comprobar primero su destino
y versión; no reemplazar otro intérprete sin revisar quién lo utiliza.
`/usr/bin/python3` permanece bajo control de la distribución. En Ubuntu 24.04,
el paquete `python3` ya proporciona Python 3.12; instalarlo con
`sudo apt-get install python3` si falta.

Ubuntu 22.04 incluye Python 3.10 para el sistema. Una instalacion separada de
CPython 3.12.14, compilada desde el tarball XZ oficial, puede ubicarse en
`/opt/cpython/3.12.14`; exponer su ejecutable como `/usr/local/bin/python3`.
Antes de compilar, verificar el SHA-256 del archivo:
`5c8462af5790baf43a321a1559dbe0db06d1be4300fb85fb53c40060668e548a`.
Conservar `/usr/bin/python3` para los paquetes del sistema. Esta instalacion
separada requiere aplicar sus actualizaciones de seguridad; no la actualiza
APT. La version y el hash provienen de la
[publicacion oficial](https://www.python.org/downloads/release/python-31214/).

Ejecutar esta comprobación con el usuario del runner al preparar o migrar un
host, incluida cualquier imagen de contenedor usada como runner:

```bash
env -i /bin/sh -c 'python3 -' <<'PY'
import fcntl
import sys
assert sys.version_info >= (3, 12), "Python 3.12+ is required with an empty environment"
assert hasattr(fcntl, "F_SETPIPE_SZ"), "Python must expose Linux pipe sizing"
print(sys.executable, sys.version.split()[0])
PY
```

El workflow ejecuta esta misma comprobación antes de compilar. Una selección
incorrecta del intérprete debe fallar en ese paso, conservando sin cambios
la prueba de transferencia por pipes y el aislamiento del worker. Los
límites y la distribución de trabajo se describen en
[ADR 0049](adr/0049-parallel-coverage-runners.md).
