# Lanzamiento de una generación aprobada de controladores

## Alcance

`ops/controller_launcher.py` es una primitiva independiente, situada fuera de
`ops/deploy` y del directorio instalado `tools`. Fija un directorio mediante un
descriptor antes de cargar cualquier controlador y comprueba su inventario
completo contra un SHA-256 aprobado por el llamador. Los imports iniciales y
tardíos usan esa misma generación aunque el publicador intercambie `tools`.

La [publicación de controladores](deployment-controller-publication.md) conserva
la generación anterior y tiene su propia aceptación. El launcher no publica
fuentes, instala unidades, selecciona por sí solo una generación aprobada ni
abre servicios. Su aceptación local no acredita una instalación en VPS3.

## Contrato de invocación

La API es:

```python
launch(root, entrypoint, arguments, *, expected_inventory_sha256)
```

`root` debe ser un `Path` absoluto. `arguments` es una lista de cadenas. Las
entradas admitidas son `runtime.py`, `release.py`, `restore_fence.py`,
`renew_crl.py` y `controller_installation.py`. Los demás módulos son dependencias
internas, no entradas del launcher. Se requiere un intérprete absoluto aprobado
de Python 3.11 o posterior, nuevo, con aislamiento y sin importar site.
La forma de la CLI es:

```text
/ruta/absoluta/aprobada/python3 -I -B -S /ruta/estable/controller_launcher.py \
  --root /raiz/privada --inventory-sha256 SHA256_APROBADO \
  --entrypoint ENTRADA -- ARGUMENTOS_DEL_CONTROLADOR
```

Es una forma de interfaz, no una instrucción para sustituir las unidades
instaladas. La ruta estable debe permanecer fuera de los directorios publicados.
La instalación y revisión del propio launcher tienen una frontera independiente.

El hash obligatorio tiene 64 dígitos hexadecimales minúsculos. Corresponde al mapa
completo de basename a `{bytes, sha256}`, serializado como JSON ASCII con claves
ordenadas, separadores compactos y un salto de línea final. Es el mismo formato
que el publicador usa para `installed_sha256`; no es el hash del manifiesto del
staging ni el de un archivo aislado. El llamador debe seleccionar y aprobar la
fuente antes de proporcionar ese valor. Calcular un hash de cualquier directorio
presente no establece su procedencia ni convierte su contenido en aprobado.

Las cadenas posteriores a `--` pasan sin interpolación. El launcher no añade la
raíz a los argumentos del controlador: runtime y restore_fence la reciben como
argumento posicional; release, renew_crl y controller_installation utilizan
`--root`. El llamador sigue siendo responsable de proporcionar la raíz prevista
para esa operación. El [bootstrap del instalador](deployment-controller-bootstrap.md)
usa dos raíces: la del launcher fija el código privado; la del instalador, tras
`--`, identifica el despliegue. Su candidato `sources` permanece separado de
`bootstrap/tools`. La CLI del instalador tiene aceptación local y un ensayo
nativo con servicios desechables. Esa evidencia no acredita instalación ni
reapertura del producto en VPS3; véase el
[contrato de instalación](deployment-controller-installation.md).

## Admisión por descriptor

El proceso exige UID y GID reales y efectivos iguales. Recorre la raíz sin seguir
enlaces y abre `tools` mediante descriptores de directorio. Ambos deben pertenecer
al UID actual y tener modo 0700. Los archivos deben ser regulares, propios, modo
0600 y con un solo enlace. La inspección no permite enlaces simbólicos, enlaces
duros adicionales, FIFO, subdirectorios, bytecode ni entradas ajenas a Python.

El inventario admite entre 1 y 64 fuentes, de 1 byte a 256 KiB cada una y hasta
8 MiB en conjunto. Las lecturas son acotadas y comparan identidad, tamaño y
metadatos antes y después. Se vuelven a comprobar el directorio y sus nombres;
la entrada solicitada debe pertenecer al inventario exacto aprobado. Un cambio
observado o una discrepancia provoca rechazo antes de ejecutar el controlador.

