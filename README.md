# TT2026-B136 — Trabajo Terminal

**«Prototipo de un sistema web para la gestión de expedientes digitales y
actividad procesal de un despacho legal, con servicios de integridad y no
repudio.»**

Escuela Superior de Cómputo (ESCOM) — Instituto Politécnico Nacional (IPN).

- **Autores:** Eduardo Alonso Sánchez · Hatziry Vitales Herrera
- **Directora:** Sandra Díaz Santiago
- **Periodo:** 2026-2

## Contenido

- [`latex/`](latex/) — fuente LaTeX del documento (compila con LuaLaTeX).
  Ver [`latex/README.md`](latex/README.md) para la guía de compilación y la
  estructura del proyecto.

## Compilar el documento

```bash
cd latex
make          # genera latex/main.pdf con LuaLaTeX
```

Requiere TeX Live (LuaLaTeX, biber, makeglossaries) y la fuente Times New Roman.
