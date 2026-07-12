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

## Desarrollo

### Workspace de Rust

El prototipo se desarrolla como un workspace de Cargo con arquitectura
hexagonal. Los crates viven bajo [`crates/`](crates/):

- [`crates/domain`](crates/domain/) - entidades y reglas de negocio; no
  depende de ningún otro crate del workspace.
- [`crates/application`](crates/application/) - casos de uso y puertos;
  depende de `domain`.
- [`crates/infrastructure`](crates/infrastructure/) - adaptadores concretos
  (persistencia, criptografía, servicios externos); depende de `domain` y
  `application`.
- [`crates/web`](crates/web/) - capa HTTP del backend; depende de
  `application` y `domain`.
- [`crates/bin`](crates/bin/) - el binario `despacho-cli`, punto de entrada
  que compone las capas anteriores; depende de todas.

La dirección de dependencias siempre apunta hacia el dominio: las capas
externas conocen a las internas, nunca al revés.

### Comandos comunes

```bash
cargo build --workspace                  # compila todos los crates
cargo test --workspace                   # ejecuta todas las pruebas
cargo run --bin despacho-cli -- --help   # ayuda del binario
```

### Frontend web

- [`frontend/`](frontend/) - placeholder de la interfaz web, construido con
  Astro y Svelte. Ver [`frontend/README.md`](frontend/README.md) para
  instalación y uso (requiere Node.js y npm). Por ahora es intencionalmente
  mínimo.
