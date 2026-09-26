# Operacion de los runners de CI

## Servidor dedicado

El modo dedicado requiere Linux x86_64 con 8 vCPU y 32 GB de RAM, una cuenta
exclusiva del runner y Docker con servicios desechables. Registrar un solo
runner de este repositorio con la etiqueta `tt-ci-dedicated`. Un unico proceso
Nextest reparte las pruebas individuales entre seis slots; no instalar
cuatro runners que compilen simultaneamente el mismo workspace.

Limitar conjuntamente el usuario del runner y su Docker rootless a 7 CPU y
26 GiB de RAM, con `MemoryHigh=24G`. Mantener libres los recursos restantes
para el sistema y futuros servicios. Mantener una cuenta independiente para
despliegues, sin pertenencia a los grupos ni acceso a las credenciales del
runner. El workflow limita PostgreSQL a 2 GiB y Redis a 128 MiB,
incluidos en el presupuesto anterior. Usar almacenamiento persistente para
`~/.cache/tt-ci` y `output/tmp`; no colocar esas rutas en tmpfs. El espacio de
compilacion debe dimensionarse con la primera ejecucion completa, conservando
los caches compatibles y retirando generaciones obsoletas fuera de trabajos
activos.

Preparar Python 3.12+, PostgreSQL **cliente 16** (`psql`, `pg_dump`,
`pg_restore`), Rust stable con `llvm-tools-preview`, las dependencias de
`scripts/setup-document-formats.sh`, Docker y las herramientas habituales de
compilacion. El workflow instala cargo-llvm-cov 0.9.1 y cargo-nextest 0.9.146.
Verificar el acceso a Docker con el usuario del servicio, sus limites de
cgroup y los prerrequisitos antes de habilitarlo. El servidor no necesita
publicar puertos de PostgreSQL o Redis fuera de loopback.

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
10-20 minutos solo se acredita con una medicion completa. Aumentar de cuatro
a seis u ocho slots requiere revisar CPU, memoria, conexiones PostgreSQL y
los resultados de esa campana. El navegador reparte sus suites entre dos jobs independientes por suite;
cada job conserva un solo worker y servicios aislados. Ambos deben aprobar
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
