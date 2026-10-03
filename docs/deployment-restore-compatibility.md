# Compatibilidad exacta para preparación offline de restore

`restore_admission.admit` comprueba una captura prospectiva completa contra un
descriptor administrativo ligado por un SHA-256 esperado externo. Compara el
release y los controladores instalados, además de hechos explícitos de las
herramientas y bases de destino. Es una comprobación sólo de lectura para
preparación offline; su resultado no permite restaurar ni abrir servicios.

## Entradas y procedencia

La función interna recibe:

```python
admit(root, backup, descriptor_path, *, expected_descriptor_sha256,
      controller_directory, observed)
```

`backup` debe ser un directorio privado administrado y completo con el
[manifiesto de captura](backup-capture-manifest.md). Los respaldos históricos de
cuatro archivos y las capturas que declaran no estar inicializadas se rechazan.

El descriptor contiene hechos que ese manifiesto parcial no captura: motores,
herramientas, nombres y atributos de roles, y el predecesor de auditoría esperado.
Su hash debe proceder de un recibo administrativo conservado por una vía
independiente. Un hash junto al descriptor no autentica ninguno de los dos.
No se puede reconstruir la procedencia perdida usando el estado actual del host.

`observed` contiene exactamente `postgres` y `redis`, con las mismas estructuras
que el descriptor. El llamador debe obtener esos hechos del destino y de los
binarios exactos que usará. Esta función valida su forma y exige igualdad; no
consulta servicios, ejecuta herramientas ni acredita la frescura o veracidad de
observaciones suministradas. El recolector operacional aún debe integrarse.

## Descriptor versión 1

El archivo es JSON UTF-8 regular privado `0600`, con un máximo de 64 KiB. Se
rechazan campos desconocidos o duplicados, números no finitos, booleanos en
posiciones de enteros y versiones de formato distintas. Sus campos son:

| Campo | Contenido |
| --- | --- |
| `format`, `version` | `qadra-restore-compatibility` y entero `1`. |
| `backup` | `id`, `captured_at` e inventario `files` de los cinco archivos completos, con tamaño y SHA-256. |
| `release` | `version`, `commit`, `schema` y hash `manifest_sha256` de `release.json`. |
| `controller` | Inventario exacto de módulos Python con tamaño y SHA-256. |
| `postgres` | Versiones, formato custom, identidad de base/esquema, locales, perfiles de roles y hashes de herramientas. |
| `redis` | Familia de motor, versiones, formato RDB, base lógica, modo AOF y hashes de herramientas. |
| `audit_predecessor` | `null` para cadena explícitamente vacía, o `sequence` y `head` originales. |

Los cinco archivos son `database.dump`, `redis.rdb`, `private-state.tar.gz`,
`COMPLETE` y `backup-manifest.json`. Se comprueban sus identidades completas;
el lector del manifiesto ya recalcula los tres hashes de datos. El UUID y la
fecha deben coincidir con esa misma captura. No se adopta un manifiesto nuevo
para completar retrospectivamente un recibo anterior.

La identidad declarada de origen debe coincidir con el release seleccionado,
el marcador de esquema y el directorio de controladores indicado. Además se
verifica todo el inventario del release instalado: nombres relativos seguros,
archivos regulares, ausencia de symlinks y entradas extra, SHA-256 y digest de
migraciones con la secuencia nombre/NUL/contenido usada al construir el paquete.
Los límites siguen siendo 512 MiB y 20.000 archivos, incluido `release.json`.
Los archivos vacíos siguen permitidos si su hash está declarado por el paquete.

## Hechos comparados

PostgreSQL exige `server_version_num` entre 100000 y 999999, versiones canónicas
major.minor de `pg_dump` y `pg_restore` con el mismo major del servidor, formato
`custom`, encoding `UTF8` y proveedor de locale `libc`. Base, esquema y roles son
identificadores ASCII de hasta 63 bytes. `collate` y `ctype` son textos ASCII
acotados de hasta 128 bytes. No se realiza conversión de esquema ni de locale.

Se registran exactamente los roles propietario y runtime, distintos entre sí.
Sus perfiles tienen booleanos explícitos `login`, `superuser`, `createdb`,
`createrole`, `inherit`, `replication`, `bypassrls`, un `connection_limit` entre
-1 y 1.000.000 y hasta 16 membresías únicas ordenadas en `member_of`. Runtime
debe conservar LOGIN, NOINHERIT, ausencia de privilegios elevados y de membresías.
Los atributos del propietario se comparan exactamente sin imponer un nuevo tipo
de rol administrativo. No se admiten contraseñas, hashes de contraseña o DSNs.

Redis distingue `redis` de `valkey`, conserva versiones canónicas major.minor.patch
del servidor y CLI, exige base lógica cero, formato RDB entre 1 y 9999 y un booleano
`appendonly`. Una versión numéricamente mayor de otra familia no es equivalencia.
Las estructuras completas deben ser iguales entre origen y observaciones.

Las herramientas se ligan por tamaño positivo de hasta 512 MiB y SHA-256:
servidor, `pg_dump` y `pg_restore` para PostgreSQL; servidor, `redis_cli` y
`redis_check_rdb` para Redis. El recolector debe calcular esos hashes sobre los
binarios que realmente usará; la función no recibe rutas de herramientas.

La cabecera de SQL debe comenzar con `PGDMP`; la de RDB debe tener la forma
`REDISdddd` y coincidir con el formato declarado. Estas lecturas acotadas no
validan el contenido completo del dump ni el checksum interno del RDB.

## Resultado y límites

El resultado tiene sólo `backup_id`, `descriptor_sha256` y `audit_predecessor`.
Una cadena no vacía usa secuencia entre cero y `2^63-1` y hash SHA-256 canónico.
La ausencia del campo no equivale a cadena vacía. No se consulta ni adopta la
cabeza actual del destino. Después de restaurar en un ámbito privado seguirá
siendo obligatorio verificar la cadena SQL completa contra ese predecesor antes
de la [invalidación administrativa](database-restore-commands.md).

La comprobación puede ejecutarse con el [bloqueo de restore](deployment-restore-fence.md)
presente; no lo interpreta como permiso, no lo crea y no lo elimina. Un bloqueo
CRL pendiente provoca rechazo porque el material de confianza puede estar en
transición. La función no llama a Runtime, provisionamiento, extractores,
activación de release, bases de datos ni control de servicios.

Faltan la comprobación del catálogo restaurado, validación completa del dump/RDB,
PKI y contadores frente a evidencia independiente, cierre de escritores y una
decisión explícita sobre la fuente Redis y la pérdida de controles recientes.
Igualdad de hechos no escoge esa política ni permite publicar autenticación.
El controlador posterior deberá conservar el lock y revalidar bajo su protocolo
de servicios detenidos; el propietario Unix aún puede modificar archivos.

Las [pruebas focales](../scripts/tests/test_deploy_restore_admission.py) reutilizan
la captura con dobles explícitos de herramientas y servicios. Los prefijos de
sus archivos de fixture no los convierten en respaldos nativos poblados. Esta
entrega no acredita un restore real ni una instalación en un host desplegado.
