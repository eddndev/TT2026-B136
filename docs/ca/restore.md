# Respaldo y restauracion de la CA

Este procedimiento complementa [respaldos](../deployment-backups.md) y el
[manual CA](operations.md). Describe una restauracion administrativa explicita;
no existe un comando integral de restore CA. La adopcion del servicio en VPS3
siguen pendientes. El ensayo nativo local de dump/restore CA con identidad
poblada aprobo; la recuperacion integral del host requiere su propia aceptacion.
Los comandos siguientes son interfaces que deben concretarse con el inventario
aprobado del destino, desde el usuario de despliegue y su bus systemd.

## Conjunto que debe conservarse

Mantener intacta la captura original y su recibo externo de hashes. Verificar
`backup-manifest.json`, `COMPLETE`, los tres payloads y la release/controladores
contra los valores aprobados; calcular hashes nuevos no prueba su procedencia.
Ensayar primero sobre procesos, puertos y directorios aislados propios.

| Material | Ubicacion y comprobacion |
| --- | --- |
| Identidad y auditoria CA | `database.dump` de toda la base `qadra`, incluidas `qadra_ca.users` y `qadra_ca.audit_events`. |
| Configuracion y KEK CA | Archivo privado del servicio bajo `<root>/config`, incluido en `private-state.tar.gz`; misma KEK que protege el TOTP restaurado. |
| PKI y recibos CA | Todo `<root>/data/ca`, incluidos raiz, clave, indice, certificados, contadores y directorios `operations`, `revocations`, `publications`. |
| TSA existente | `<root>/data/tsa`, del mismo conjunto; no regenerarlo. |
| Estado Redis | `redis.rdb`, o controles actuales conservados si la instancia sigue integra. |
| Autoridad SQL | Inventario aprobado de roles, propietarios, membresias y permisos; `pg_dump` de una base no crea los roles del cluster. |
| Ejecucion | Release completa, inventario del controlador, unidades y configuracion del proxy aprobados. |

Guardar la configuracion CA como archivo regular `0600`, del usuario del
servicio, dentro del directorio privado `config` de la instalacion. La interfaz
`qadra-ca --config /RUTA/PRIVADA/ca-service.json` admite otras ubicaciones, pero
`Runtime.backup` solo archiva `config`, `data/ca` y `data/tsa`, mas la excepcion
CRL indicada abajo. Una configuracion externa exige una copia privada adicional
ligada al mismo recibo. No incluir secretos en Git, argumentos, logs o capturas.

Conservar UUID/correo del operador, KEK, credenciales SQL/Redis y huella de raiz
adoptada. La restauracion recupera la cuenta existente: no ejecutar
`enroll-operator`, no generar otro TOTP ni reutilizar una identidad del despacho.
Una KEK nueva no descifra la credencial restaurada. El archivo de entrega inicial
del autenticador es material privado separado, no configuracion del servicio.

### Publicacion CRL pendiente

Una captura generica rechaza operaciones CA pendientes. Terminar o reconciliar
la operacion original antes de solicitar otra captura; no borrar su journal
para eludir la guarda. La captura interna de su propia publicacion CRL conserva
el UUID original y agrega solamente:

```text
maintenance/crl/<UUID>/journal.json
maintenance/crl/<UUID>/original-crl.pem
maintenance/crl/<UUID>/candidate.pem
```

Los tres archivos se guardan con sus rutas exactas y modo `0600`, junto al
portal journal `data/ca/publications/<UUID>/journal.json`, aun sin recibo.
El journal de mantenimiento capturado esta en `prepared`, con ese mismo UUID
y `backup: null`; coincide con la confianza y los contadores previos del dump
y del archivo privado. No se archiva el staging de claves bajo ese mantenimiento.
Un archivo ausente, enlazado o un journal incompatible impide completar la copia.
La renovacion legacy sin journal de portal conserva el alcance previo del tar.

Al recuperar esa captura, restaurar los tres archivos antes de iniciar CA.
La reanudacion reutiliza el candidato guardado y puede tomar una nueva captura
antes de publicar; no necesita generar otra CRL. Un respaldo anterior que tenga
publicacion pendiente pero carezca de esos archivos no acredita recuperacion
automatica. Conservarlo e investigar; no fabricar un journal ni mezclar el
candidato, el indice o los contadores de otro punto de recuperacion.

## 1. Cerrar todos los escritores y el ingreso

Registrar primero las unidades, fragmentos, drop-ins, usuario, bus, cgroups,
PIDs y listeners observados. La unidad CA debe ser la realmente adoptada; no
suponer que la preparacion actual del host ya la crea. Cerrar tambien el proxy
CA o su unidad web propia si existe. Bloquear sus accesos externos durante todo
el procedimiento, incluido el ensayo de reapertura local.

Con `CA_UNIT` fijada al nombre comprobado, estas son las operaciones systemd:

