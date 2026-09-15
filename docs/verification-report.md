# Informe de verificación local

La actualización académica posterior de estos resultados y la comprobación del
PDF se documentan en [la revisión del reporte](academic-report-verification.md).
Esa revisión documental no constituye una nueva ejecución de la suite Rust.

## Corte reproducido: consultas documentales e integración Qadra

- Fecha local: 14 de septiembre de 2026 (`America/Mexico_City`).
- Alcance: listado y detalle documental autorizados, búsqueda literal de nombre,
  filtro de sellado y conexión del sistema de diseño Qadra a expedientes reales.
- Decisión: [consultas de metadatos](adr/0018-authorized-document-queries.md).
  El [plan de cierre](product-completion.md) conserva las funciones pendientes.

### Backend y servicios reales

```bash
cargo fmt --all
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/test-backends.sh
bash scripts/api-demo.sh
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-document-queries-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-document-queries-coverage.json
```

La suite completa y su ejecución instrumentada aprobaron **539 pruebas**, sin
fallos y con una ignorada del proveedor externo. PostgreSQL y Redis fueron
reales y desechables, con bases separadas de identidad, expedientes y documentos.
Formato, compilación, Clippy, demostración HTTP y umbrales aprobaron.

Las veinte pruebas nuevas cubren validación del filtro, permisos de lectura,
autenticación, aislamiento, orden y paginación, comodines tratados literalmente,
metadatos sin decodificar el contenido cifrado ni la evidencia, cambios de rol,
inactividad, revocación y fallo de auditoría. Una prueba concurrente bloquea la
inserción de auditoría: listado y detalle no devuelven resultados antes del
commit, y retirar la asignación espera ese orden.

La demo HTTP reproduce consultas de los cuatro roles, dos expedientes,
revocación, sellado concurrente con un éxito y un conflicto, y migración y
restauración de **cuatro documentos con 51 eventos**. Compara evidencia ZIP y
verifica sus componentes con OpenSSL. El corte anterior de 46 eventos conserva
su fecha; los eventos nuevos proceden de las consultas añadidas al ensayo.

La primera ejecución de la suite detectó una carrera preexistente en el fixture
TCP del adaptador remoto simulado: una prueba liberaba un puerto antes de probar
su inaccesibilidad y otro servidor de prueba podía reutilizarlo. Se reprodujo
la interferencia y se reemplazó por un servidor que recibe la petición y cierra
sin respuesta, reteniendo el puerto. El adaptador no cambió. La suite del stub
aprobó después y veinte repeticiones acotadas también; esos conteos no se suman
a las 539 pruebas de la suite completa.

### Cobertura reproducida

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1045/1079 | 96.8 % |
| `application` | 2138/2277 | 93.9 % |
| `infrastructure` | 3602/3892 | 92.5 % |
| `web` | 637/737 | 86.4 % |
| `bin` | 834/1086 | 76.8 % |

Total: **8256/9071 líneas (91.0 %)**. Los tres crates sujetos al umbral del 90 %
aprueban. La cobertura corresponde al workspace Rust, no a los archivos Svelte.

### Interfaz y pruebas con HTTP simulado

La referencia Qadra se comprobó antes de adaptar sus flujos: 18 pruebas
unitarias y 14 de navegador, además de formato y compilación, aprobaron en
este entorno. Después de la integración aprobaron **21 pruebas unitarias y
23 de navegador con HTTP simulado**, junto con `npm run build` y
`npm run format:check` (Node.js 22.22.2). La automatización usa Node.js 24.

Las pruebas añaden expedientes persistentes, filtros y páginas solicitados al
servidor, permisos Client y descarte de resultados de una sesión, expediente,
búsqueda o detalle anteriores. Dos regresiones reproducidas antes de corregir
el código cubren una apertura que quedaba bloqueada al cambiar la búsqueda y
un detalle atrasado que reemplazaba la selección de una carga nueva.

La revisión visual conserva los originales de marca y las siete hojas de
estilo de Qadra. Las ampliaciones se concentran en `cases.css`. Se comprobaron
expedientes, lista y detalle en escritorio y móvil. Una prueba de geometría
verifica que buscador, botón y selector no se solapen a 390 píxeles; verificar
solo el ancho de la página no detectaba ese defecto de composición.

### Navegador con servicios reales

```bash
bash scripts/web-demo.sh
```

