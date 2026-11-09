# Manual técnico de Qadra

Documento LaTeX independiente con el manual técnico de Qadra: instalación,
configuración, despliegue, operación y mantenimiento del prototipo. Es hermano
de `manual-usuario/`, no forma parte del reporte técnico de `latex/` y se
compila por separado.

## Estructura

| Ruta | Contenido |
|---|---|
| `main.tex` | Documento principal (clase `report`). |
| `preamble.tex` | Paquetes, estilo y macros propias del manual (`\archivo`, `\nota`, `\importante`). |
| `portada.tex` | Portada. |
| `capitulos/NN-nombre.tex` | Capítulos 1 a 13: introducción, requisitos, arquitectura, instalación local, configuración, despliegue, operación, autoridad certificadora, línea de comandos, API HTTP, verificación de evidencia, mantenimiento y solución de problemas. |
| `capitulos/ap-a-mapa-documentacion.tex` | Apéndice A: mapa de la documentación del repositorio. |
| `figures/` | Logotipos institucionales. |

## Compilar

Requiere TeX Live con LuaLaTeX y las fuentes Times New Roman y DejaVu Sans Mono.

```bash
cd manual-tecnico
latexmk -lualatex -outdir=build main.tex
```

El PDF queda en `build/main.pdf`. La carpeta `build/` y los PDF generados no
se versionan (`.gitignore`). El flujo `Documents` de integración continua solo
compila el reporte; los manuales se compilan en local.

Después de compilar, revise en `build/main.log` que no haya referencias sin
resolver ni cajas desbordadas, y en el PDF la paginación de las tablas largas.

## Convenciones de redacción

- Cada capítulo empieza con `\chapter{...}\label{cap:<clave>}` y sus etiquetas
  usan un prefijo propio (`sec:inst-`, `tab:op-`, ...). Las referencias entre
  capítulos usan `\cref`; el reporte y el manual de usuario se citan por
  nombre, porque son documentos distintos.
- `\archivo{ruta}` es solo para rutas y nombres sin espacios: elimina los
  espacios de su argumento. Los comandos con argumentos van en `\texttt`, y
  las opciones largas en prosa, como `\mbox{\texttt{-{}-opcion}}`.
- Las tablas se componen en `\small` desde el preámbulo; los bloques de
  comandos usan `verbatim` con líneas de menos de 80 caracteres.
- Un tema se explica en un solo capítulo; los demás remiten a él.

## Fuentes de los datos

El manual condensa documentación existente y remite a ella por ruta; no la
sustituye. Todo comando, ruta, variable de entorno, puerto, versión y nombre
de binario se toma del repositorio vigente:

| Dato | Fuente |
|---|---|
| Versiones y requisitos | `Cargo.toml`, `compose.yaml`, `.github/workflows/`, `docs/deployment.md`, `docs/document-format-operations.md`, `docs/media-decoder-setup.md` |
| Variables y opciones de `serve` | `.env.example`, `crates/bin/src/serve_args.rs` |
| Subcomandos | `crates/bin/src/cli.rs`, `crates/bin/src/database_args.rs`, `crates/bin/src/ca/main.rs` |
| Despliegue y operación | `ops/deploy/`, `ops/ca/`, `scripts/`, `docs/deployment*.md`, `docs/database-operations.md` |
| CA y TSA | `pki/`, `docs/ca/` |
| API HTTP | `docs/http-api.md` y los contratos `docs/*-api.md` |
| Verificación de evidencia | `crates/evidence-verify/`, `latex/chapters/anexo-c-verificacion.tex` |
| Normas de contribución y CI | `CONTRIBUTING.md`, `AGENTS.md`, `docs/adr/` |

Al cambiar cualquiera de esas fuentes, actualice el capítulo que la cita. El
manual no registra despliegues concretos, versiones instaladas ni el estado de
una instalación, y no contiene secretos ni nombres de servidores: los ejemplos
usan valores genéricos. Tampoco afirma validez jurídica de la evidencia: la CA
y la TSA del prototipo son internas.
