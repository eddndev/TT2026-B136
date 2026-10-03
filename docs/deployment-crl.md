# Renovación recuperable de la CRL

## Alcance y estado

El controlador manual conserva la CA y las claves de la instalación privada.
Renueva su lista de revocación y publica una revisión de confianza mediante el
mismo ejecutable aceptado. No cambia certificados, permisos, esquema ni release.
La decisión y el límite entre archivos, SQL y arranque están en
[ADR 0063](adr/0063-recoverable-crl-maintenance.md).

Los nueve controladores aceptados de `2de3326` están instalados en VPS3. La
aceptación real del 2 de octubre renovó la revisión de confianza 1 a 2 y la CRL
4096 a 4097, conservando CA, claves y revocaciones. La aplicación `v0.1.1`
reinició con salud válida; se comprobó el journal aceptado, el respaldo exacto
de cuatro piezas y un único evento nuevo en una cadena de auditoría válida.
La verificación completa tardó 8.246 s en el servidor, 10.146 s con transporte,
y aprobó 172 comprobaciones. El export temporal de auditoría se eliminó.

Esta instalación todavía tiene cero usuarios: no acredita aceptación funcional
autenticada ni recuperación poblada. El procedimiento no programa renovaciones
automáticas.

## Preparación

Ejecutar como la cuenta `qadra`, con el intérprete usado por sus servicios.
El procedimiento siguiente requiere el [launcher estable y una generación
completa aprobada](deployment-controller-launcher.md), incluyendo todos los
controladores y sus dependencias. Sustituir `SHA256_APROBADO` por el SHA de su
inventario aprobado externamente y `/RUTA/PYTHON_APROBADO` por la ruta absoluta
del intérprete Python 3.11+ aceptado. No usar un hash calculado del destino para
eludir una discrepancia. Conservar las fuentes anteriores y sus permisos privados.
La instalación histórica descrita arriba precede a esta entrada; no acredita
su migración. Mientras falte la aceptación de esa instalación, estos comandos
no deben ejecutarse ni sustituirse por una invocación directa de `tools`.

Debe existir una release aceptada en `current`, el esquema inicializado, la CA
original, su índice, contador y CRL, y la misma autoridad y CRL en PostgreSQL.
La revisión esperada se obtiene de una consulta nueva; no se debe copiar de
un ejemplo antiguo. La CRL anterior puede haber expirado, pero el candidato y
la CA deben tener un intervalo válido al publicar. Una CA expirada o perdida
requiere otro procedimiento, no regeneración silenciosa.

Desde el host administrativo:

```bash
ssh -l qadra vps3 '/RUTA/PYTHON_APROBADO -I -B -S /home/qadra/qadra/controller_launcher.py --root /home/qadra/qadra --inventory-sha256 SHA256_APROBADO --entrypoint renew_crl.py -- --root /home/qadra/qadra status'
```

Después de revisar la identidad, reemplazar `REVISION` por el entero observado:

```bash
ssh -l qadra vps3 '/RUTA/PYTHON_APROBADO -I -B -S /home/qadra/qadra/controller_launcher.py --root /home/qadra/qadra --inventory-sha256 SHA256_APROBADO --entrypoint renew_crl.py -- --root /home/qadra/qadra renew --expected-revision REVISION'
```

## Qué ocurre

1. El bloqueo exclusivo impide otra activación, rollback o renovación simultánea.
   Se comparan los bytes DER y la revisión de confianza con los archivos actuales.
2. Una carpeta privada prepara el candidato con la clave existente y una copia
   del índice. No copia la clave privada, no cambia el índice vivo y conserva
   los números de serie previamente revocados.
3. Se detienen API y web y se confirma que SQL, CA, CRL y contador siguen en el
   estado original exacto. El respaldo completo captura SQL, Redis y PKI sin un
   marcador que dependa de un registro externo al archivo. Se conservan sus huellas.
4. Se publica y sincroniza el marcador de mantenimiento, se confirma otra vez la
   detención y se vuelve a comparar el estado. Entonces el contador avanza y la
   CRL se instala mediante reemplazos durables. El comando
   administrativo publica usando la revisión esperada; SQL conserva la historia
   y su evento de auditoría.
5. Se vuelve a leer SQL. Sólo la sucesora exacta, con la misma identidad y el
   candidato esperado, permite retirar el marcador, arrancar la misma release
   y comprobar salud. Las referencias `current` y `previous` no se mueven.

Durante mantenimiento no debe ejecutarse directamente `gen-crl.sh`, `revoke.sh`
ni emisión de certificados sobre esa CA. Esos comandos por sí solos no coordinan
la publicación SQL ni el marcador. Cualquier otra operación de PKI debe respetar
el mismo bloqueo exclusivo y conservar la evidencia histórica.

## Interrupción y reanudación

El registro privado queda en `maintenance/crl/UUID/journal.json`. El marcador
`config/crl-maintenance.json` identifica la operación pendiente. El arranque de
API, la preparación del proxy y activación/rollback rechazan el marcador incluso
después de reiniciar el host; no se retira manualmente para forzar el servicio.
No compartir el directorio privado ni el respaldo en issues o logs públicos.

Tras inspeccionar el estado, usar el UUID exacto de la operación:

```bash
ssh -l qadra vps3 '/RUTA/PYTHON_APROBADO -I -B -S /home/qadra/qadra/controller_launcher.py --root /home/qadra/qadra --inventory-sha256 SHA256_APROBADO --entrypoint renew_crl.py -- --root /home/qadra/qadra resume UUID'
```

Si la revisión original sigue vigente, se publica el mismo candidato. Si la
sucesora exacta ya existe, se confirma sin publicar otra revisión. Una respuesta
perdida no equivale a un rollback. Una cabeza distinta, material alterado, un
candidato expirado o SQL inaccesible mantiene detenido el ingreso y requiere
investigación. Nunca se rebaja el contador ni se restaura SQL para deshacer una
publicación confirmada.

La reanudación comprueba otra vez el inventario del respaldo contra sus
huellas SHA-256 registradas; un archivo ausente, alterado o sustituido por un
enlace impide reabrir el ingreso.
El recibo incluye los cuatro archivos históricos o esos cuatro y el nuevo
[manifiesto de captura](backup-capture-manifest.md), cuando está presente. Su
aparición, desaparición o cambio después de guardar el recibo provoca rechazo;
no se actualiza automáticamente el inventario para admitirlo.

Si la captura inicial falló y aún no existe un respaldo registrado, el controlador
puede intentarla otra vez únicamente con servicios detenidos y con la identidad,
revisión SQL, CA, CRL y contador originales exactos. Sólo en esa condición retira
temporalmente su propio marcador para capturar. Antes de cambiar confianza lo
reinstala y sincroniza; un fallo también lo reinstala y mantiene detenido el
ingreso. Si el contador, la CRL o SQL ya avanzaron, no crea un respaldo nuevo que
pretenda representar el estado original.

Una interrupción antes de instalar el marcador deja únicamente el material
original concordante. No existe en esa ventana una combinación parcial de CRL
y SQL. La copia resultante puede restaurarse sin un marcador cuyo registro
quedó fuera del archivo; siguen siendo obligatorias las validaciones completas
de restauración, vigencia y salud antes de abrir tráfico.

El respaldo conserva su propósito de recuperación completa, no de rollback
parcial de confianza. La política de copia externa y las restricciones del
estado histórico siguen en [respaldos](deployment-backups.md).