Un escenario Playwright aprobó con la API Rust, PostgreSQL y Redis aislados y
la TSA OpenSSL local. Desde la interfaz realizó login con recuperación MFA,
creación de expediente, carga de documento, detalle persistido, sellado,
verificación y descarga ZIP. El contenido descargado coincide byte por byte
con la muestra generada. Después de logout, recarga e inicio con otro código,
el expediente y el documento siguen disponibles desde consultas del servidor.
Se comprobó una vista de 390 píxeles sin desbordamiento horizontal ni errores
JavaScript. El script elimina servicios, claves y credenciales desechables.

La comprobación de navegador no intercepta HTTP. Las pruebas de UI con respuestas
simuladas se documentan por separado y no sustituyen este escenario real.
Usabilidad, carga de producción, versiones, clasificación, gestión procesal,
autenticación por certificado y firma por credencial individual siguen pendientes.
Los resultados de compilación y revisión académica están en
[la verificación del reporte](academic-report-verification.md).

## Corte reproducido: documentos por expediente y auditoría transaccional

- Fecha local: 12 de septiembre de 2026 (`America/Mexico_City`).
- Base: `f5d6716`; rama: `feat/document-case-authorization`.
- Alcance y decisiones: [criterios de entrega](next-goal.md),
  [ADR-0016](adr/0016-case-document-transactions.md) y
  [operación y restauración](database-operations.md).

### Verificación del estado final

```bash
cargo fmt --all
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/test-backends.sh
bash scripts/demo.sh
bash scripts/api-demo.sh
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-case-doc-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-case-doc-coverage.json
```

La suite completa final, ejecutada durante cobertura, aprobó **519 pruebas**,
sin fallos y con una ignorada del proveedor externo: 58 pruebas netas más que
el barrido anterior. PostgreSQL y Redis fueron reales y desechables, con bases
separadas de identidad, expedientes y documentos. Formato, build, Clippy,
demostraciones y umbrales terminaron con código cero. CI configura también
`DOCUMENT_TEST_DATABASE_URL`; no se cuentan retornos por variables ausentes
como ejercicio de esos adaptadores.

La demo HTTP reproduce cuatro roles, dos expedientes y revocación con la misma
sesión. Dos procesos del servidor compiten por sellar: uno obtiene `200`, otro
`409`, queda un evento de sellado y ambos entregan ZIP idénticos verificables
con OpenSSL. El ensayo posterior importa cuatro documentos (tres sellados),
conserva exactamente 46 entradas históricas, repite sin duplicación y restaura
un respaldo `pg_dump`/`pg_restore`; compara estado SQL y evidencia byte por byte
y verifica nuevamente firma, certificado/CRL y sello con OpenSSL.

Las regresiones incluyen rollback por fallo de inserción y commit, revocación
ordenada, escritores de auditoría concurrentes, barreras durables antes del
import, fallos de marcadores después del commit, restauraciones parciales,
recibos alterados y permisos PostgreSQL por propiedad, columna, esquema y roles
asumibles. Identidad retira desafíos/sesiones ante fallo de auditoría y conserva
revocaciones; no se afirma atomicidad distribuida con Redis. La validación de
cadena/CRL del firmante usa el instante del sello; TSA conserva la política
OpenSSL actual y puede rechazar una autoridad expirada hoy.

### Cobertura final

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1045/1079 | 96.8 % |
| `application` | 2073/2212 | 93.7 % |
| `infrastructure` | 3496/3785 | 92.4 % |
| `bin` | 834/1086 | 76.8 % |
| `web` | 578/683 | 84.6 % |

Total: **8026/8845 líneas (90.7 %)**. Pasan los tres umbrales obligatorios del
90 %. No se ensayó despliegue público ni carga de producción. Se conservaron
los 19 archivos locales protegidos del reporte, presentación y entregables;
no había un `runtime-data` local que migrar. Los ensayos usan sus propias fuentes.

Los cortes siguientes son históricos y conservan sus mediciones originales.

## Corte reproducido: barrido del backend

- Fecha local: 11 de septiembre de 2026 (`America/Mexico_City`).
- Base: `0cc6921`; rama de revisión: `feat/backend-hardening`.
- Alcance: defectos concurrentes, invariantes persistidos y claridad de las
  fronteras de identidad, almacenamiento y HTTP. Ver
  [el barrido](backend-review.md) y [ADR-0015](adr/0015-backend-concurrency-and-invariants.md).

### Verificaciones ejecutadas sobre el estado final

```bash
cargo fmt --all
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/test-backends.sh
bash scripts/demo.sh
bash scripts/api-demo.sh
```

