# Manual de usuario de Qadra

Documento LaTeX independiente con el manual de usuario de Qadra. No forma
parte del reporte técnico de `latex/` y se compila por separado.

## Estructura

| Ruta | Contenido |
|---|---|
| `main.tex` | Documento principal (clase `report`). |
| `preamble.tex` | Paquetes, estilo y macros propias del manual. |
| `portada.tex` | Portada. |
| `capitulos/` | Introducción, Parte A (despacho) y Parte B (cliente y verificación). |
| `figures/` | Logotipos institucionales. |
| `figures/capturas/` | Capturas de pantalla `A-NN-nombre.png` (despacho) y `B-NN-nombre.png` (cliente y verificación). |

## Compilar

Requiere TeX Live con LuaLaTeX y las fuentes Times New Roman y DejaVu Sans Mono.

```bash
cd manual-usuario
latexmk -lualatex -outdir=build main.tex
```

El PDF queda en `build/main.pdf`. La carpeta `build/` no se versiona.

## Capturas pendientes

Cada captura todavía no tomada aparece como un recuadro
`\capturapendiente{capturas/X-NN-nombre.png}{qué muestra · rol}`. Para
incorporarla:

1. Guarde la imagen en `figures/capturas/` con el nombre indicado.
2. Reemplace la macro por `\includegraphics[width=0.9\textwidth]{capturas/X-NN-nombre.png}`.

Las capturas no deben mostrar datos reales de clientes ni expedientes.
La lista completa, con el rol y lo que debe verse en cada una, está en
`capturas.md`.

## Fuentes de los textos

Los nombres de botones, campos y mensajes se toman del frontend en
`web/apps/qadra/src/`. Cuando un texto se arma en pantalla y no puede citarse
literal, se usa un marcador `[MENSAJE: ...]`, `[BOTÓN: ...]` o `[CAMPO: ...]`
para confirmarlo con la captura.
