# Despliegue privado por versiones

[Deploy version](../.github/workflows/deploy.yml) se activa al publicar tags
`vMAJOR.MINOR.PATCH`, por ejemplo `v1.0.0`, `v1.1.0` y `v1.1.1`. No despliega
por pushes de rama ni por publicar una GitHub Release. El filtro inicial `v*`
es amplio; la validación exacta rechaza ceros iniciales, componentes incompletos,
prereleases y sufijos antes de ejecutar los demás jobs.

Se comparan el tag, el commit del evento y HEAD. Los tags anotados se resuelven
al commit. Se reutilizan CI y Web, incluyendo cobertura, PostgreSQL/Redis,
qpdf y navegador real. Si un check falla, no se despliega. Después se compilan
backend release y frontend con lockfiles, y se empaquetan como
`qadra-VERSION-COMMIT.tar.gz`, con SHA-256, inventario y huella de migraciones.
Antes de transferir se comprueba que el tag remoto no cambió ni desapareció.
Véase [ADR 0048](adr/0048-private-versioned-deployment.md).

## Instalación preparada

Ubuntu 24.04 x86_64, acceso administrativo de la aplicación con `ssh amorcito`:

| Elemento | Valor |
| --- | --- |
| Usuario y raíz | `hat`, `/home/hat/qadra` |
| Frontend nginx y proxy API | `127.0.0.1:18086` |
| API Rust | `127.0.0.1:18087` |
| PostgreSQL 16 propio | `127.0.0.1:15486` |
| Redis propio autenticado | `127.0.0.1:16386` |

Se reutilizan los ejecutables instalados, con bases, puertos y credenciales
independientes de otros proyectos. `qadra_runtime` es un rol PostgreSQL sin
privilegios administrativos; `qadra_admin` se usa en provisión y respaldos.
La aplicación recibe solo la conexión runtime. No se usan las contraseñas de
desarrollo de Compose. Estos servicios son independientes del runner
`vps2-tt2026-b136` y su servicio
`actions.runner.eddndev-TT2026-B136.vps2-tt2026-b136.service`.

`config/settings.json` contiene las contraseñas generadas y una KEK base64 de
32 bytes, con modo 0600 dentro de directorios 0700. El lanzador configura
`DATABASE_URL`, `REDIS_URL`, `KEK_BASE64`, `PKI_CA_DIR`, `TSA_DIR` y `TMPDIR`.
qpdf 12.4.1 y sus bibliotecas acompañantes están en el paquete verificado y
se seleccionan con `--qpdf-library`. Node y Cargo no son necesarios para ejecutar
la aplicación en el servidor. El correo opcional no se habilita por defecto.

La primera activación prepara migraciones, CA, certificado firmante, TSA, CRL y
confianza interna. No crea automáticamente un Owner de aplicación. Las siguientes
activaciones preservan claves/datos. La TSA local no es una constancia NOM-151
de un PSC autorizado.

## Secretos y variables de Actions

En Settings > Secrets and variables > Actions del repositorio:

| Tipo | Nombre | Contenido |
| --- | --- | --- |
| Secret | `QADRA_SSH_KEY` | Privada Ed25519 OpenSSH sin passphrase, conservando saltos de línea |
| Secret | `QADRA_KNOWN_HOSTS` | Línea completa de la llave Ed25519 verificada del servidor |
| Variable | `QADRA_HOST` | Host/IP alcanzable desde GitHub Actions |
| Variable | `QADRA_PORT` | Puerto SSH, actualmente `22` |
| Variable | `QADRA_USER` | Actualmente `hat` |
| Variable | `QADRA_ROOT` | Actualmente `/home/hat/qadra` |

La pública debe estar en `~/.ssh/authorized_keys` (0600; `.ssh` 0700), conservando
otras entradas. La pareja proporcionada coincide con la identidad de `amorcito`
y ya estaba autorizada; no se generó otra ni se duplicó la entrada. Los secretos
fueron configurados cifrados con la API de GitHub y no se guardan en Git.

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

