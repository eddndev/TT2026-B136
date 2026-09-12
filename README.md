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
| `serve --signer-cert C --signer-key K [--bind IP:PUERTO] [--data-dir DIR]` | Inicia la API autenticada, conecta PostgreSQL/Redis, firma y sella con la TSA OpenSSL y persiste documentos cifrados. |
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
| `audit append --action <acción> --resource <recurso>` | Agrega un evento a la bitácora encadenada por hash. |
| `audit verify-chain` | Verifica la cadena completa; nombra el índice roto si hay alteración. |
| `audit show` | Imprime la bitácora completa. |

### Variables de entorno

Se cargan del entorno o de un archivo `.env` local (ver
[`.env.example`](.env.example); el `.env` real nunca se versiona).

| Variable | Uso |
| --- | --- |
| `KEK_BASE64` | Llave de cifrado de llaves del `vault`: 32 bytes aleatorios en base64 (`openssl rand -base64 32`). |
| `NEW_KEK_BASE64` | Llave de reemplazo, leída solo por `vault rotate-kek`. |
| `CINCEL_BASE_URL` | URL base del proveedor remoto de sellos de tiempo. |
| `CINCEL_API_KEY` | Credencial del proveedor remoto (secreto). |
| `AUDIT_LOG_PATH` | Ruta del archivo de bitácora (por defecto `audit-log.jsonl` en el directorio actual). |
| `PKI_CA_DIR` | Directorio de trabajo de la autoridad certificadora (por defecto `pki-ca` bajo el directorio actual). |
| `TSA_DIR` | Directorio de trabajo de la autoridad de sellado local (por defecto `pki-tsa` junto a `PKI_CA_DIR`). |
| `RUST_LOG` | Filtro de diagnóstico (`error`, `warn`, `info`, `debug`, `trace`); los diagnósticos van a `stderr`. |
| `DATABASE_URL` | Cadena de conexión a PostgreSQL para usuarios, expedientes, asignaciones y migraciones. |
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
revocación con la misma sesión y las restricciones documentales del Cliente.
No configura ni consulta Cincel. El contrato completo está en
[`docs/http-api.md`](docs/http-api.md).

### Aplicación HTTP local

La API entrega una rebanada multiusuario sobre `/api/v1`: usuarios en
PostgreSQL, login Argon2id con TOTP o recuperación, sesiones opacas revocables
en Redis, cuatro roles RBAC, expedientes con asignaciones y consultas filtradas,
carga y persistencia cifrada de documentos,
sellado local, verificación, exportación y auditoría. `X-Actor` fue retirado: el
actor y los permisos proceden de `Authorization: Bearer` y del usuario vigente.

El comando `serve` requiere `DATABASE_URL`, `REDIS_URL`, `KEK_BASE64`, una CA,
CRL, certificado y llave del firmante, y una TSA local inicializada. Para
desarrollo se incluyen `compose.yaml` y la guía completa de la API. La UI y el
vínculo entre documentos y expedientes permanecen pendientes. Cliente puede
consultar metadatos de expedientes asignados, pero sigue sin acceso documental.

La configuración de límites HTTP y las restricciones operativas están en
[el contrato](docs/http-api.md). El [barrido del backend](docs/backend-review.md)
distingue correcciones verificadas y trabajo pendiente antes de producción;
[el siguiente objetivo](docs/next-goal.md) define la asociación documental,
auditoría transaccional y migración recuperable.

### Frontend web

- [`frontend/`](frontend/) - placeholder de la interfaz web, construido con
  Astro y Svelte. Ver [`frontend/README.md`](frontend/README.md) para
  instalación y uso (requiere Node.js y npm). Por ahora es intencionalmente
  mínimo.
