# Bloqueo durable de admisión para restauración

El controlador incorpora un bloqueo persistente para impedir nuevas entradas
normales mientras una restauración está pendiente. Esta capacidad es parcial:
no detiene procesos, no restaura datos y no autoriza volver a abrir servicios.

## Registro e identidad

El registro vive en `maintenance/restore/active.json`, fuera de `config` y del
material incluido en el archivo privado de respaldo. La función interna
`restore_fence.enter(root, operation_id)` recibe un UUID explícito, canónico y
distinto de cero. No inventa otra identidad al reintentar.

El archivo contiene sólo `operation_id`, usa JSON ASCII, tiene modo `0600` y
está limitado a 4096 bytes. Los directorios de mantenimiento tienen modo `0700`.
La entrada toma el mismo `deploy.lock` que usa mantenimiento CRL. Publica con
los auxiliares privados existentes: temporal exclusivo, sincronización del
archivo, rename y sincronización del directorio contenedor.

Una reentrada con la misma identidad valida el registro y vuelve a sincronizar
su directorio, conservando bytes y fecha de modificación. Esa sincronización
resuelve el caso en que el rename anterior terminó y el fsync del directorio
falló. Otra identidad, contenido corrupto, campos duplicados o adicionales,
permisos incorrectos o rutas redirigidas provocan rechazo; no se reparan ni se
adoptan silenciosamente.

La presencia del registro basta para cerrar admisión, aunque esté mal formado,
sea ilegible, un symlink, un directorio o un FIFO. Las rutas de mantenimiento
inválidas también bloquean. La lectura acotada comprueba el tipo antes de leer
y no espera a que aparezca un escritor de FIFO. Un fallo no borra el registro.

## Entradas protegidas

| Entrada | Frontera |
| --- | --- |
| API `serve`, `Runtime.start` y espera de disponibilidad para web | Rechazo antes de ejecutar procesos o autorizar un arranque. |
| Construcción de `Runtime`, captura de respaldo e inicialización | Rechazo antes de cargar configuración o delegar efectos. |
| Admisión de paquete, activación y rollback de release | Rechazo antes de preparar contenido o cambiar enlaces. |
| Preparación del host y bases, incluida la inicialización con marcador previo | Rechazo antes de leer secretos o modificar estado. |
| Renovación CRL y reanudación, incluidas sus funciones preparatorias públicas | Rechazo antes de material, parada o escritura de su propio registro. |
| Unidades generadas PostgreSQL y Redis | `ExecStartPre` consulta el bloqueo de restore antes de iniciar listeners normales. |

El prearranque de las bases consulta sólo el bloqueo de restore. El bloqueo CRL
conserva su comportamiento: ese mantenimiento necesita las bases para publicar
y reanudar. A su vez, una operación CRL pendiente impide crear un nuevo registro
de restore. La parada explícita de servicios sigue disponible.

El único modo ejecutable del módulo es una consulta de admisión:

```text
<python> <root>/tools/restore_fence.py <root>
```

Devuelve cero sin salida cuando admite. Ante bloqueo devuelve uno con un
diagnóstico neutro, sin cargar configuración. No acepta acciones para crear,
limpiar, expirar o ignorar el registro. La creación sigue siendo una primitiva
interna destinada a la composición administrativa posterior.

`host.prepare` copia el módulo junto con los demás controladores y genera ambos
prearranques con el intérprete Python seleccionado. Una actualización del paquete
de aplicación por sí sola no instala esos archivos ni cambia unidades existentes.
Su instalación requiere revisar el inventario del controlador y las unidades;
ninguna prueba de fuentes acredita su presencia en un host desplegado.

## Límites y trabajo pendiente

Publicar el registro no termina operaciones ya admitidas ni drena conexiones.
Los puntos preparatorios consultan el bloqueo al entrar; no incorporan un nuevo
lock que abarque todos sus efectos. El administrador Unix puede eludir o cambiar
archivos. Por ello esta primitiva no prueba ausencia de escritores concurrentes
ni constituye una frontera frente al propietario del host.

Un controlador de restore posterior deberá conservar el lock, cerrar y verificar
servicios y listeners, admitir la compatibilidad del respaldo, restaurar en su
ámbito privado y validar el resultado antes de poder retirar el bloqueo. Esta
entrega no ofrece retirada pública, copia de datos, restauración SQL/Redis,
invalidación automática ni relajación del ingreso.

El [manifiesto de captura](backup-capture-manifest.md) y los
[comandos administrativos SQL](database-restore-commands.md) cubren partes
independientes. El registro de admisión no contiene identidad de respaldo ni
recibo de restauración completada. Las pruebas del comportamiento parcial están
en [test_deploy_restore_fence.py](../scripts/tests/test_deploy_restore_fence.py),
con [un focal de FIFO real](../scripts/tests/test_deploy_restore_fifo.py).
Usan archivos privados y dobles de efectos; no son un ensayo de restore real.

El coordinador ejecutó los siete focales satisfactoriamente en 0.319 s. Antes del
cambio del lector, el focal de FIFO real excedió su plazo de dos segundos; después
comprueba rechazo acotado y conservación del FIFO. Estos resultados corresponden
a la prueba local de fuentes, sin acreditar CI, instalación de unidades o
restauración en un host desplegado.

Por separado, el coordinador ejecutó 76 pruebas de compatibilidad de diez módulos
en 1.280 s, relativas a los controladores existentes. Ese grupo conserva la misma
limitación: no sustituye una instalación ni un ensayo de restauración poblado.
