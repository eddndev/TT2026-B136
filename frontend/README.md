# Frontend web (placeholder)

Este directorio contiene el frontend web del prototipo, construido con
[Astro](https://astro.build/) y la integración de
[Svelte](https://svelte.dev/). Por ahora es un andamiaje mínimo de forma
intencional: una sola página que renderiza un componente Svelte de ejemplo.

## Requisitos

- Node.js (versión LTS reciente) y npm.

## Instalación y uso

```bash
cd frontend
npm install      # instala las dependencias (astro, @astrojs/svelte, svelte)
npm run dev      # servidor de desarrollo con recarga en caliente
npm run build    # genera el sitio estático en frontend/dist
npm run preview  # sirve localmente el resultado de build
```

## Estructura

- `src/pages/index.astro` - página de inicio (placeholder).
- `src/components/Hello.svelte` - componente Svelte de ejemplo.
- `astro.config.mjs` - configuración de Astro con la integración de Svelte.
- `tsconfig.json` - extiende la configuración estricta base de Astro.

El frontend crecerá conforme avance el desarrollo del prototipo; por el
momento se mantiene pequeño a propósito.
