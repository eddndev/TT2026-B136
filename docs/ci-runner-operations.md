# Python para los runners de CI

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