La suite completa aprobó **461 pruebas**, sin fallos y con una ignorada del
proveedor de sellado externo. Son 35 pruebas netas adicionales frente al corte
de expedientes. Se usaron PostgreSQL y Redis desechables y bases separadas;
las pruebas de backends no se omitieron por ausencia de variables. Compilación,
formato, Clippy y ambas demostraciones terminaron con código cero.

Se reprodujeron fallos antes de corregir consumo concurrente de MFA, permisos
basados en identidades caducadas, controles en correo, deserialización inválida,
desbordamiento de versión, sobrescritura de evidencia, lectura parcial de la
bitácora y coordinación de migraciones. Las pruebas Redis cubren también un
contador heredado ya bloqueado sin TTL, ventanas que no se amplían y cuentas
sin contador que no crean claves al consultarse.

Las pruebas HTTP comprueban rechazo por saturación antes de leer el cuerpo,
permisos de trabajo retenidos después de cancelar la petición, recuperación de
capacidad tras fallo o cancelación y cabeceras/body de identidad estrictos.
La demostración integrada mantiene los cuatro roles, sesiones, expedientes,
revocación, evidencia documental y comprobación independiente con OpenSSL.

### Cobertura reproducida del barrido

```bash
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-hardening-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-hardening-coverage.json
```

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1039/1079 | 96.3 % |
| `application` | 1820/1950 | 93.3 % |
| `infrastructure` | 2467/2643 | 93.3 % |
| `bin` | 834/1045 | 79.8 % |
| `web` | 554/675 | 82.1 % |

Total: 6714/7392 líneas (90.8 %); los tres umbrales
obligatorios del 90 % aprobaron con PostgreSQL y Redis reales.

No se hizo una prueba de carga de producción ni se resolvieron TLS, pooling,
handshake Redis, transacción entre documento y bitácora, asociación documental
por expediente ni la amenaza de firma HTTP. Esos límites siguen explícitos en
[backend-review.md](backend-review.md) y [next-goal.md](next-goal.md).

Las fuentes y entregables locales del reporte y presentación se conservaron.
Los cortes que siguen son evidencia anterior y mantienen sus propios conteos.

## Corte reproducido: expedientes y asignaciones

- Fecha local: 11 de septiembre de 2026 (`America/Mexico_City`).
- Fecha UTC observada en las demostraciones: 12 de septiembre de 2026.
- Rama: `feat/case-membership`, basada en `cb1f79c` de `main`.
- Alcance: metadatos de expedientes, asignaciones y autorización por pertenencia.
  La decisión está en [ADR-0014](adr/0014-case-membership-and-isolation.md).

### Verificaciones ejecutadas

```bash
cargo fmt --all
cargo build --workspace
bash scripts/test-backends.sh
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/demo.sh
bash scripts/api-demo.sh
```

La suite completa ejecutada por `test-backends.sh` aprobó **426 pruebas**, sin
fallos y con una ignorada del proveedor externo de sellado. PostgreSQL y Redis
estuvieron disponibles: se ejecutaron las dos pruebas de identidad y las siete
de expedientes. Se añadieron siete pruebas de dominio, diez de casos de uso,
siete de persistencia y ocho HTTP, con fallo inicial antes de implementar.
Formato, compilación, Clippy y ambas demostraciones terminaron con código cero.
El workflow de CI se validó con `actionlint`; las nuevas pruebas usan una base
separada para no alterar la precondición del bootstrap de identidad.

Las pruebas reales de persistencia cubren reconexión, filtrado antes de paginar,
revocación, usuarios inexistentes o inactivos, asignaciones concurrentes sin
duplicados y rollback si falla la inserción de la membresía del creador. La
demostración HTTP cubre los cuatro roles y muestra que:

- Owner consulta todos los expedientes y administra las asignaciones.
- Litigante crea un expediente y obtiene automáticamente su asignación.
- Un UUID ajeno y uno inexistente tienen el mismo error `case_not_found`.
- Retirar la asignación elimina el acceso en la siguiente petición con la misma
  sesión, tanto de detalle como de listado.
- Cliente asignado consulta metadatos, pero carga, sellado, verificación y
  descarga documental responden `403`.
- Cambiar el rol o desactivar una cuenta afecta a su sesión ya emitida.
- El flujo documental mantiene cifrado, sello local y ZIP verificable con OpenSSL.

