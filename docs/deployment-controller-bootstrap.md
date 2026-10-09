# Bootstrap privado del instalador de controladores

## Alcance y estado

El [launcher](deployment-controller-launcher.md) admite la entrada fija
`controller_installation.py`. Permite ejecutar el instalador desde una generación
aprobada distinta de los controladores que se van a sustituir, sin otro loader ni
imports desde el directorio de trabajo.

El transporte de argumentos y fuentes tiene aceptación local. El núcleo y la CLI
aprobaron también el ensayo nativo con cuatro servicios inocuos descrito en el
[informe de verificación](verification-report.md). Esa aceptación aislada no
acredita una instalación ni aceptación operativa del producto en VPS3.
La instalación real posterior del 9 de octubre de 2026 se registra por separado
en [instalación de controladores](deployment-controller-installation.md).
No sustituye los contratos de [publicación](deployment-controller-publication.md),
[aprobación](deployment-controller-approval.md) y
[cierre de entradas](deployment-controller-entry-gate.md).

## Dos raíces y un candidato separado

```text
area-privada/
  source/                     exportacion de la revision aprobada
  controller_launcher.py      launcher confiable, fuera de tools
  bootstrap/
    tools/*.py                codigo que ejecuta la instalacion
  candidate/
    manifest.json
    sources/*.py              codigo que se publicara
```

La primera opción `--root`, antes de `--`, pertenece al launcher e identifica
`bootstrap`. La segunda, después de `--`, pertenece al instalador e identifica
el despliegue real. No son intercambiables. El launcher conserva cwd y argumentos;
no deduce ni añade la raíz del despliegue.

`candidate` debe contener exactamente `manifest.json` y `sources`. No colocar
allí `tools`, el launcher, recibos ni autorizaciones. El bootstrap debe permanecer
fuera de `DEPLOYMENT/tools` y de los directorios que se intercambiarán. Las rutas
son absolutas y canónicas, sin enlaces simbólicos. Los directorios privados son
propios y modo 0700; los archivos, propios, regulares, modo 0600 y con un solo
enlace. Se requiere Linux y un intérprete absoluto aprobado de Python 3.11 o
posterior, invocado con `-I -B -S`.

Hay tres huellas diferentes: la del archivo launcher, la del inventario completo
del bootstrap y la del manifiesto candidato. La intención liga además el inventario
candidato instalado y el estado anterior del despliegue. Todos los valores
esperados proceden de la selección revisada; no se reemplazan por los observados
cuando hay discrepancia. Un digest comprueba bytes, no aprueba su procedencia.

## Preparación desde fuentes versionadas

La siguiente receta usa una sola revisión aprobada para bootstrap y candidato.
No copia el árbol de trabajo ni los controladores instalados. Requiere Git, tar,
install y sha256sum en el equipo de preparación. Las variables siguientes deben
identificar el repositorio disponible, una revisión completa de 40 dígitos, un
área nueva y los tres SHA-256 externos aprobados, de 64 dígitos minúsculos:

```bash
BOOTSTRAP_PYTHON=/ruta/absoluta/aprobada/python3
CONTROLLER_REPO=/ruta/absoluta/repositorio
CONTROLLER_REVISION=REVISION_COMPLETA_APROBADA
CONTROLLER_WORK=/ruta/absoluta/area-nueva
LAUNCHER_SHA256=SHA256_LAUNCHER_APROBADO
BOOTSTRAP_SHA256=SHA256_INVENTARIO_BOOTSTRAP_APROBADO
MANIFEST_SHA256=SHA256_MANIFIESTO_CANDIDATO_APROBADO

set -euo pipefail
umask 077
"$BOOTSTRAP_PYTHON" -I -B -S - "$CONTROLLER_REVISION" "$LAUNCHER_SHA256" \
  "$BOOTSTRAP_SHA256" "$MANIFEST_SHA256" <<'PY'
import re
import sys
if (sys.version_info < (3, 11) or not re.fullmatch(r"[0-9a-f]{40}", sys.argv[1])
        or any(not re.fullmatch(r"[0-9a-f]{64}", value) for value in sys.argv[2:])):
    raise SystemExit("Approved revision, pins and Python 3.11 or later are required")
PY
mkdir -m 700 "$CONTROLLER_WORK"
mkdir -m 700 "$CONTROLLER_WORK/source" "$CONTROLLER_WORK/bootstrap" \
  "$CONTROLLER_WORK/candidate"
mkdir -m 700 "$CONTROLLER_WORK/bootstrap/tools" "$CONTROLLER_WORK/candidate/sources"
git -C "$CONTROLLER_REPO" archive "$CONTROLLER_REVISION" \
  ops/deploy ops/controller_launcher.py | tar -xf - -C "$CONTROLLER_WORK/source"
install -m 600 "$CONTROLLER_WORK/source/ops/controller_launcher.py" \
  "$CONTROLLER_WORK/controller_launcher.py"
printf '%s  %s\n' "$LAUNCHER_SHA256" "$CONTROLLER_WORK/controller_launcher.py" | sha256sum -c -
for controller_source in "$CONTROLLER_WORK/source/ops/deploy/"*.py; do
  install -m 600 "$controller_source" "$CONTROLLER_WORK/bootstrap/tools/"
  install -m 600 "$controller_source" "$CONTROLLER_WORK/candidate/sources/"
done
```

