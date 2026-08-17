# Memoria de continuidad

Última actualización: 17 de agosto de 2026, zona `America/Mexico_City`.

Este archivo resume el estado de trabajo para retomar la sesión. Antes de
continuar, validar los datos volátiles con `git status`, `git diff` y una nueva
compilación. Las fuentes y los resultados automatizados siguen siendo la fuente
de verdad.

## Estado de Git

- Rama activa: `docs/avance-cripto-beamer`.
- Commit base: `f80706b`.
- Los cambios de cierre se versionarán en commits separados para código,
  documentación y presentación, con staging explícito.
- `frontend/` no tiene cambios y debe permanecer intacto.
- El antiguo sitio Astro de presentación fue eliminado por completo. La
  presentación vigente se genera exclusivamente con LaTeX Beamer.

Estado esperado de archivos:

```text
 M latex/chapters/02-marco-teorico.tex
 M latex/chapters/03-analisis-diseno.tex
 M latex/chapters/04-implementacion.tex
 M latex/chapters/05-pruebas.tex
 M latex/chapters/06-conclusiones.tex
 M latex/chapters/anexo-e-matriz.tex
 M latex/main.pdf
?? AGENTS.md
?? MEMORY.md
?? docs/verification-report.md
?? presentacion/
```

## Convenciones y restricciones

- `CLAUDE.md` fue renombrado a `AGENTS.md`; leerlo antes de modificar archivos.
- No modificar, reescribir ni integrar cambios en `frontend/`.
- Usar `apply_patch` para cambios manuales.
- Mantener archivos con lógica por debajo de 400 líneas.
- La presentación debe seguir en Beamer 16:9, formal y profesional.
- No volver a crear una presentación web o un proyecto Astro salvo instrucción
  explícita del usuario.
- El usuario autorizó versionar el cierre; mantener staging explícito y no usar
  `git add .` ni `git add -A`.

## Trabajo completado

### Verificación del núcleo criptográfico

- `cargo test --workspace`: 367 pruebas descubiertas, 366 aprobadas y una
  ignorada.
- La prueba ignorada es el humo contra el sandbox real de Cincel.
- Cobertura total: 93.6 %.
- Cobertura por crate: `domain` 95.9 %, `application` 96.2 %, `infrastructure`
  93.5 % y `bin` 88.1 %; el nuevo crate `web` queda cubierto en su ruta de
  salud.
- `bash scripts/demo.sh` terminó correctamente.
- La evidencia resumida está en `docs/verification-report.md`.

### Documentación LaTeX

Se actualizaron el marco teórico, análisis y diseño, implementación, pruebas,
conclusiones y el anexo de pruebas. `latex/main.pdf` compila correctamente y
tiene 227 páginas.

La actualización de riesgos debe conservar esta formulación:

- El registro en Cincel sí se completó.
- El problema materializado aparece durante el consumo: la API no mostró
  estabilidad suficiente para una campaña automatizada y repetible.
- El sandbox requiere pago y su costo todavía no está incorporado por falta de
  una cotización verificable y de un tamaño definitivo de campaña.
- RTC-01 pasó de amenaza a incidencia materializada.
- La probabilidad de 70 % y el VME de $6,562.50 MXN se conservan como línea base,
  no como costo real incurrido.
- El VME total de línea base es $45,578.13 MXN; el presupuesto conocido sin el
  sandbox es $83,078.13 MXN.
- RLC-04 se considera parcialmente materializado como degradación del PSC, no
  como imposibilidad de registro.

La decisión vigente es usar `LocalOpensslTsa` como autoridad de demostración:
una TSA RFC 3161 local cuyo
certificado RSA-3072 con uso exclusivo `timeStamping` es emitido por la CA
interna. Opera detrás del mismo puerto `TimestampService` que el adaptador
remoto y produce tokens reales verificables con OpenSSL.

La integración con Cincel queda fuera del alcance de evidencia de la entrega
actual. El registro se pudo iniciar/completar, pero la API no ofrece garantía
operativa suficiente y el sandbox es de pago con precios no transparentes. El
adaptador se conserva como punto de sustitución futura, sin conmutación
silenciosa ni afirmación de equivalencia jurídica.

La documentación debe mantener una distinción explícita: la TSA interna aporta
continuidad y evidencia técnica, pero no independencia de tercero, fecha cierta
externa ni una constancia NOM-151 emitida por un PSC autorizado. No debe existir
conmutación silenciosa entre la TSA interna y un proveedor regulado.

### Presentación Beamer

La presentación se encuentra en `presentacion/`:

- Fuente principal: `presentacion/presentacion.tex`.
- Diapositivas de riesgo: `presentacion/risk-slides.tex`.
- Tema: `presentacion/theme.tex`.
- Resultado: `presentacion/presentacion.pdf` con 13 diapositivas.

Las diapositivas 9 a 11 explican:

1. La materialización de la dependencia inestable del PSC.
2. La mitigación mediante una TSA interna intercambiable.
3. El alcance técnico y el riesgo residual, sin atribuir equivalencia legal.

La imagen de la estructura del workspace ya fue ampliada y revisada. Las tres
diapositivas de riesgos también fueron revisadas como imágenes renderizadas. El
log final de Beamer no contiene advertencias `Overfull`, `Underfull` ni
`LaTeX Warning`.

## Verificación realizada al cierre

```bash
cd latex && make
cd ../presentacion && make
git diff --check
git diff --exit-code -- frontend
```

Resultados:

- `latex/main.pdf`: 227 páginas, compilación exitosa.
- `presentacion/presentacion.pdf`: 13 páginas, compilación exitosa.
- `git diff --check`: sin errores.
- `frontend/`: sin diferencias.
- La tabla extensa de comandos de la CLI se compactó para eliminar la
  advertencia `Float too large`.

## Pendientes reales

- El adaptador HTTP inicial ya está implementado en `crates/web` y cubierto por
  una prueba de contrato para `/healthz`.
- La evidencia externa de un PSC autorizado queda fuera del alcance actual;
  solo se reabrirá si existe un proveedor con contrato, estabilidad y precios
  verificables.
- El cierre requiere conservar los commits de código, documentación y
  presentación, además de confirmar que `frontend/` permanece intacto.

## Primeros pasos para retomar

```bash
git branch --show-current
git status --short --untracked-files=all
git diff --check
make -C latex
make -C presentacion
```

Después, abrir `latex/main.pdf` y `presentacion/presentacion.pdf` si se modifica
contenido visible. Para cambios sobre el riesgo del PSC, revisar primero
`latex/chapters/03-analisis-diseno.tex` y
`presentacion/risk-slides.tex`.