Los servicios temporales se detuvieron y sus datos se eliminaron al terminar.
El script de pruebas también permite reproducir la cobertura con servicios
reales mediante un comando Cargo como argumento.

### Cobertura reproducida

```bash
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-cases-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-cases-coverage.json
```

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1029/1069 | 96.3 % |
| `application` | 1815/1951 | 93.0 % |
| `infrastructure` | 2393/2573 | 93.0 % |
| `bin` | 834/1039 | 80.3 % |
| `web` | 470/579 | 81.2 % |

Total: 6541/7211 líneas (90.7 %). Los tres crates con
umbral obligatorio superaron el 90 %.

### Límites del avance

Los expedientes guardan título y referencia; las asignaciones controlan acceso
de usuarios. Participantes procesales, audiencias y plazos siguen pendientes.
Los documentos aún no pertenecen a expedientes y el personal conserva permisos
documentales globales; Cliente sigue denegado. Documentos y bitácora permanecen
en archivos separados, y las mutaciones de expedientes todavía no tienen
historial de auditoría. La UI continúa como placeholder.

Este avance no recompiló el reporte ni la presentación: sus fuentes y cambios
locales se conservaron. Las medidas documentales, de rendimiento y del binario
que siguen son históricas, no resultados nuevos de esta corrida.

## Evidencia histórica: 17 de agosto de 2026

Esta sección conserva el corte anterior para comparación. Sus conteos,
cobertura, tiempos, tamaños y pendientes describen aquella revisión.

### Corte reproducido

- Fecha local: 17 de agosto de 2026 (`America/Mexico_City`).
- Fecha observada en la salida UTC de la demostración: 17 de agosto de 2026.
- Revisión verificada: rebanada vertical HTTP local en la rama
  `docs/avance-cripto-beamer`.
- Rust: `rustc 1.94.0` y `cargo 1.94.0`.
- OpenSSL: `3.5.7` del 9 de junio de 2026.
- Cobertura: `cargo-llvm-cov 0.8.7`.

### Suite automatizada

Comando:

```bash
cargo test --workspace
```

Resultado:

```text
395 funciones de prueba descubiertas
394 aprobadas
0 fallidas
1 ignorada
```

La prueba ignorada es el humo contra el sandbox real del proveedor de
sellado. Se conserva para documentar el adaptador, pero no se ejecuta como
requisito de esta entrega: la API es inestable y el sandbox es de pago con
precios no transparentes. Los otros 13 casos del
adaptador remoto se ejecutaron contra el stub HTTP local y aprobaron,
incluidos token inmediato, procesamiento diferido, rechazo, errores HTTP y
redacción de la credencial.

`cargo fmt --all -- --check`, `cargo build --workspace` y
`cargo clippy --workspace --all-targets -- -D warnings` terminaron con código
cero. El binario release mide 7 886 552 bytes, por debajo del límite de 25 MiB.
El workflow de CI quedó configurado para levantar PostgreSQL y Redis tanto en
el job de pruebas como en el de cobertura, y su YAML se parseó localmente.

### Cobertura

Comandos:

```bash
cargo llvm-cov --workspace --json --summary-only \
  --output-path /tmp/tt2026-coverage.json
bash scripts/coverage-gate.sh /tmp/tt2026-coverage.json
```

Resultado:

```text
domain             973/ 1013 lines   96%  (gate: >=90%)
application       1749/ 1885 lines   92%  (gate: >=90%)
infrastructure    2257/ 2436 lines   92%  (gate: >=90%)
bin                834/ 1034 lines   80%  (gate: none)
web                319/  436 lines   73%  (gate: none)
```

Las proporciones sin truncar son 96.1 %, 92.8 %, 92.7 %, 80.7 % y 73.2 %,
respectivamente. El total del workspace es 6 132 de 6 804 líneas, 90.1 %.
La corrida de cobertura levantó PostgreSQL y Redis reales para no contabilizar
como cubiertos adaptadores que las pruebas omiten cuando esos servicios no
están disponibles.

### Demostración integral

Comando:

```bash
bash scripts/demo.sh
```

La ejecución terminó con código cero y reprodujo:

1. Creación de la CA interna y emisión del certificado del firmante.
2. Emisión de un certificado de TSA y de un token RFC 3161 local.
3. Hash SHA-256 del documento de muestra.
4. Cifrado AES-256-GCM, alteración de un byte y rechazo autenticado.
5. Rotación de la KEK sin volver a cifrar el documento.
6. Firma RSA-3072 y verificación integral de cuatro componentes.
7. Alteración del documento y rechazo con causas por componente.
8. Exportación del ZIP de evidencia y verificación con OpenSSL y `unzip`.
9. Revocación del certificado y detección mediante CRL.
10. Calibración Argon2id, TOTP y bitácora encadenada con detección de cambios.

