# Informe de verificación local

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
  la pertenencia a casos, las consultas ampliadas y la UI permanecen pendientes.

La transcripción extensa de una corrida anterior se conserva en
[`docs/demo-transcript.md`](demo-transcript.md).