El manifiesto usa el formato versionado del publicador: JSON ASCII, claves
ordenadas, separadores compactos y un salto de línea final. Este bloque sólo
construye ese archivo en el área nueva y contrasta los pins proporcionados; no
importa controladores ni genera una intención de instalación:

```bash
"$BOOTSTRAP_PYTHON" -I -B -S - "$CONTROLLER_WORK" "$CONTROLLER_REVISION" \
  "$BOOTSTRAP_SHA256" "$MANIFEST_SHA256" <<'PY'
import hashlib
import json
from pathlib import Path
import re
import sys

work, revision, bootstrap_sha, manifest_sha = sys.argv[1:]
work = Path(work)
if not re.fullmatch(r"[0-9a-f]{40}", revision):
    raise SystemExit("A complete approved source revision is required")

def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")

def inventory(directory):
    result = {}
    for path in sorted(directory.iterdir()):
        raw = path.read_bytes()
        result[path.name] = {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
    return result

files = inventory(work / "candidate/sources")
if files != inventory(work / "bootstrap/tools") or "controller_installation.py" not in files:
    raise SystemExit("The complete bootstrap and candidate source sets must agree")
if hashlib.sha256(encoded(files)).hexdigest() != bootstrap_sha:
    raise SystemExit("Source inventory differs from the approved bootstrap")
manifest = encoded({"format": "qadra-controller-publication", "version": 1,
                    "source_revision": revision, "files": files})
if hashlib.sha256(manifest).hexdigest() != manifest_sha:
    raise SystemExit("Candidate manifest differs from its approved digest")
with (work / "candidate/manifest.json").open("xb") as stream:
    stream.write(manifest)
PY
```

El launcher y el publicador vuelven a exigir sus límites, permisos e inventarios
estrictos al admitir la operación. No reducir la copia a los módulos que parezcan
necesarios ni retirar archivos para hacer coincidir un hash. En esta receta los
dos mapas de fuentes coinciden porque proceden de la misma revisión; el hash del
manifiesto sigue siendo otro valor.

## Invocación y reentrada explícitas

Antes de operar, se necesita la intención revisada para las rutas e identidades
finales: despliegue, candidato, launcher, intérprete y estado anterior. Esta receta
no la fabrica ni inspecciona un destino discrepante para generar su aprobación.
`INSTALL_INTENT` y, cuando corresponda, `REOPEN_AUTHORITY` son archivos privados
existentes, con sus hashes externos. `INSTALL_OPERATION` es el UUID canónico no
nulo de esa misma intención; `DEPLOYMENT_ROOT` es su raíz exacta.

La invocación inicial para la intención y operación aprobadas es:

```bash
"$BOOTSTRAP_PYTHON" -I -B -S "$CONTROLLER_WORK/controller_launcher.py" \
  --root "$CONTROLLER_WORK/bootstrap" --inventory-sha256 "$BOOTSTRAP_SHA256" \
  --entrypoint controller_installation.py -- \
  --root "$DEPLOYMENT_ROOT" --operation-id "$INSTALL_OPERATION" \
  --intent "$INSTALL_INTENT" --intent-sha256 "$INSTALL_INTENT_SHA256" --timeout 600
```

Una respuesta perdida exige reconciliar el mismo intento, conservando UUID,
fuentes, rutas y pins. No autoriza una operación nueva ni reapertura automática.
La reapertura añade la autorización explícita que liga el recibo cerrado:

```bash
"$BOOTSTRAP_PYTHON" -I -B -S "$CONTROLLER_WORK/controller_launcher.py" \
  --root "$CONTROLLER_WORK/bootstrap" --inventory-sha256 "$BOOTSTRAP_SHA256" \
  --entrypoint controller_installation.py -- \
  --root "$DEPLOYMENT_ROOT" --operation-id "$INSTALL_OPERATION" \
  --intent "$INSTALL_INTENT" --intent-sha256 "$INSTALL_INTENT_SHA256" \
  --reopen "$REOPEN_AUTHORITY" --reopen-sha256 "$REOPEN_AUTHORITY_SHA256" --timeout 600
```

Conservar los recibos de la operación y el área privada para su reentrada. No
retirar máscaras, caché o generaciones manualmente como continuación de un error.
La ejecución del bootstrap no cambia por sí sola los puntos de entrada
administrativos de despliegue y renovación de CRL; éstos también deben seleccionar
la generación aprobada conforme al procedimiento operativo de instalación.

## Evidencia local

El RED del bootstrap ejecutó dos casos en 0.091 s; el caso positivo rechazó la
nueva entrada por no estar admitida. Tras añadir sólo esa entrada al launcher,
bootstrap y regresiones de launcher/ciclo de vida aprobaron **8/8 en 1.703 s**.
Los hijos son Python inocuos: comprueban dos raíces, argumentos literales, imports
fijados y rechazo antes de ejecución ante hash incorrecto. No ejecutan el núcleo
instalador, systemd ni servicios del despliegue.

```bash
python3 -B -m unittest discover -s scripts/tests -p 'test_deploy_controller_bootstrap.py'
python3 -B -m unittest discover -s scripts/tests -p 'test_deploy_controller_launcher*.py'
```
