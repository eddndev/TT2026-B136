# Aprobar una generación de controladores

## Alcance

`ops/deploy/controller_approval.py` selecciona una generación ya publicada mediante
su evidencia exacta. `controller_unit_commands.py` genera candidatos de cuatro
unidades, sin instalarlos ni operar servicios. Esta selección es independiente
de publicar fuentes, instalar el lanzador y reabrir la aplicación.

El llamador aporta UUID de operación, hash esperado del diario publicado,
hash esperado del inventario instalado y hash de aprobación anterior o ausencia
explícita. No debe obtener el valor esperado de un archivo discrepante para
aceptarlo. El protocolo toma el mismo bloqueo no bloqueante de despliegue y
revalida identidad del directorio raíz, publicación, generaciones y permisos.

## Persistencia y reentrada

La operación conserva un registro privado `approval.json`, con identidad de raíz
y generación, revisión fuente, hashes de manifiesto/publicación/inventario y
predecesor explícito. Un archivo temporal sincronizado se publica exclusivamente
con Linux `renameat2(RENAME_NOREPLACE)`. No hay sustitución por hard links ni
sobrescritura ordinaria si esa operación no está disponible. Después se sincroniza
el directorio. El guard de archivos con múltiples enlaces se conserva.

La selección `approved.json` es otro archivo, con bytes propios e inode separado.
Su reemplazo es atómico, después de comparar nuevamente el predecesor capturado
y la evidencia inmutable de la operación. Se sincroniza el directorio padre antes
de devolver el recibo. Una respuesta perdida después de reemplazar no acredita
fracaso: la misma operación reobserva y sincroniza el resultado exacto sin
reemplazar su inode. Una aprobación posterior no autoriza volver silenciosamente
a otra generación.

Un proceso puede dejar un temporal privado sin referencia antes del rename. No
se adopta ni se borra por patrón; una nueva tentativa exclusiva no depende de
ese archivo. Las generaciones y archivos ajenos se conservan. Un registro con
un enlace externo inesperado sigue rechazándose y requiere inspección expresa.

## Candidatos de unidades

El renderer exige el registro completo, su hash esperado independiente y la
identidad esperada de raíz. Acepta exactamente API, web, PostgreSQL y Redis,
con sus comandos capturados. Sólo reemplaza la entrada Python de API, la espera
previa de web y la barrera previa de PostgreSQL/Redis. Inserta el lanzador externo
a `tools`, `-I -B -S` y el hash literal del inventario aprobado. Los demás bytes,
incluidos los límites de recursos, se conservan.

Rechaza directivas de ejecución desconocidas, duplicadas o vacías, continuaciones
ambiguas, inventarios incompletos y rutas con metacaracteres o especificadores
systemd. No consulta archivos actuales ni el manager y no deriva aprobación del
contenido que pretende ejecutar. El llamador aún debe validar fragmentos efectivos,
drop-ins, archivos e identidades justo antes de instalar los candidatos.

## Verificación y límites

Los ensayos locales usan generaciones y procesos Python inocuos. Las pruebas
cubren bloqueo, comparación del predecesor, pérdida de confirmación, rechazo de
identidades y hashes, conservación de bytes y selección A/A frente a fuentes
intercambiadas. La regresión de muerte del proceso exige reentrada exacta y
mantiene el rechazo de hard links externos. Los resultados ejecutados se registran
en [el informe de verificación](verification-report.md).

No se han instalado estas unidades ni la aprobación en VPS3. La transición desde
lectores antiguos, la caché de bytecode anterior, instalación del lanzador,
recarga del manager y reapertura necesitan su propio procedimiento y aceptación.
`host.prepare` reescribe configuración y no sustituye esa transición acotada.
Véanse [el publicador](deployment-controller-publication.md) y
[el lanzador](deployment-controller-launcher.md).