```bash
systemctl --user stop "$CA_UNIT" qadra-api.service qadra-web.service
systemctl --user mask "$CA_UNIT" qadra-api.service qadra-web.service
systemctl --user show "$CA_UNIT" qadra-api.service qadra-web.service \
  --property=LoadState,ActiveState,SubState,MainPID,ControlGroup,FragmentPath
```

Exigir mascara efectiva y ausencia de procesos/listeners; `stop` solo no evita
otro arranque. Si `mask` rechaza un fragmento local existente, detenerse: guardar
sus bytes, metadatos y hash en el expediente privado, conservar el original
fuera del directorio de unidades mediante el procedimiento aprobado y publicar
la mascara. No usar `--force` para destruir el unico original. Registrar tambien
si la unidad estaba habilitada antes del cierre, para restaurar esa orientacion.

El cierre y gate existentes pinnean solo web, API, PostgreSQL y Redis. Sus
recibos no demuestran que CA esta detenida. Conservar evidencia separada de la
parada, mascara y ausencia de procesos/listeners CA. Detener tambien cualquier
CLI escritor; el `deploy.lock` no bloquea clientes externos que no lo respeten.

Usar la [coordinacion de cierre](../deployment-restore-quiesce.md) y su bloqueo
`deploy.lock` durante la intervencion. `restore_fence.enter(root, operation_id)`
es una primitiva interna, no una CLI; rechaza mantenimiento CRL y operaciones CA
pendientes. El modo ejecutable de `restore_fence.py` solo consulta la barrera.
No inventar opciones `enter`, `clear` o `force`, ni quitar journals para avanzar.
Conservar el UUID y la barrera durable durante la restauracion. Si el estado
actual impide admitirla, mantener cerrado el ingreso y reconciliar ese estado.

Solo despues de cerrar los escritores, detener los backends cuya persistencia
se reemplazara, siguiendo el cierre aprobado. No extraer archivos encima de
procesos activos ni intentar arrancar las unidades normales de base bajo la
barrera de restore: sus prearranques la rechazan. El ensayo y la recuperacion
privada usan procesos administrados aislados, no una relajacion de esa guarda.

## 2. Restaurar base, autoridad y archivos del mismo punto

En el destino aislado admitido, preparar los roles del cluster antes de cargar
el dump custom con `pg_restore`. Seguir [operaciones SQL](../database-operations.md)
y la [compatibilidad](../deployment-restore-compatibility.md) de herramientas,
base, propietario y release. No emplear `--no-owner` o `--no-acl` para ocultar
roles faltantes o convertir un error de permisos en una restauracion aceptada.

El esquema fijo `qadra_ca` y ambas tablas pertenecen al propietario de la base.
El rol de ejecucion CA es distinto del propietario y del runtime del despacho:

- `USAGE` en `qadra_ca`; `SELECT` en `users` y `audit_events`.
- `INSERT` en `audit_events`; `UPDATE` solamente de `recovery_codes`, `revision`
  y `updated_at` en `users`.
- Sin propiedad, DDL, permisos concedibles, autoridad privilegiada heredada,
  insercion de usuarios ni acceso a `public`.
- Conexion a la base y rol exactos, con `search_path=qadra_ca,pg_temp`.

Contrastar estos permisos con el
[validador CA](../../crates/infrastructure/src/ca_identity/permissions.rs).
El [esquema SQL](../../crates/infrastructure/src/ca_identity/schema.sql) sirve
para provision inicial; no volver a ejecutarlo sobre tablas restauradas.
Conservar acceso del administrador de respaldo a todo `qadra_ca`. El descriptor
de restore del despacho solo describe sus roles owner/runtime; no certifica
por si mismo esta autoridad CA adicional.

Recuperar configuracion, claves, certificados, indices, contadores, recibos y,
cuando correspondan, los tres archivos CRL anteriores. Mantener directorios
privados `0700`, archivos privados `0600` y propietarios previstos. Comprobar
huellas sin imprimir material privado. Preservar la barrera y las mascaras de
restauracion, que no deben desaparecer al recuperar `config`.

La release debe contener binario `qadra-ca`, worker y dependencias `ops/ca` y
`ops/deploy`, CLI, scripts PKI y web CA del inventario aprobado. Restaurar las
rutas configuradas hacia ese conjunto completo. La generacion pinneada del
controlador y su launcher tienen verificacion independiente; no sustituir el
hash aprobado por el hash del contenido que aparezca en disco.

## 3. Invalidar sesiones y persistir Redis

Con todos los escritores cerrados, conservar Redis actual si esta integro.
Si se perdio, recuperar el RDB verificado en un directorio privado nuevo con
version compatible, loopback y autenticacion; no mezclarlo con un AOF anterior.
Contrastar PID, directorio, puerto y base logica `0` antes de eliminar claves.