La verificación independiente produjo `Verified OK`, `certificado.pem: OK` y
`Verification: OK`. Los archivos temporales y secretos de demostración fueron
eliminados automáticamente al terminar.

### Demostración de la aplicación HTTP

Comando:

```bash
bash scripts/api-demo.sh
```

La ejecución terminó con código cero y, sin variables de Cincel, levantó el
servidor en un puerto efímero; creó usuarios persistidos; completó TOTP;
comprobó `401`, RBAC, logout y recuperación de un solo uso; cargó y persistió
un documento cifrado; comprobó que el texto claro no aparece en el repositorio;
lo firmó y selló mediante la TSA local; verificó los cuatro componentes;
exportó el ZIP; y comprobó firma, certificado, CRL y sello con `openssl`. La
cadena de auditoría terminó válida con al menos diez eventos. El contrato y sus
límites se documentan en
[`docs/http-api.md`](http-api.md).

La suite incluye además una prueba que renombra deliberadamente un registro
JSON bajo el UUID de otro documento. El repositorio detecta que la identidad
interna no coincide con la ruta solicitada y rechaza el registro como
inconsistente.

### Entregables documentales

`make -C latex` generó `latex/main.pdf` con 232 páginas en tamaño carta y
`make -C presentacion` generó `presentacion/presentacion.pdf` con 14
diapositivas 16:9. Se renderizaron las 232 páginas de la tesis y las 14
diapositivas; se inspeccionaron ampliadas las páginas modificadas de
implementación, pruebas, conclusiones y anexos. El log final de Beamer no
contiene advertencias `Overfull`, `Underfull` ni `LaTeX Warning`.

### Decisión sobre el proveedor de sellado

El registro en el entorno de Cincel pudo completarse, pero eso no garantiza la
operación del servicio. Durante el consumo, la API no ofreció respuestas y
estabilidad suficientes para una campaña repetible; además, el sandbox
requiere pago y sus precios no son transparentes para presupuestar la muestra.
RTC-01 se registra como incidencia materializada; su probabilidad y VME se
conservan como línea base, no como medición de un costo ya incurrido.

Por decisión del proyecto, Cincel no es una dependencia técnica ni un criterio
de evidencia de la entrega actual. El adaptador y su stub se conservan para
demostrar la intercambiabilidad del puerto, pero la prueba contra el servicio
real permanece ignorada y no se agenda como campaña pendiente.

La decisión está registrada en
[`docs/adr/0009-local-timestamp-authority.md`](adr/0009-local-timestamp-authority.md).
La mitigación activa es `LocalOpensslTsa`, una TSA RFC 3161 cuyo certificado
RSA-3072 es emitido por la CA interna con `extendedKeyUsage = timeStamping`.
Opera detrás del mismo puerto que el adaptador remoto y sus tokens son
aceptados por `openssl ts -verify`. Esta vía demuestra continuidad técnica;
no aporta independencia de tercero ni sustituye una constancia NOM-151 emitida
por un PSC autorizado.

### Límites abiertos

- La evidencia externa de un PSC autorizado queda fuera del alcance de esta
  entrega. Una integración futura requerirá elegir un proveedor con contrato,
  disponibilidad y precios verificables; no se presenta la TSA local como
  sustituto jurídico de una constancia NOM-151.
- Argon2id quedó calibrado en el hardware de referencia (AMD Ryzen 7 7730U,
  16 hilos lógicos) con `m=262144,t=2,p=1`: cinco corridas promediaron 529.4 ms,
  dentro de la banda objetivo de 500 a 1 000 ms. Si el hardware de despliegue
  difiere, la medición debe repetirse.
- La API local ya entrega identidad multiusuario, sesiones revocables, RBAC,
  carga, persistencia cifrada en JSON, sellado, verificación, exportación y
  auditoría. La identidad procede del bearer token; `X-Actor` fue retirado.
- PostgreSQL persiste usuarios y Redis conserva el estado efímero de identidad.
  Documentos y auditoría siguen en archivos locales sin una transacción común;
  la pertenencia a casos y las consultas ampliadas permanecen pendientes.
  La interfaz documental de `web/` se verifica por separado a continuación.

