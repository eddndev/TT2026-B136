# Memoria de continuidad

Última actualización: 17 de agosto de 2026, zona `America/Mexico_City`.

Este archivo resume el estado de trabajo para retomar la sesión. Antes de
continuar, validar los datos volátiles con `git status`, `git diff` y una nueva
compilación. Las fuentes y los resultados automatizados siguen siendo la fuente
de verdad.

## Estado de Git

- Rama activa: `docs/avance-cripto-beamer`.
- El cierre anterior está versionado; el corte actual añade identidad
  multiusuario, PostgreSQL, Redis, sesiones revocables y RBAC al workflow
  documental local.
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

- `cargo test --workspace`: 395 pruebas descubiertas, 394 aprobadas y una
  ignorada.
- La prueba ignorada es el humo contra el sandbox real de Cincel.
- Cobertura total: 90.1 %.
- Cobertura por crate: `domain` 96.1 %, `application` 92.8 %,
  `infrastructure` 92.7 %, `bin` 80.7 % y `web` 73.2 %. El gate bloqueante del
  90 % se conserva sobre los tres crates con lógica criptográfica.
- `bash scripts/demo.sh` terminó correctamente.
- `bash scripts/api-demo.sh` terminó correctamente sin variables de Cincel y
  verificó el ZIP descargado con OpenSSL y `unzip`.
- La evidencia resumida está en `docs/verification-report.md`.

### Aplicación HTTP autenticada

- `despacho-cli serve` compone el workflow con `LocalOpensslTsa`, sin leer ni
  consultar Cincel.
- La API carga documentos, los persiste cifrados, los firma y sella, verifica
  los cuatro componentes, exporta el paquete de evidencia y comprueba la
  cadena de auditoría.
- El contrato vive en `docs/http-api.md`; las decisiones están en ADR-0011 y
  ADR-0012.
- PostgreSQL persiste usuarios, hashes Argon2id, roles, estado activo, secretos
  TOTP cifrados y códigos recovery hasheados. Redis conserva desafíos,
  sesiones opacas revocables, límites de login y reclamos TOTP con TTL.
- `X-Actor` fue retirado. El actor se deriva del bearer token y cada petición
  protegida recarga rol y estado desde PostgreSQL.
- Los roles Owner, Litigante, Paralegal y Cliente aplican una matriz
  conservadora. Cliente no accede a documentos hasta persistir pertenencia a
  casos.
- El repositorio documental continúa como un JSON cifrado por UUID; migrar
  documentos y auditoría a PostgreSQL sigue pendiente.

### Documentación LaTeX

Se actualizaron implementación, pruebas, conclusiones y anexos para documentar
la identidad multiusuario y la persistencia híbrida aplicada. `latex/main.pdf`
compila con 232 páginas; las 232 se renderizaron y las páginas nuevas de
implementación, pruebas, conclusiones y anexos se inspeccionaron ampliadas.

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
- Resultado: `presentacion/presentacion.pdf`, 14 diapositivas en formato 16:9.

Las diapositivas 10 a 12 explican:

1. La materialización de la dependencia inestable del PSC.
2. La mitigación mediante una TSA interna intercambiable.
3. El alcance técnico y el riesgo residual, sin atribuir equivalencia legal.

La imagen de la estructura del workspace ya fue ampliada y revisada. Las tres
diapositivas de riesgos y la nueva diapositiva de identidad también fueron
revisadas como imágenes renderizadas. Las 14 diapositivas se inspeccionaron y
el log final de Beamer no contiene advertencias `Overfull`, `Underfull` ni
`LaTeX Warning`.

## Verificación ejecutada al cierre

```bash
cd latex && make
cd ../presentacion && make
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --bin despacho-cli
cargo llvm-cov --workspace --json --summary-only \
  --output-path /tmp/tt2026-coverage.json
bash scripts/coverage-gate.sh /tmp/tt2026-coverage.json
bash scripts/demo.sh
bash scripts/api-demo.sh
git diff --check
git diff --exit-code -- frontend
```

La recalibración de Argon2id del 17 de agosto de 2026 sobre AMD Ryzen 7 7730U
fijó `m=262144,t=2,p=1`: cinco corridas promediaron 529.4 ms, dentro de la banda
de 500 a 1 000 ms. Todos los comandos anteriores terminaron correctamente. La
cobertura se midió con PostgreSQL y Redis locales activos para ejercitar los
adaptadores reales; los jobs `test` y `coverage` de CI levantan esos mismos
servicios. El binario release mide 7 886 552 bytes frente al límite de 25 MiB.
Cargo conserva un aviso de compatibilidad futura de
`redis 0.25.4`; esa versión está fijada para mantener Rust 1.78 y no produce
advertencias de Clippy ni fallos actuales.

## Pendientes reales

- Persistir expedientes, documentos y auditoría en PostgreSQL con transacciones
  apropiadas; hoy solo los usuarios son relacionales.
- Modelar pertenencia a casos y alcance por recurso antes de habilitar al rol
  Cliente.
- Añadir consultas, versionado documental más allá de la versión inicial y UI.
- Evaluar TLS interno y un cliente/pool asíncrono antes de despliegue público;
  el corte local aísla los clientes síncronos en el pool bloqueante de Tokio.
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