La interfaz existente es `restore_redis.invalidate_sessions(...)`, con
`redis_cli`, `host`, `port`, `password`, `expected_pid`, `expected_directory`,
`max_scan_calls`, `max_keys`, `batch_size` y `timeout` explicitos. Es una funcion
administrativa interna, no una CLI ni un comando `FLUSHDB`. El llamador mantiene
cerrados los escritores y persiste el resultado. Elimina solo claves completas
de 64 digitos hexadecimales bajo estos namespaces:

```text
identity:session
identity:challenge
identity:certificate-login
ca-identity:session
ca-identity:challenge
```

Conservar cuotas, marcas `totp-used`, sus valores y expiraciones absolutas de
ambas identidades. Comprobar recorrido final vacio. Persistir la invalidacion,
completar la conversion a AOF si se recupero desde RDB y verificar tras reinicio
que sesiones/desafios no reaparecen. Seguir el orden y los plazos de
[persistencia Redis](../deployment-backups.md); un recibo de borrado no demuestra
que el estado sobrevivira al reinicio.

Una captura antigua no recupera consumos de codigos de recuperacion, cambios de
credenciales o controles posteriores a ella. Contrastar ese estado con evidencia
posterior disponible; si falta, mantener autenticacion cerrada hasta resolver
el punto de recuperacion y las ventanas aplicables. No afirmar que un codigo
consumido despues del backup seguira consumido al restaurar ese backup.

## 4. Aceptar antes de reabrir

Con ingreso externo cerrado, verificar primero en el destino privado:

1. Apertura CA con el archivo restaurado: el comando real es
   `qadra-ca --config /RUTA/PRIVADA/ca-service.json`. La composicion valida esquema,
   permisos, cadena de auditoria y UUID/correo activo del operador. El proceso
   debe pertenecer al ensayo; `/healthz` por si solo no acredita los pasos siguientes.
2. Login nuevo de `eddndev@gmail.com` y MFA con la credencial recuperada. Rechazo
   de tokens/desafios anteriores, de la cuenta Owner del despacho y de permisos
   SQL del runtime CA sobre `public`. Conservar UUID e historial del operador.
3. Revision y CRL persistidas, huella de raiz, indice, contador, inventario,
   descarga por serie/huella y recibos del mismo punto de recuperacion.
4. Si existe publicacion preparada, comprobar sus tres archivos y el UUID antes
   de una reanudacion autorizada. Tras aceptar la restauracion y retirar su
   barrera mediante el procedimiento administrativo revisado, usar **Historial**
   y **Retomar esta publicacion**. La API existente recibe
   `{"action":"resume-crl","operation_id":"UUID_ORIGINAL"}` en
   `POST /api/v1/ca/operations`, con sesion CA y MFA nuevas. No usar la CLI legacy
   de resume para eludir el journal del portal ni solicitar otra renovacion.
5. Exigir recibo `published`, sucesora exacta y candidato original reutilizado.
   Un estado parcial o material discrepante deja cerrado el ingreso; no reparar
   OpenSSL, bajar contadores o regenerar la raiz para conseguir aceptacion.

No hay un comando publico que retire automaticamente la barrera de restore.
Su retiro requiere aceptar los recibos y la evidencia del procedimiento completo;
no eliminar `maintenance/restore/active.json` como solucion a un error de arranque.
Retirar solo las mascaras de esta intervencion, restaurar los fragmentos
aprobados y la orientacion previa de las unidades, y ejecutar `daemon-reload`
antes del arranque autorizado. Comprobar proceso, listener y release efectiva.
Abrir proxy/HTTPS al final. Con una publicacion pendiente, permitir acceso CA
controlado para retomar y mantener cerrado el trafico Qadra hasta su aceptacion.

## Evidencia nativa aun pendiente

Las pruebas locales de aislamiento SQL, login/MFA, worker/OpenSSL y protocolo
CRL no equivalen a este restore completo. Antes de declarar recuperacion CA en
VPS3 falta evidencia de:

- Stop/mask y reapertura con las unidades CA realmente adoptadas, controladores
  pinneados, ausencia de escritores y proxy cerrado.
- Restauracion aislada de una captura poblada: rol/permisos CA, misma KEK,
  operador/MFA, cadena de auditoria, recibos y rechazo de autoridad Office.
- Invalidacion de sesiones CA en Redis nativo restaurado, persistencia AOF y
  reinicio, conservando cuotas/TOTP vigentes y rechazando tokens previos.
- Restauracion de una publicacion `prepared` con los tres archivos y reanudacion
  nativa del mismo UUID/candidato, sin generar una segunda CRL.

Registrar herramientas, versiones, hashes, recibos y resultado de cada prueba
sin secretos. Mantener las copias fuente intactas; la adopcion publica y el
tiempo real de recuperacion requieren su propia aceptacion.
