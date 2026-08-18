# Presentación de avances

Presentación Beamer 16:9 del avance técnico del Trabajo Terminal TT2026-B136.
Resume las diez capacidades criptográficas, la arquitectura, las pruebas
reproducidas, el riesgo materializado del PSC y su mitigación con una TSA
interna en un PDF de dimensiones fijas.

## Compilación

Requiere LuaLaTeX, `latexmk`, Beamer y el tema Metropolis. Desde este
directorio:

```bash
make
```

El resultado se genera como `presentacion.pdf`. Para recompilar mientras se
edita:

```bash
make watch
```

Los datos del corte provienen de
[`docs/verification-report.md`](../docs/verification-report.md). Los logotipos
y el diagrama del workspace se reutilizan desde las figuras del reporte para
mantener consistencia visual con la documentación principal.
