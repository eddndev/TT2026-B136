# Despliegue privado por versiones

La [publicación completa de controladores](deployment-controller-publication.md)
tiene aceptación local de directorios; la coherencia de imports y su instalación
operativa se verifican por separado.

[Deploy version](../.github/workflows/deploy.yml) se activa al publicar tags
`vMAJOR.MINOR.PATCH`, por ejemplo `v1.0.0`, `v1.1.0` y `v1.1.1`. No despliega
por pushes de rama ni por publicar una GitHub Release. El filtro inicial `v*`
es amplio; la validación exacta rechaza ceros iniciales, componentes incompletos,
prereleases y sufijos antes de ejecutar los demás jobs.

Se comparan el tag, el commit del evento y HEAD. Los tags anotados se resuelven
al commit. Se reutilizan CI y Web, incluyendo cobertura, PostgreSQL/Redis,
qpdf, admisión multimedia y navegador real. Si un check falla, no se despliega.
Después se compilan backend release y frontend con lockfiles, y se empaquetan como
`qadra-VERSION-COMMIT.tar.gz`, con SHA-256, inventario y huella de migraciones.
Los jobs usan runners propios: validación y transferencia en VPS1, compilación
del paquete en VPS3 y los gates reutilizados conservan su reparto. Compilar en
VPS3 mantiene la compatibilidad con su ABI de Ubuntu 22.04; un binario compilado
en Ubuntu 24.04 puede exigir símbolos de GLIBC que ese servidor no ofrece.
Antes de transferir se comprueba que el tag remoto no cambió ni desapareció.
Véase [ADR 0048](adr/0048-private-versioned-deployment.md).

## Instalación objetivo

VPS3, Ubuntu 22.04 x86_64, con alias administrativo `ssh vps3` y puerto SSH
22022. La aplicación usa una cuenta independiente sin privilegios de root:

| Elemento | Valor |
| --- | --- |
| Usuario y raíz | `qadra`, `/home/qadra/qadra` |
| Frontend nginx y proxy API | `127.0.0.1:18086` |
| API Rust | `127.0.0.1:18087` |
| PostgreSQL 16 propio | `127.0.0.1:15486` |
| Redis propio autenticado | `127.0.0.1:16386` |

Se reutilizan los ejecutables instalados, con bases, puertos y credenciales
independientes de otros proyectos. `qadra_runtime` es un rol PostgreSQL sin
privilegios administrativos; `qadra_admin` se usa en provisión y respaldos.
La aplicación recibe solo la conexión runtime. No se usan las contraseñas de
desarrollo de Compose. Estos servicios son independientes de los runners de CI,
aunque comparten VPS3 y sus recursos. VPS1 y VPS2 conservan sus tareas de soporte; no se reutilizan
sus instancias de pruebas como bases persistentes de la aplicación.

`config/settings.json` contiene las contraseñas generadas y una KEK base64 de
32 bytes, con modo 0600 dentro de directorios 0700. El lanzador configura
`DATABASE_URL`, `REDIS_URL`, `KEK_BASE64`, `PKI_CA_DIR`, `TSA_DIR` y `TMPDIR`.
qpdf 12.4.1 y sus bibliotecas acompañantes están en el paquete verificado y
se seleccionan con `--qpdf-library`. Los ejecutables FFmpeg y ffprobe 9.0.2,
verificados desde la instalación fijada en `/opt/tt-media`, acompañan cada release
en `bin/`; `--ffmpeg-path` y `--ffprobe-path` seleccionan esas copias exactas.
No se sustituye esta dependencia por la versión multimedia de Ubuntu. Node y Cargo
no son necesarios para ejecutar la aplicación; sí para construirla en el runner.
El correo opcional no se habilita por defecto.

La primera activación prepara migraciones, CA, certificado firmante, TSA, CRL y
confianza interna. No crea automáticamente un Owner de aplicación. Las siguientes
activaciones preservan claves/datos. La TSA local no es una constancia NOM-151
de un PSC autorizado.

Si la publicación inicial de confianza quedó confirmada pero se interrumpió la
escritura de `config/schema`, la provisión consulta PostgreSQL antes de generar
material. Solo reanuda con revisión 1 vigente según el reloj de la base y CA/CRL
locales idénticas en DER a las publicadas; entonces no vuelve a publicar la
confianza. Una CA/CRL ausente o distinta, expiración o revisión posterior exige
mantenimiento explícito. No se regeneran CA ni CRL para eludir esa comprobación.


