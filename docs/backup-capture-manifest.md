# Manifiesto de captura de respaldos

Los respaldos prospectivos de `Runtime.backup` incluyen `backup-manifest.json`.
El archivo liga tamaños y SHA-256 de los datos capturados con una identidad local
declarada de aplicación y controlador. No admite una restauración operacional:
esa decisión necesita comprobaciones adicionales que se enumeran al final.

Los cuatro archivos anteriores conservan su significado:

| Archivo | Significado |
| --- | --- |
| `database.dump` | Captura PostgreSQL en formato custom. |
| `redis.rdb` | Instantánea obtenida del servicio Redis comprobado. |
| `private-state.tar.gz` | Configuración y material CA/TSA existente al capturar. |
| `COMPLETE` | Marcador final de captura y sincronización satisfactorias. |

El manifiesto es un quinto archivo de metadatos. No sustituye `COMPLETE` ni
convierte automáticamente un respaldo histórico en uno del formato nuevo.
La [guía de respaldos](deployment-backups.md) describe la captura de los datos.

## Formato 1

El objeto tiene exactamente estos campos:

| Campo | Contenido |
| --- | --- |
| `format` | Cadena fija `qadra-private-backup`. |
| `version` | Entero `1`. |
| `backup_id` | UUID nuevo, canónico, en minúsculas y distinto de cero. |
| `captured_at` | Instante UTC del registro de captura, con forma `YYYY-MM-DDTHH:MM:SSZ`. |
| `source` | Estado de inicialización e identidades declaradas de release y controlador. |
| `payloads` | Inventario exacto de los tres archivos de datos, sin `COMPLETE`. |

Cada entrada de `payloads` tiene exclusivamente `bytes` y `sha256`. Los tamaños
son enteros positivos; los digests contienen 64 caracteres hexadecimales en
minúsculas. El identificador es independiente del nombre del directorio: puede
moverse la captura antes de registrar su ubicación en un recibo externo.

`source` contiene cuatro campos:

- `initialized`: booleano que indica la presencia coherente de `config/schema`
  y del enlace `current` hacia un release administrado.
- `schema`: digest declarado por ese marcador, o `null` antes de inicializar.
- `release`: objeto con `version`, `commit`, `schema` y `manifest_sha256`; el
  último digest liga los bytes de `release.json`.
- `controller`: inventario de todos los archivos Python inmediatos del directorio
  del controlador que ejecuta la captura. Cada nombre tiene `bytes` y `sha256`.

Con inicialización declarada, el esquema del marcador debe coincidir con el
esquema del release. Sin `current` ni marcador se registra `initialized=false`,
con `schema` y `release` nulos. Un estado mezclado, un enlace fuera de `releases`
o un marcador inválido provoca rechazo.

Estas identidades son declaraciones locales. No prueban que el catálogo SQL ni
cada archivo instalado coincidan con el release. `initialized=true` tampoco
certifica la existencia o vigencia del material PKI. El hash del archivo privado
liga lo capturado, sin interpretar su contenido.

La identidad del controlador procede de sus archivos realmente leídos, incluido
`runtime.py` y los módulos del manifiesto. No se atribuye al controlador el commit
del paquete de aplicación. No se copian rutas absolutas, variables de entorno,
contraseñas, tokens, correos ni contenidos privados al documento.

## Publicación y lectura

La captura mantiene las comprobaciones existentes de API/web detenidas y de
identidad del proceso/directorio Redis, así como la validación de su RDB. Después
de capturar y sincronizar los tres archivos de datos:

1. Calcula sus hashes y la identidad declarada de origen.
2. Crea `.backup-manifest.tmp` de forma exclusiva, con modo `0600`.
3. Escribe y sincroniza ese archivo; lo renombra a `backup-manifest.json` y
   sincroniza el directorio exacto del respaldo.
4. Publica y sincroniza `COMPLETE` mediante el procedimiento existente.

Un fallo del manifiesto impide publicar `COMPLETE`. No se sobrescribe un
manifiesto previo ni se completa retrospectivamente un respaldo histórico.
Sólo se limpia el temporal propio; los datos capturados pueden permanecer como
una operación incompleta, sin autorización para usarlos como respaldo completo.

