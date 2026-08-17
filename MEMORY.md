# Memoria de continuidad

Última actualización: 17 de agosto de 2026, zona `America/Mexico_City`.

Este archivo resume el estado de trabajo para retomar la sesión. Antes de
continuar, validar los datos volátiles con `git status`, `git diff` y una nueva
compilación. Las fuentes y los resultados automatizados siguen siendo la fuente
de verdad.

## Estado de Git

- Rama activa: `docs/avance-cripto-beamer`.
- El cierre anterior está versionado en documentación/presentación, calibración
  de Argon2id y fundamento HTTP; el corte actual añade el workflow documental,
  persistencia local cifrada, API y demostración integral.
- El árbol de trabajo queda limpio al cerrar esta sesión.
- `frontend/` no tiene cambios y debe permanecer intacto.
- El antiguo sitio Astro de presentación fue eliminado por completo. La
  presentación vigente se genera exclusivamente con LaTeX Beamer.

Estado esperado de archivos:

```text
 clean
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

- `cargo test --workspace`: 383 pruebas descubiertas, 382 aprobadas y una
  ignorada.
- La prueba ignorada es el humo contra el sandbox real de Cincel.
- Cobertura total: 91.9 %.
- Cobertura por crate: `domain` 96.2 %, `application` 95.3 %,
  `infrastructure` 93.4 %, `bin` 82.6 % y `web` 75.3 %. El gate bloqueante del
  90 % se conserva sobre los tres crates con lógica criptográfica.
- `bash scripts/demo.sh` terminó correctamente.
- `bash scripts/api-demo.sh` terminó correctamente sin variables de Cincel y
  verificó el ZIP descargado con OpenSSL y `unzip`.
- La evidencia resumida está en `docs/verification-report.md`.

### Aplicación HTTP local

- `despacho-cli serve` compone el workflow con `LocalOpensslTsa`, sin leer ni
  consultar Cincel.
- La API carga documentos, los persiste cifrados, los firma y sella, verifica
  los cuatro componentes, exporta el paquete de evidencia y comprueba la
  cadena de auditoría.
- El contrato vive en `docs/http-api.md` y la decisión en
  `docs/adr/0011-local-document-workflow.md`.
- `X-Actor` es solo una etiqueta de auditoría, no autenticación ni
  autorización. El bind predeterminado es `127.0.0.1:3000`.
- El repositorio demostrativo escribe un JSON por UUID mediante reemplazo
  atómico y no persiste el texto claro; PostgreSQL y Redis siguen pendientes
  para producción.

### Documentación LaTeX

Se actualizaron implementación, pruebas, conclusiones y el anexo de pruebas
para documentar la rebanada HTTP, la persistencia local cifrada y los nuevos
resultados. `latex/main.pdf` contiene 229 páginas.

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
- Resultado: `presentacion/presentacion.pdf`, 13 diapositivas.

Las diapositivas 9 a 11 explican:

1. La materialización de la dependencia inestable del PSC.
2. La mitigación mediante una TSA interna intercambiable.
3. El alcance técnico y el riesgo residual, sin atribuir equivalencia legal.

La imagen de la estructura del workspace ya fue ampliada y revisada. Las tres
diapositivas de riesgos también fueron revisadas como imágenes renderizadas. El
log final de Beamer no contiene advertencias `Overfull`, `Underfull` ni
`LaTeX Warning`.

## Verificación ejecutada al cierre

```bash
cd latex && make
cd ../presentacion && make
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/demo.sh
bash scripts/api-demo.sh
git diff --check
git diff --exit-code -- frontend
```

Todos los comandos anteriores terminaron con código cero el 17 de agosto de
2026. La corrida de demostración midió Argon2id en 623.7 ms con los parámetros
calibrados, dentro de la banda de 500 a 1 000 ms. `git diff --check`, la regla
ASCII del código fuente y el límite de 400 líneas por archivo también pasaron;
el archivo de lógica más largo quedó en 379 líneas.

## Pendientes reales

- La API documental local está implementada; faltan autenticación de sesiones,
  JWT, RBAC, consultas adicionales y versionado documental más allá de la
  versión inicial.
- Sustituir el repositorio JSON local por PostgreSQL/Redis y construir la UI.
- La evidencia externa de un PSC autorizado queda fuera del alcance actual;
  solo se reabrirá si existe un proveedor con contrato, estabilidad y precios
  verificables.
- La aplicación puede continuar sin depender de Cincel porque su composición
  selecciona explícitamente la TSA local.

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