## Secretos y variables de Actions

En Settings > Secrets and variables > Actions del repositorio:

| Tipo | Nombre | Contenido |
| --- | --- | --- |
| Secret | `QADRA_SSH_KEY` | Privada Ed25519 OpenSSH sin passphrase, conservando saltos de línea |
| Secret | `QADRA_KNOWN_HOSTS` | Línea completa de la llave Ed25519 verificada del servidor |
| Variable | `QADRA_HOST` | `129.121.60.42`, alcanzable desde los runners propios |
| Variable | `QADRA_PORT` | Puerto SSH `22022` |
| Variable | `QADRA_USER` | `qadra` |
| Variable | `QADRA_ROOT` | `/home/qadra/qadra` |

La pública debe estar en `~/.ssh/authorized_keys` (0600; `.ssh` 0700), conservando
otras entradas. Utilizar una identidad Ed25519 exclusiva para la cuenta `qadra`
y el despliegue; no entregar la clave administrativa de root al workflow. Los
secretos se configuran cifrados en GitHub y no se guardan en Git. La configuración
del servidor original no acredita que estas variables ya apunten a VPS3.

Obtener la identidad del servidor por una conexión previamente verificada o
su consola, contrastando la huella. No crear confianza dentro del job mediante
`ssh-keyscan` sin verificar. La línea tiene esta forma:

```text
hostname ssh-ed25519 BASE64_PUBLIC_HOST_KEY
```

Para puertos distintos de 22, usar `[hostname]:puerto`. El job exige
`StrictHostKeyChecking=yes`, `IdentitiesOnly=yes`, `BatchMode=yes` y host Ed25519.
Una rotación de host exige verificar y actualizar `QADRA_KNOWN_HOSTS`. Para
rotar la identidad cliente, instalar la nueva pública antes de sustituir el
secreto y retirar después la anterior.

## Preparación y actualización administrativa

Verificar PostgreSQL 16, Redis 7.4.11, OpenSSL y Python antes de preparar VPS3.
El controlador de paquetes requiere Python 3.11 o posterior; no basta asumir la
versión de `/usr/bin/python3` en Ubuntu 22.04. Conservar las dependencias ya
instaladas: la provisión fija en las unidades el intérprete con el que se ejecuta
y la ruta del Redis seleccionado. En VPS3 se seleccionó Python 3.12.14. Verificar esas rutas antes de iniciar. Para
añadir nginx sin habilitar su servicio global público:

```bash
sudo systemctl mask nginx.service
sudo apt-get update
sudo apt-get install -y nginx
```

No detener ni reutilizar servicios globales de otros proyectos. Un administrador
crea la cuenta `qadra` y su raíz privada `/home/qadra/qadra`; la instalación
original se conserva. Como usuario de despliegue, desde un checkout revisado:

```bash
loginctl enable-linger "$(id -un)"
python3 -B ops/deploy/host.py --root "$HOME/qadra" --start-databases
systemd-analyze --user verify "$HOME/.config/systemd/user/"qadra-*.service
nginx -t -c "$HOME/qadra/config/nginx.conf"
```

Si la política del host deniega lingering, el administrador debe habilitarlo.
El script conserva `settings.json` al repetirse, registra servicios y habilita
las bases. Comprueba puertos libres al crear configuración. La API y nginx se
habilitan tras una activación saludable; nginx espera a la API también al
reiniciar. Los cinco directorios temporales nginx son privados bajo `run/`:
`client-body`, `proxy`, `fastcgi`, `uwsgi` y `scgi`; no dependen de rutas globales
en `/var/lib/nginx`. Los servicios tienen permisos privados y límites de recursos. Sus
límites de memoria son 2 GiB para API, 768 MiB para PostgreSQL, 256 MiB para Redis y 128 MiB
para nginx, con cuota de dos CPU por servicio. Son techos, no reservas; la
convivencia con CI requiere medir presión de memoria y CPU durante la aceptación.

La raíz contiene `config/` (secretos/huella), `data/` (bases/CA/TSA/temporales),
`tools/` (controlador instalado), `incoming/`, `releases/`, `backups/`, `logs/`
y `run/`. `current` y `previous` apuntan a releases admitidas. Actualizar el
controlador o las unidades exige repetir la instalación desde una revisión
revisada, fuera de una activación. Los tags cambian el paquete de aplicación;
la activación no accede al repositorio ni compila bajo la cuenta `qadra`.