Si faltan paquetes, un administrador instala en Ubuntu 24.04:

```bash
sudo apt-get update
sudo apt-get install -y postgresql-16 postgresql-client-16 redis-server nginx \
  openssl python3 ca-certificates
```

No detener ni reutilizar servicios globales de otros proyectos. Como usuario de
despliegue, desde un checkout revisado:

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
reiniciar. Los servicios tienen permisos privados y límites de recursos.

La raíz contiene `config/` (secretos/huella), `data/` (bases/CA/TSA/temporales),
`tools/` (controlador instalado), `incoming/`, `releases/`, `backups/`, `logs/`
y `run/`. `current` y `previous` apuntan a releases admitidas. Actualizar el
controlador o las unidades exige repetir la instalación desde una revisión
revisada, fuera de una activación. Los tags cambian el paquete de aplicación,
sin acceso al repositorio ni compilación en el servidor.

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
ssh -N -L 18086:127.0.0.1:18086 amorcito
```

Abrir `http://127.0.0.1:18086`. No se expone HTTP en interfaces externas y el
tramo remoto queda protegido por SSH. El bootstrap de Owner está bloqueado en
nginx: realizarlo localmente sobre la API siguiendo [identidad](http-api.md),
con email/contraseña elegidos por el operador. La respuesta MFA y los códigos
de recuperación deben conservarse como secretos.

```bash
ssh amorcito 'python3 /home/hat/qadra/tools/release.py --root /home/hat/qadra status'
ssh amorcito 'systemctl --user status qadra-api qadra-web qadra-postgres qadra-redis'
ssh amorcito 'journalctl --user -u qadra-api -n 100 --no-pager'
ssh amorcito 'tail -n 100 /home/hat/qadra/logs/nginx-error.log'
```

La comprobación consulta PostgreSQL y Redis; exige 401 JSON del endpoint sin
sesión, HTML del frontend, respuesta correcta del proxy y versión/commit exactos.
El arranque Rust conserva admisión qpdf y validación de catálogo/inventario.
Esto prueba disponibilidad, no todos los flujos de negocio. Web conserva sus
pruebas independientes contra servicios desechables.

## Recuperación y esquema

Una activación fallida restaura y comprueba la versión anterior, y devuelve fallo
a Actions. Si tampoco logra recuperarla, detiene la entrada web. La primera
instalación no tiene versión anterior. Para volver explícitamente:

```bash
ssh amorcito 'python3 /home/hat/qadra/tools/release.py --root /home/hat/qadra rollback'
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

Antes de activar, se crea un `pg_dump` y un archivo privado de configuración,
CA y TSA, con API detenida; `COMPLETE` identifica un respaldo terminado. Copiar
respaldos a almacenamiento externo protegido, comprobar restauración y definir
retención. No se borran automáticamente releases ni respaldos. Tras restaurar
SQL histórico, seguir la invalidación de sesiones/desafíos de la guía de base
de datos y conservar controles TOTP/límites. Rollback de aplicación no restaura SQL.

Los certificados y CRL requieren mantenimiento: los guiones generan certificados
de un año y CRL de siete días. Seguir [PKI](../pki/README.md) y la publicación de
confianza de [participantes](typed-participants-api.md), con revisión esperada
explícita, antes de expirar. Conservar evidencia histórica y reiniciar el servicio
para cargar el material operativo actualizado.

## Evidencia y límites

Consultar [el informe](verification-report.md). Servicios preparados y pruebas
focales no equivalen a una versión publicada: hasta el primer tag verificado
no hay frontend/API activos. No se omiten gates fallidos para probar transporte.
La primera ejecución real, el recorrido autenticado y la recuperación entre
dos versiones reales requieren evidencia propia.

Referencias: [Actions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax),
[respaldo PostgreSQL](https://www.postgresql.org/docs/16/backup-dump.html),
[proxy nginx](https://nginx.org/en/docs/http/ngx_http_proxy_module.html).