La transcripción extensa de una corrida anterior se conserva en
[`docs/demo-transcript.md`](demo-transcript.md).

## Verificación de la interfaz web

Comprobaciones ejecutadas el 11 de septiembre de 2026 en Windows, con
Node.js 24.21.0 y npm 11.19.0. Estos resultados corresponden a `web/` y no
actualizan las mediciones históricas de Rust o criptografía anteriores.

- `npm test`: 18 pruebas aprobadas del cliente HTTP, errores, sesiones,
  descarga binaria, nombres compatibles con el ZIP, tamaño de documentos,
  UUID, permisos visibles, estados documentales, filtros y rutas por rol.
  Incluyen respuestas tardías que no deben afectar una sesión posterior.
- `npm run test:e2e -- --workers=1`: 14 pruebas aprobadas en Chromium con
  Playwright. Cubren
  alta inicial, MFA con TOTP o recuperación, carga, sellado, verificación,
  descarga, logout, roles, rechazo MFA, sesión vencida, alta de integrantes,
  auditoría, resultados obsoletos, recuperación de documentos pendientes de
  sello, resumen de sesión, filtros, vista de tarjetas, historial del navegador,
  visibilidad de contraseña y menú accesible en una pantalla de 390 px de ancho.
  Las regresiones cubren consultar otra vez el mismo documento, recibir una
  respuesta después de cerrar sesión y consumir las acciones de navegación.
  Los 12 flujos de `workflow.spec.mjs` no emitieron errores de JavaScript.
- `npm run build`: compilación estática completada con Astro 7 y Svelte 5.
- `npm run format:check`: sin diferencias de formato.
- `npm audit`: la comprobación anterior del mismo día reportó cero
  vulnerabilidades. Esta revisión de interfaz no modifica las dependencias.
- Revisión visual de capturas de inicio en escritorio y móvil, acceso móvil
  y detalle documental de escritorio, generadas por las pruebas de navegador.

Las pruebas interceptan las rutas HTTP con respuestas de prueba; no se
ejecutaron la API Rust, PostgreSQL, Redis ni la TSA en esta comprobación.
Tampoco se repitieron `scripts/api-demo.sh`, `scripts/demo.sh` ni las pruebas
de Cargo. No se modificó código Rust. La integración completa con servicios
reales requiere el entorno descrito en `docs/http-api.md`.

La nueva automatización `.github/workflows/web.yml` ejecuta formato,
pruebas, compilación y pruebas de navegador en Linux. Este informe no afirma
una corrida remota de ese workflow.

### Integración de marca Qadra

Comprobaciones ejecutadas el 12 de septiembre de 2026, después de incorporar
el nombre y los assets originales de Qadra en `web/`:

- `npm run format:check` y `npm run build`: completados correctamente.
- `npm run test:e2e -- --workers=1`: 14 pruebas aprobadas en Chromium con las
  mismas respuestas HTTP simuladas. No se repitieron las pruebas unitarias
  porque esta corrección no modifica la lógica del cliente.
- Los SHA-256 del logo SVG, favicon SVG, logo PNG y licencia coinciden con
  los archivos originales. Su procedencia se conserva en
  `web/public/brand/qadra/README.md`.
- Revisión del nombre, carga local de imágenes y marca en acceso de
  escritorio y móvil, y en la navegación del espacio documental.

Esta comprobación tampoco ejecuta los servicios reales del backend.

### Revisión de español de México

El 12 de septiembre de 2026 se revisaron los textos de acceso, navegación,
documentos, administración, ayuda y errores, así como las guías de `web/`.
Se corrigieron tildes, signos de apertura y concordancia; el documento HTML
declara `es-MX`. Las entidades HTML y los escapes Unicode permiten mostrar
los caracteres correctos y conservar los archivos de código en ASCII.

- `npm test`: 18 pruebas aprobadas con las expectativas de texto actualizadas.
- `npm run test:e2e -- --workers=1 --max-failures=2`: 14 pruebas aprobadas en
  la ejecución final. Una ejecución anterior agotó los 30 segundos de espera
  durante el acceso simulado; la repetición completa pasó con el mismo límite.
- `npm run format:check` y `npm run build`: completados correctamente.
- Revisión visual del acceso móvil, inicio móvil y detalle documental de
  escritorio: acentos legibles y sin desbordamiento por los textos corregidos.

Las comprobaciones de navegador mantienen la API simulada; no se probaron
los servicios reales del backend en esta revisión.
