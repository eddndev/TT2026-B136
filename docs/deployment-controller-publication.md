# Publicación completa de controladores privados

## Alcance y precondiciones

`ops/deploy/controller_publication.py` implementa una operación interna para
publicar un conjunto completo de fuentes Python. Conserva íntegro el directorio
anterior y permite reconciliar el mismo intento después de una respuesta incierta.
No opera sobre servicios, datos, claves, configuración ni enlaces de releases.
No equivale a una actualización operativa segura de procesos ya arrancados.

La función `publish_controllers(root, operation_id, staged_directory,
expected_manifest_sha256=..., expected_previous_sha256=...)` requiere rutas
absolutas canónicas, directorios propios 0700, archivos propios 0600 y un
`deploy.lock` existente. Rechaza enlaces y archivos con múltiples enlaces duros.
El UUID de operación debe ser canónico y no nulo. El publicador y el staging
permanecen fuera de ambos directorios que se intercambian.

El staging contiene exactamente `manifest.json` y `sources/`. El manifiesto
ASCII declara formato `qadra-controller-publication`, versión 1, revisión fuente
Git completa y mapa exacto de basenames Python a tamaño/SHA-256. La fuente se
selecciona y aprueba antes de llamar; el digest no autentica por sí solo su
procedencia. Se rechazan claves duplicadas, fuentes faltantes o sobrantes,
bytecode, subdirectorios, FIFO, enlaces y límites excedidos: 64 archivos,
256 KiB por archivo y 8 MiB total. El inventario anterior también debe ser
exclusivamente Python y coincidir con su digest explícito; un archivo ajeno
provoca rechazo, no borrado.

## Publicación y recuperación

Bajo el mismo lock no bloqueante del despliegue, la operación fija identidades
UID/dispositivo/inode, copia fuentes a un directorio privado nuevo y sincroniza
cada archivo y directorio. Registra `prepared` en
`maintenance/controllers/<operation_id>/journal.json` antes de modificar `tools`.

Un único `renameat2(RENAME_EXCHANGE)` de Linux intercambia los directorios completos,
con descriptores abiertos de sus padres. `tools` no desaparece y el directorio
anterior queda en `maintenance/controllers/<operation_id>/exchange`. Si el sistema
no ofrece esa operación, se rechaza sin sustituirla por dos renames o copias
sobre la instalación. Se sincronizan ambos padres y se comprueba otra vez cada
generación antes de escribir el recibo `published`.

Un fallo posterior al intercambio es un resultado incierto. No causa un segundo
intercambio automático ni un rollback. Reentrar con el mismo UUID, staging y
huellas vuelve a comprobar identidades, inventarios y orientación real:

| Directorios observados | Resultado permitido |
| --- | --- |
| Anterior en tools y candidato en exchange, estado prepared | Completar la misma publicación |
| Candidato en tools y anterior en exchange | Completar sincronización y devolver el recibo original |
| Cualquier otra combinación | Rechazar sin adopción ni limpieza |

Un error anterior al diario puede dejar un directorio propio incompleto para
inspección explícita; no se adopta ni se elimina recursivamente. El publicador
sólo descarta un temporal de diario si conserva el inode que él creó. No limpia
generaciones antiguas ni ofrece reversión genérica. Los archivos de configuración,
unidades, datos y PKI permanecen fuera de sus escrituras.

## Límite de imports y despliegue

El intercambio garantiza el conjunto visible por cada directorio, pero Python
puede cargar un módulo tardío mediante una ruta que ahora apunta a otra generación.
Un proceso que cargó A antes del intercambio puede importar B después. Tomar el
lock sólo en el publicador o censar procesos no fija los imports de sus lectores.

La caracterización nativa ejecuta intérpretes inocuos: la ruta nominal observa
A/B y un descriptor de directorio fijado observa A/A. Esto demuestra la frontera;
no implementa un launcher completo ni modifica las unidades instaladas. Antes
de instalar operativamente hace falta excluir lanzadores/lectores durante el
cambio o integrar y aceptar un mecanismo que fije la generación de cada proceso.
El procedimiento de `host.prepare` no sirve como sustituto: además de copiar
fuentes reescribe otras partes de la instalación.

## Verificación local del 3 de octubre de 2026

Seis pruebas reprodujeron primero la ausencia del módulo. La implementación
aprobó **6/6 en 0.450 s**, usando archivos, locks, fsync e intercambio Linux reales.
Cubren inventario completo, preservación externa, rechazos, lectores fijados,
fallos de durabilidad y reentrada, cambios de identidad y conservación de archivos
ajenos. Las fallas se inyectan en fronteras de archivos; no se operan servicios.

La caracterización independiente de imports aprobó **1/1 en 0.041 s**. No se
repitió después de implementar el publicador porque no importa ese módulo.
Estos resultados no acreditan instalación en VPS3 ni restauración de bases.

```bash
python3 -B -m unittest discover -s scripts/tests -p 'test_deploy_controller_publication*.py'
python3 -B scripts/tests/native_controller_import_characterization.py
```