`runpy` ejecuta la entrada como `__main__` mediante
`/proc/self/fd/<descriptor>/<entrada>`. Esa ruta fija `__file__`, `sys.argv[0]`
y el primer elemento de `sys.path`. No se añade como alternativa la ruta nominal
de `tools`, una ruta relativa ni el directorio actual. La biblioteca estándar se
carga desde el intérprete aislado; módulos de controladores ya importados causan
rechazo. Se deshabilita la escritura de bytecode antes de ejecutar fuentes.

El launcher no retiene `deploy.lock` durante la ejecución. Los controladores de
release y renovación adquieren ese lock para sus propias operaciones. El descriptor
mantiene su generación mientras el publicador cambia la ruta nominal. La captura
que obtiene `Path(__file__).resolve().parent` sigue resolviendo el directorio
anterior conservado por la publicación, no el nuevo `tools`.

## Proceso y terminación

La entrada se ejecuta en el mismo PID, con cwd y entorno conservados. Las señales
llegan directamente a ese proceso. Al retornar o propagar una excepción o
`SystemExit`, se restauran los argumentos, las rutas de búsqueda y la opción de
bytecode del llamador, y se cierran los descriptores propios. Los descriptores
tienen `CLOEXEC`: no sobreviven a un reemplazo mediante `os.execve`.

No se descargan los módulos importados al retornar. Cada invocación debe usar un
intérprete nuevo; la API no ofrece alternar generaciones dentro de una máquina
virtual Python reutilizada. Tampoco hace inmutables los archivos frente a cambios
arbitrarios de la misma cuenta Unix ni elimina generaciones conservadas.

## Verificación focal

Los seis casos fallaron primero por ausencia del launcher. La implementación
aprobó **6/6 en 1.669 s** con hijos Python locales inocuos. Cubren imports A/A
después de un intercambio real, inventario completo resuelto desde `__file__`,
las cuatro entradas, argumentos y entorno, rechazos de inventario y permisos,
cierre tras retorno/excepción/`SystemExit`, `execve` sin heredar el descriptor y
terminación por SIGTERM. Los casos de UID/GID incompatibles sustituyen observadores
en el hijo; no cambian cuentas reales.

```bash
python3 -B -m unittest discover -s scripts/tests -p 'test_deploy_controller_launcher*.py'
```

La prueba de `execve` utiliza otro intérprete Python, no el binario Rust. No se
ejecutaron los controladores operativos, servicios ni acciones remotas en esta
aceptación. La caracterización previa de imports y las pruebas del publicador
conservan su evidencia separada.

La quinta entrada tiene un RED independiente: dos casos en 0.091 s, con rechazo
del caso positivo por entrada no admitida. Después de añadirla, esos casos y las
regresiones anteriores aprobaron **8/8 en 1.703 s**. Se verifican argumentos de
instalación con dos raíces, imports desde bootstrap y rechazo del hash incorrecto;
no se ejecuta el instalador real. Véase el procedimiento de bootstrap para su
preparación desde fuentes versionadas y los límites de esa evidencia.

## Integración pendiente

Las unidades actuales aún invocan los controladores instalados por su ruta
nominal. La [aprobación de generaciones](deployment-controller-approval.md)
selecciona un hash externo y prepara candidatos de unidades, con evidencia
persistente y reentrada exacta ante respuestas inciertas. Su aceptación local
no instala los candidatos. No debe sustituirse un hash discrepante por el que
se encuentre al arrancar.

También quedan pendientes la transición desde lectores antiguos, la exclusión
de entradas durante esa transición y la reapertura explícita tras verificarla.
Fijar un descriptor en procesos nuevos no modifica los imports de intérpretes
anteriores. La caché de bytecode de una instalación antigua se conserva para una
transición revisada: el publicador y el launcher la rechazan, no la eliminan ni
la adoptan como parte del inventario. `host.prepare` reescribe configuración y
unidades, por lo que no sustituye ese procedimiento acotado.