## Crear y publicar un tag

Integrar primero el workflow y scripts en la revisión que se etiquetará.
Revisar todos los checks; el workflow los vuelve a ejecutar para ese tag.
No reutilizar ni mover una versión publicada.

```bash
git switch main
git pull --ff-only origin main
git tag -a v1.0.0 -m "release v1.0.0"
git push origin refs/tags/v1.0.0
```

Usar la versión elegida. La identidad de despliegue es tag/commit del manifiesto
y de `/version.json`, aunque la versión interna Cargo sea diferente. El servidor
rechaza tags retrasados de menor versión y una versión ligada a otro commit.
Solo una activación corre a la vez; GitHub puede sustituir una ejecución pendiente
por otra posterior, pero no cancela una activación en curso.

## Acceso, estado y logs

```bash
ssh -l qadra -N -L 18086:127.0.0.1:18086 vps3
```

Los ejemplos SSH requieren una identidad Ed25519 autorizada para `qadra`;
`-l qadra` selecciona esa cuenta y el alias `vps3` conserva host y puerto.
Abrir `http://127.0.0.1:18086`. No se expone HTTP en interfaces externas y el
tramo remoto queda protegido por SSH. El bootstrap de Owner está bloqueado en
nginx: realizarlo localmente sobre la API siguiendo [identidad](http-api.md),
con email/contraseña elegidos por el operador. La respuesta MFA y los códigos
de recuperación deben conservarse como secretos.

```bash
ssh -l qadra vps3 'python3 /home/qadra/qadra/tools/release.py --root /home/qadra/qadra status'
ssh -l qadra vps3 'systemctl --user status qadra-api qadra-web qadra-postgres qadra-redis'
ssh -l qadra vps3 'journalctl --user -u qadra-api -n 100 --no-pager'
ssh -l qadra vps3 'tail -n 100 /home/qadra/qadra/logs/nginx-error.log'
```

La comprobación consulta PostgreSQL y Redis; exige 401 JSON del endpoint sin
sesión, HTML del frontend, respuesta correcta del proxy y versión/commit exactos.
El arranque Rust conserva admisión qpdf y multimedia, y validación de
catálogo/inventario.
Esto prueba disponibilidad, no todos los flujos de negocio. Web conserva sus
pruebas independientes contra servicios desechables.

## Recuperación y esquema

Una activación fallida restaura y comprueba la versión anterior, y devuelve fallo
a Actions. Si tampoco logra recuperarla, detiene la entrada web. La primera
instalación no tiene versión anterior. Para volver explícitamente:

```bash
ssh -l qadra vps3 'python3 /home/qadra/qadra/tools/release.py --root /home/qadra/qadra rollback'
```

Se conservan las escrituras de la base actual; `current` y `previous` se
intercambian tras recuperar correctamente. Debe coincidir la huella de
migraciones. Un cambio de huella detiene el despliegue antes de apagar o migrar,
incluso si se considera aditivo. No editar esa huella para eludir el rechazo.

Los cambios de esquema requieren mantenimiento explícito: ensayar migración y
restauración en copia aislada, respaldar base/roles/material privado, detener
ingreso/API, migrar con el binario nuevo y conexión admin, validar inventario y
definir qué revisión admite esa base antes de registrar una nueva huella. No
forman parte del despliegue automático. Seguir [operación de base de datos](database-operations.md).

El controlador ampliado, integrado por PR51, captura PostgreSQL, Redis y la
configuración privada CA/TSA antes de activar, con API/web detenidas;
`COMPLETE` se publica al terminar correctamente. Los controladores de mantenimiento `2de3326` están instalados en VPS3 bajo
`deploy.lock`; la aplicación activa es `v0.1.1` sobre `e3aa87a`.
La [guía de respaldos](deployment-backups.md) concreta permisos, fallos,
copias antiguas sin Redis, transferencia fuera del servidor y restauración manual
con invalidación de sesiones/desafíos y conservación de controles TOTP/límites.
El respaldo inicial SQL/PKI tiene copia privada fuera de VPS3 y restauración
aislada comprobadas; no contiene RDB. La nueva captura
`20261002T105313Z-886940db` tiene SQL, RDB, PKI y `COMPLETE` validados, y su
copia externa de cuatro archivos coincide por hash y tamaño. Su restauración
conjunta con PostgreSQL 16.15 y Redis 7.4.11 aprobó, incluida persistencia AOF,
confianza y auditoría. Conservó cero usuarios y Redis vacío: no acredita un
despacho poblado. La prueba Redis de cinco claves sintéticas es evidencia separada. No se borran automáticamente releases
ni respaldos. Rollback de aplicación conserva SQL y Redis actuales.