El lector exige `COMPLETE`, valida estructura y límites, y vuelve a calcular los
hashes de los tres datos. Rechaza campos desconocidos o duplicados, formas JSON
ambiguas, versiones desconocidas, fechas inexistentes y tamaños fuera de rango.
No escribe, extrae archivos, consulta bases ni inicia servicios.

El manifiesto y sus datos no se autentican entre sí frente a quien pueda cambiar
ambos. Su hash debe ligarse además a un recibo conservado por una vía confiable.
Las comprobaciones de archivos detectan cambios observados durante la lectura;
no impiden que el administrador del host altere el conjunto. El instante de
captura tampoco representa una transacción común entre SQL, Redis y archivos.

## Límites implementados

| Elemento | Límite |
| --- | --- |
| Manifiesto UTF-8 | 64 KiB; archivo regular privado `0600`. |
| Cada archivo de datos | De 1 byte a 8 GiB; regular privado `0600`. |
| Directorio del respaldo | Hijo directo administrado, modo `0700`, sin redirección por symlink. |
| Fuentes del controlador | De 2 a 64 nombres Python inmediatos; 256 KiB por archivo y 8 MiB en conjunto. |
| `release.json` | 8 MiB; archivo regular sin symlink. |
| Marcador de esquema | 65 bytes como máximo. |
| `COMPLETE` al leer | Archivo regular privado no vacío, de hasta 4096 bytes. |

Los hashes de datos se calculan en bloques acotados. Se comprueban tipo, tamaño
y permisos antes de leer, y dispositivo, inode, tamaño, modo y tiempos antes y
después para rechazar cambios detectados o sustituciones. Los límites son techos
de seguridad del formato; no constituyen mediciones de capacidad ni cambian los
timeouts existentes de captura.

## Compatibilidad de recibos CRL

`crl_backup` conserva el recibo `{"path":..., "files":{...}}`:

- Sin manifiesto, una captura histórica registra los cuatro hashes anteriores.
- Con manifiesto, primero lo valida y añade su hash como quinto archivo.
- La comprobación posterior exige igualdad con el inventario guardado. Rechaza
  cambios del manifiesto, su desaparición de un recibo de cinco archivos o su
  aparición junto a un recibo histórico de cuatro.
- Un manifiesto inválido o enlazado no permite volver al tratamiento histórico.

No se modifica el recibo anterior para adoptar archivos nuevos. Los mecanismos
externos que aún registran cuatro hashes no autentican los nuevos metadatos por
el hecho de transferirlos junto a los demás archivos.

## Instalación y evidencia acotada

`host.prepare` copia todos los módulos Python de `ops/deploy` a `tools`, incluidos
`backup_manifest.py`, `backup_manifest_files.py` y `backup_manifest_schema.py`.
Actualizar un paquete de aplicación no actualiza esos controladores. Su instalación
requiere un inventario y recibo revisados que incluyan toda la dependencia nueva;
no debe reutilizarse un instalador antiguo con una lista cerrada de menos archivos.

Doce pruebas focales aprobaron en 0.135 s, incluida
la copia e importación aislada desde `tools`. Las pruebas usan archivos, tar y
hashes reales, con systemd, PostgreSQL y Redis como dobles explícitos. No son una
prueba del restore desplegado ni un resultado de CI para cualquier revisión
posterior. Otras 22 pruebas existentes de respaldo y recuperación CRL aprobaron
por separado en 0.121 s. Sus fuentes son
[test_deploy_backup_manifest.py](../scripts/tests/test_deploy_backup_manifest.py)
y [su soporte compartido](../scripts/tests/deploy_backup_manifest_support.py).

Antes de admitir restore faltan compatibilidad de motores y herramientas,
propietarios y configuración de la base, cabeza auditada original verificada,
PKI y contadores sin regresión, validación del paquete, bloqueo durable de
servicios, política de origen Redis y espera por controles perdidos. Los
[comandos administrativos SQL](database-restore-commands.md) cubren otra parte
de esa frontera; este manifiesto parcial no reemplaza dichas comprobaciones.
