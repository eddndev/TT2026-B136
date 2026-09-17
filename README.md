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

El PDF del reporte se genera a partir de las fuentes y se distribuye en
[Releases](https://github.com/eddndev/TT2026-B136/releases), fuera del historial
de archivos de las nuevas revisiones. El workflow
[Documents](.github/workflows/documents.yml) compila el reporte al publicar
una release, y adjunta el PDF, su suma SHA-256 y el commit de origen.
También comprueba la compilación en las PR que
cambian los documentos. Ver [la guía de distribución](latex/README.md).

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
cargo test --workspace                   # pruebas; servicios externos requieren variables
bash scripts/test-backends.sh            # suite con PostgreSQL y Redis desechables
cargo run --bin despacho-cli -- --help   # ayuda del binario
```

El guion de backends crea PostgreSQL con autenticación SCRAM y una contraseña
aleatoria para cada instancia desechable. Exporta las conexiones únicamente al
proceso de prueba y elimina sus datos al terminar; no modifica servidores existentes.

Antes de confirmar cambios de Rust, ejecutar además:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

### Catálogo de comandos de `despacho-cli`

La bandera global `--json` hace que cualquier comando emita su resultado
como un objeto JSON en lugar de texto legible.

| Comando | Descripción |
| --- | --- |
| `serve --signer-cert C --signer-key K [--bind IP:PUERTO] [--data-dir DIR]` | Inicia la API por expediente con PostgreSQL/Redis y TSA OpenSSL; `--data-dir` identifica el origen local cuyo corte se comprueba. |
| `database migrate --runtime-role ROL` | Aplica esquema y permisos con una conexión administrativa para un rol operativo existente. |
| `database import --data-dir DIR --mapping MAPA [--apply]` | Inspecciona documentos locales y el destino sin escribir; `--apply` importa con mapa explícito y reconciliación. |
| `crypto hash <archivo>` | Imprime el resumen SHA-256 del archivo. |
| `vault encrypt <archivo> --doc-id <uuid> [--version N]` | Cifra el archivo (AES-256-GCM con envoltura de llaves) y escribe `<archivo>.enc`. |
| `vault decrypt <paquete> --doc-id <uuid> [--version N] [--out RUTA]` | Descifra un paquete y rechaza cualquier alteración. |
| `vault rotate-kek --file <paquete>` | Reenvuelve la llave de datos bajo la llave nueva (`NEW_KEK_BASE64`). |
| `pki init-ca` | Crea la autoridad certificadora raíz interna (idempotente). |
| `pki issue --cn <nombre>` | Emite un certificado de entidad final y reporta su serie. |
| `pki revoke --serial <serie>` | Revoca un certificado emitido. |
| `pki gen-crl` | Regenera la lista de revocación (CRL). |
| `pki show <certificado>` | Muestra sujeto, emisor, serie y vigencia de un certificado. |
| `sign <archivo> --cert <cert> --key <llave>` | Firma el resumen del documento (RSA-3072, PKCS#1 v1.5) y escribe `<archivo>.sig`. |
| `timestamp <archivo> [--mock] [--out RUTA] [--pki-dir DIR]` | Obtiene un sello de tiempo RFC 3161; `--mock` usa la autoridad local. |
| `verify <archivo> --sig F --cert C --ca R [--tsr T] [--crl L] [--tsa-cert A] [--at INSTANTE]` | Reporte integral de cuatro componentes (integridad, firma, estado del certificado, sello); sale con código distinto de cero si el veredicto no es válido. |
| `package export <archivo> --sig F --tsr T --cert C --ca R --crl L [--tsa-chain CH] --out RUTA` | Exporta el paquete de evidencia (ZIP) con `INSTRUCCIONES.md` para verificar todo con `openssl`. |
| `auth calibrate` | Mide el costo del hash de contraseñas (Argon2id) en este equipo. |
| `auth hash-password` | Lee una contraseña por entrada estándar e imprime su hash PHC. |
| `auth verify-password --hash <phc>` | Verifica una contraseña contra un hash almacenado. |
| `auth totp enroll --user <correo> --secret-out RUTA` | Enrola un segundo factor TOTP e imprime la URI de aprovisionamiento y códigos de recuperación. |
| `auth totp verify --code <código> --secret-file RUTA` | Verifica un código TOTP de seis dígitos. |
| `audit append --action <acción> --resource <recurso>` | Agrega un evento a la bitácora de archivo del flujo offline; rechaza un origen ya migrado. |
| `audit verify-chain` | Verifica la cadena del archivo offline; nombra el índice roto si hay alteración. |
| `audit show` | Imprime la bitácora de archivo del flujo offline. |

### Variables de entorno

Se cargan del entorno o de un archivo `.env` local (ver
[`.env.example`](.env.example); el `.env` real nunca se versiona).

| Variable | Uso |
| --- | --- |
| `KEK_BASE64` | Llave de cifrado de llaves del `vault`: 32 bytes aleatorios en base64 (`openssl rand -base64 32`). |
| `NEW_KEK_BASE64` | Llave de reemplazo, leída solo por `vault rotate-kek`. |
| `CINCEL_BASE_URL` | URL base del proveedor remoto de sellos de tiempo. |
| `CINCEL_API_KEY` | Credencial del proveedor remoto (secreto). |
| `AUDIT_LOG_PATH` | Ruta de bitácora offline (por defecto `audit-log.jsonl`); la API usa la cadena PostgreSQL. |
| `PKI_CA_DIR` | Directorio de trabajo de la autoridad certificadora (por defecto `pki-ca` bajo el directorio actual). |
| `TSA_DIR` | Directorio de trabajo de la autoridad de sellado local (por defecto `pki-tsa` junto a `PKI_CA_DIR`). |
| `RUST_LOG` | Filtro de diagnóstico (`error`, `warn`, `info`, `debug`, `trace`); los diagnósticos van a `stderr`. |
| `DATABASE_URL` | PostgreSQL para usuarios, expedientes, documentos y auditoría: rol restringido en `serve`, administrativo en los comandos `database`. |
| `REDIS_URL` | Cadena de conexión a Redis para desafíos, sesiones revocables, límites y replay TOTP. |

### Demostración de extremo a extremo

```bash
bash scripts/demo.sh
```

Compila el binario y recorre el ciclo completo de evidencia en un
directorio temporal: autoridad interna, hash, cifrado con detección de
alteraciones, firma, sello de tiempo local, verificación integral
(positiva y negativa), exportación del paquete de evidencia verificado
solo con `openssl` y `unzip`, revocación, autenticación y bitácora.
Requiere `cargo`, `openssl`, `unzip` y utilerías estándar. Una
transcripción real recortada está en
[`docs/demo-transcript.md`](docs/demo-transcript.md).

La demostración de la aplicación HTTP se ejecuta por separado:

```bash
bash scripts/api-demo.sh
```

Este segundo guion levanta PostgreSQL, Redis y el servidor sobre puertos
efímeros; crea usuarios de los cuatro roles, completa TOTP, comprueba autorización,
logout y recuperación de un solo uso, carga un documento, verifica que no se
persista texto claro, lo firma y sella con la TSA local, y valida el ZIP con
OpenSSL. También crea expedientes, comprueba listados filtrados, asignaciones,
revocación con la misma sesión, aislamiento documental entre expedientes y
las restricciones documentales del Cliente. Ensaya además importación y
restauración con evidencia sellada en servicios desechables. Comprueba carga
clasificada atómica, filtros exactos, revisiones en conflicto, vaciado de valores
y conservación de historia, autores y ZIP tras restaurar.
La ampliación de calendarios pasó la campaña integrada de permisos globales,
revisiones, clasificación civil y restauración de diez respuestas exactas,
junto con las 21 de programación y resultados. No configura ni consulta Cincel.
El contrato completo está en
[`docs/http-api.md`](docs/http-api.md).

### Aplicación HTTP local

La API ofrece usuarios, expedientes, asignaciones, documentos cifrados y una
cadena de auditoría compartida en PostgreSQL. Login usa Argon2id con TOTP o
recuperación y sesiones opacas revocables en Redis. Las operaciones documentales
usan `/api/v1/cases/{case_id}/documents`: el actor procede del bearer vigente y
la confirmación revalida rol, pertenencia y asociación exacta. Owner accede a
todos los expedientes; Litigante y Paralegal requieren asignación, y Paralegal no
sella. Cliente puede consultar metadatos asignados, pero no documentos.

Las mutaciones PostgreSQL y sus eventos de auditoría comparten transacción.
La evidencia sellada es inmutable; verificación y exportación confirman acceso
y auditoría antes de devolver el resultado. Los desafíos y sesiones Redis se
compensan si falla la auditoría, sin una transacción distribuida ni garantía de
limpieza inmediata si también falla Redis. La decisión y los límites están en
[ADR-0016](docs/adr/0016-case-document-transactions.md).

Antes de iniciar `serve`, aplicar `database migrate --runtime-role` con una
conexión administrativa sobre una base UTF-8 y configurar `DATABASE_URL` con el rol operativo
restringido. El arranque no ejecuta DDL. También requiere `REDIS_URL`,
`KEK_BASE64`, CA, CRL, certificado y llave del firmante, y TSA local inicializada.
Conservar la KEK original al migrar: una llave nueva no descifra los datos
existentes. La [guía de base de datos](docs/database-operations.md) detalla
preparación, importación explícita, conservación del origen y restauración.
Los comandos criptográficos offline permanecen disponibles.

El [contrato HTTP](docs/http-api.md) describe rutas, límites y errores. El
[barrido del backend](docs/backend-review.md) conserva la revisión anterior;
ADR-0016 actualiza sus límites sobre asociación documental y transacciones.
Los [criterios de esta entrega](docs/next-goal.md) se contrastan con resultados
actuales antes de cerrar la integración. El listado, detalle, búsqueda e historial
documental se integran con la interfaz Qadra. Las versiones conservan identidad,
cifrado y evidencia histórica. Tipo, clasificación y etiquetas tienen un historial
auditado independiente y filtros por sus valores actuales. La gestión procesal avanza por módulos; el alcance y los pendientes están en
[el plan del producto](docs/product-completion.md).

El perfil penal tiene NUC, carpeta judicial, autoridades, delitos y datos
complementarios; conserva revisiones administrativas auditadas. El cierre del
expediente suspende mutaciones y mantiene consultas y evidencia autorizadas.
La etapa inicial de Investigación pertenece al alta penal completa; completar
un perfil anterior no inventa etapas. Los [contratos de administración](docs/case-administration-api.md)
distinguen estas operaciones de la API básica y las asignaciones de acceso.
La adopción y las transiciones ordinarias conservan actos declarados y soportes
exactos. La [programación de audiencias](docs/hearings-api.md) añade revisiones,
recibos de operación, cancelación e historia, con una agenda autorizada. Las
[sesiones y resultados declarados](docs/hearing-results-api.md) conservan
comparecencias, acuerdos, procedencia, ancla y continuidad exactas, rectificación
y retiro; sus pruebas locales y límites se registran por separado. No infieren
notificaciones, resoluciones ni vencimientos.

La [API de hechos declarados](docs/procedural-facts-api.md) conserva resoluciones
y prácticas de notificación como familias propias, con padre fijo, fuentes
históricas, tiempos de precisión explícita, corrección, retiro y recibos. El
backend y la API están verificados localmente, incluido el recorrido HTTP con
servicios reales y la restauración exacta. La [interfaz Qadra](web/README.md#resoluciones-y-notificaciones-declaradas)
ya permite capturar, consultar y conciliar estas declaraciones; su verificación
integral está aprobada localmente. No determina eficacia jurídica ni inicia automáticamente
plazos o recursos.

El [catálogo de calendarios jurisdiccionales](docs/judicial-calendars-api.md)
tiene backend, API y Qadra verificados localmente: Owner publica, reemplaza y retira;
Owner, Litigator y Paralegal consultan revisiones exactas sin asignación a
expedientes; Client queda denegado. Conserva ámbito, cobertura civil, reglas,
excepciones y referencias
públicas declaradas, con historia auditada. No descarga el contenido de las URL.
Las campañas de API/restauración, cobertura y navegador pasaron. La integración
remota y el manuscrito siguen pendientes. Cómputo de plazos, reevaluación, recursos
procesales y alertas siguen pendientes.

La [aritmetica de plazos](docs/deadline-arithmetic.md) calcula candidatas de dias,
meses civiles y horas con reglas matematicas explicitas, trazas y bloqueos por
datos insuficientes. La [investigacion de perfiles](docs/deadline-rule-research.md)
distingue esos calculos de la regla normativa aplicable. La activacion de plazos,
su persistencia, reevaluacion, corte horario y alertas siguen pendientes.
La [extracción temporal exacta](docs/deadline-triggers.md) ya vincula un campo o
una calificación declarada con su resolución, notificación o resultado de audiencia
y coordina ese tiempo con la aritmética. Conserva referencias, procedencia y
precisión; la integración con fuentes verificadas y plazos persistentes sigue pendiente.

### Frontend web

- [`web/`](web/) - interfaz funcional en Astro y Svelte: acceso con MFA,
  expedientes, consultas persistentes, carga, clasificación y sus revisiones,
  historial de versiones, sellado,
  verificación, evidencia,
  alta de usuarios y verificación de auditoría.
  El directorio de participantes por expediente conserva revisiones y autoría,
  con archivo/reactivación y permisos independientes de las cuentas de acceso.
  Ver [`web/README.md`](web/README.md) para ejecutar y probar la aplicación.
  Conserva la identidad visual Qadra. Integra administración penal, etapas,
  identidades y perfiles procesales, programación de audiencias y sesiones
  declaradas. Las resoluciones y notificaciones ya tienen formularios, selección
  de fuentes históricas y conciliación explícita; su verificación integral está
  aprobada localmente. Las campañas simuladas y reales se distinguen en el informe de
  verificación. El catálogo de calendarios incorpora publicación, reemplazo,
  retiro, historia y vistas mensual y por días; este avance no completa el
  cálculo de plazos ni las alertas.
  `bash scripts/web-demo.sh` prueba el navegador con servicios
  aislados reales; requiere las dependencias instaladas de `web/`.

- [`frontend/`](frontend/) - placeholder de la interfaz web, construido con
  Astro y Svelte. Ver [`frontend/README.md`](frontend/README.md) para
  instalación y uso (requiere Node.js y npm). Por ahora es intencionalmente
  mínimo.