Los certificados y CRL requieren mantenimiento: los guiones generan certificados
de un año y CRL de siete días. La [renovación manual recuperable](deployment-crl.md)
coordina revisión esperada, respaldo, contador, archivos, publicación auditada y
reinicio de la misma release, con un marcador persistente que bloquea ingreso
mientras haya incertidumbre. Preserva la CA y las revocaciones existentes.
Su instalación y aceptación en el servidor son independientes de su presencia
en el repositorio; no hay aún una programación automática de renovaciones.
La emisión de certificados sigue en [PKI](../pki/README.md) y la publicación de
confianza en [participantes](typed-participants-api.md).

## Evidencia y límites

Consultar [el informe](verification-report.md). Como evidencia histórica del
1 de octubre de 2026, se comprobaron 30 pruebas del controlador, incluidos los casos focales de
configuración y recuperación de confianza, y la sintaxis de los workflows.
Cinco comprobaciones adicionales de recuperación con PostgreSQL/OpenSSL reales
aprobaron contra el esquema de confianza exacto. Ese día quedaron preparados
PostgreSQL, Redis, las cuatro unidades de usuario y la configuración nginx;
API y frontend aún no tenían una release activa. Los ensayos del servidor
original conservan su evidencia histórica separada.

El 2 de octubre se activó `v0.1.0` sobre `a8dc3dd`. Deploy version aprobó en
17m28s, con CI/Web completos, identidad y hashes del paquete, salud y PKI
comprobados. PostgreSQL, Redis, API y frontend están activos. La restauración
inicial SQL/PKI conservó cero usuarios y un evento de auditoría, y el binario
aceptado arrancó y respondió 401. No acredita continuidad Redis ni datos poblados.
Owner todavía espera el correo administrativo; el recorrido autenticado
permanece pendiente. La recuperación entre releases se comprobó posteriormente
como se indica abajo.

PR51 se integró como `673b9ec` tras CI de 7m57s y Documents de 1m18s sobre
`177fe1f`. La confirmación natural de `673b9ece` aprobó CI `36996915839` en
6m52s y Documents `36996915909` en 7m55s, con 3340 pruebas Rust, dos ignoradas,
una nativa Redis y cobertura 97/95/93 %. Los controladores ya se actualizaron
en VPS3; esa instalación conservó la release activa y los cero usuarios.


El 2 de octubre se activó después `v0.1.1`, etiqueta exacta de `e3aa87a`.
Deploy version aprobó en 17m44s, con 3340 pruebas Rust, 419 controladas y 51
reales, conservando sus identidades respecto de main. El paquete de 19142052
bytes y 119 archivos coincidió con el manifiesto y el esquema; salud, versión,
CA, CRL y respaldo de cuatro piezas aprobaron. Incluye la duración observada de
informes y el aislamiento de respuestas tardías de acceso. No habilita idle.

Se ejecutó rollback real a `v0.1.0` y retorno a `v0.1.1`: 7.936 s y 8.020 s,
respectivamente, incluyendo respaldo, salud y comprobación de estado. SQL,
configuración y claves conservaron sus huellas; ambos respaldos publicaron
`COMPLETE`, y los RDB aprobaron su verificador. Quedó activa `v0.1.1`. La
instalación tenía cero usuarios: no es una prueba de recuperación de datos
poblados ni de un recorrido autenticado.

PR54 integró el mantenimiento de CRL como `2de3326`. Su cabeza aprobó CI en
6m45s y Documents en 7m50s; main confirmó 7m55s y 1m15s, con las mismas 3340
pruebas Rust y cobertura 97/95/93 %. Los nueve controladores de esa revisión se
instalaron en 5.754 s, conservando copias privadas, claves y estado; la aplicación
permaneció en `v0.1.1`. La renovación y su evidencia se registran por separado
[en el procedimiento](deployment-crl.md).

Referencias: [Actions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax),
[respaldo PostgreSQL](https://www.postgresql.org/docs/16/backup-dump.html),
[proxy nginx](https://nginx.org/en/docs/http/ngx_http_proxy_module.html).
