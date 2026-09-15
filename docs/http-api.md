# API HTTP local autenticada

Contrato revisado el 2026-09-14. PostgreSQL conserva usuarios, expedientes,
asignaciones, documentos cifrados y una cadena de auditoría compartida. Redis
conserva desafíos, sesiones revocables, límites de intentos y reclamos TOTP. La
TSA OpenSSL local emite sellos RFC 3161; la ejecución no consulta Cincel.

La TSA local demuestra el comportamiento técnico, pero no aporta independencia
de un tercero, fecha cierta externa ni una constancia NOM-151 emitida por un PSC
autorizado.

## Inicio rápido verificable

La ruta más corta levanta PostgreSQL y Redis en puertos efímeros, crea una PKI y
una TSA temporales, arranca la API y recorre autenticación, RBAC, evidencia y
revocación:

```bash
bash scripts/api-demo.sh
```

El guion comprueba, entre otras condiciones, que:

- una petición sin sesión recibe `401`;
- el primer usuario es `owner` y enrola TOTP más ocho códigos de recuperación;
- un `paralegal` asignado puede cargar y verificar, pero no sellar;
- un documento ajeno al expediente responde `404`, incluso para Owner;
- las antiguas rutas globales de documentos responden `404`;
- los listados y detalles de expedientes excluyen los no asignados;
- retirar una asignación surte efecto usando la misma sesión;
- asignar un Cliente permite consultar metadatos, pero no documentos;
- logout invalida el token de inmediato;
- un código de recuperación funciona una sola vez;
- PostgreSQL no contiene la contraseña en claro;
- Redis no usa el bearer token en claro como clave;
- el documento persistido no contiene el texto claro;
- el ZIP exportado se verifica fuera de la aplicación con OpenSSL;
- una importación y restauración desechables conservan evidencia sellada.

## Inicio manual

Puede iniciar los servicios de desarrollo con Podman Compose:

```bash
podman-compose up -d postgres redis
export REDIS_URL='redis://127.0.0.1:6379/'
```

Prepare un rol PostgreSQL operativo sin privilegios administrativos siguiendo
[la guía de base de datos](database-operations.md). Con una conexión
administrativa, aplique el esquema y otorgue permisos al rol existente:

```bash
export DATABASE_URL='postgresql://administrador@localhost/despacho'
cargo run --bin despacho-cli -- database migrate --runtime-role tt_runtime
```

Antes de arrancar, cambie `DATABASE_URL` por la conexión del rol operativo y
configure su autenticación. Para una instalación nueva genere `KEK_BASE64` con
`openssl rand -base64 32` y consérvela fuera del repositorio. Una instalación
existente necesita su KEK original y debe completar la importación descrita en
la guía. Prepare CA, certificado firmante, CRL y TSA con los scripts de `pki/`:

```bash
cargo run --bin despacho-cli -- serve \
  --bind 127.0.0.1:3000 \
  --data-dir runtime-data \
  --signer-cert ruta/firmante.crt.pem \
  --signer-key ruta/firmante.key.pem \
  --ca-cert ruta/ca.crt.pem \
  --crl ruta/crl.pem \
  --tsa-config pki/tsa.cnf \
  --tsa-dir ruta/pki-tsa
```

El servidor no ejecuta DDL y rechaza roles que puedan administrar o reescribir
la auditoría. `database migrate` aplica las migraciones de identidad,
expedientes y documentos/auditoría. `--data-dir` señala el origen local
preservado: si contiene datos, el arranque exige un corte completado y
reconciliado con la base. Los documentos nuevos se guardan en PostgreSQL. El
servidor escucha solamente en `127.0.0.1:3000` por defecto.

## Autenticación

Todas las rutas usan `/api/v1`, salvo `/healthz`. El alta inicial se permite
solo mientras PostgreSQL no contiene usuarios; la inserción del primer owner se
protege con un bloqueo de tabla para conservar esa condición ante concurrencia.
El alta de otros usuarios verifica sesión, estado y rol dentro del caso de uso
y revalida al actor al confirmar en PostgreSQL. Bootstrap, creación y consumo
de un código de recuperación guardan su evento con la mutación en una sola
transacción.

| Método y ruta | Acceso | Resultado |
| --- | --- | --- |
| `GET /healthz` | Público | Estado del proceso. |
| `POST /api/v1/auth/bootstrap` | Público hasta el primer usuario | Crea el owner y devuelve material TOTP y recuperación una sola vez; `201`. |
| `POST /api/v1/auth/login` | Público | Verifica contraseña y devuelve un desafío MFA de 5 minutos. |
| `POST /api/v1/auth/mfa/totp` | Desafío | Consume un intento MFA y devuelve una sesión de 24 horas. |
| `POST /api/v1/auth/mfa/recovery` | Desafío | Consume un código de recuperación y devuelve una sesión. |
| `GET /api/v1/auth/me` | Bearer | Devuelve la identidad vigente. |
| `POST /api/v1/auth/logout` | Bearer | Revoca la sesión; `204`. |
| `POST /api/v1/users` | Owner | Crea otro usuario y devuelve su material de enrolamiento; `201`. |

Ejemplo de alta y primer paso de login:

```bash
base=http://127.0.0.1:3000
curl --fail-with-body -X POST "$base/api/v1/auth/bootstrap" \
  -H 'Content-Type: application/json' \
  --data '{"email":"owner@example.com","password":"correct horse battery staple"}'

curl --fail-with-body -X POST "$base/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  --data '{"email":"owner@example.com","password":"correct horse battery staple"}'
```

El segundo paso envía `challenge_token` y `code` a la ruta TOTP o recovery. La
respuesta contiene `access_token`, `token_type: "Bearer"` y
`expires_in_seconds`. Las peticiones protegidas usan:

```text
Authorization: Bearer <access_token>
```

Los tokens son 256 bits aleatorios y opacos. Redis conserva únicamente una
clave derivada por SHA-256, por lo que logout puede revocarlos de inmediato sin
persistir el token en claro. Cada petición protegida recarga desde PostgreSQL el
estado activo y el rol actual del usuario.

Cinco contraseñas rechazadas bloquean la clave normalizada del correo durante
15 minutos. Incremento y expiración son atómicos; la lectura repara contadores
heredados sin caducidad sin ampliar una ventana vigente. Los errores de
credenciales no revelan si la cuenta existe. Un
código TOTP válido se reclama atómicamente en Redis y no puede reutilizarse en
la ventana aceptada; el desafío se consume atómicamente antes de verificar el segundo factor. Dos
intentos simultáneos no pueden emitir dos sesiones con el mismo desafío; un
intento rechazado también lo consume. Redis requiere la operación `GETDEL`
(disponible desde Redis 6.2; el entorno incluido usa Redis 7).

Antes de emitir una sesión se vuelve a comprobar que la cuenta esté activa.
Si falla la auditoría después de crear un desafío o sesión en Redis, la
aplicación intenta retirarlo y devuelve un error sin token. Si también falla
esa limpieza, la clave puede permanecer hasta su expiración. Logout revoca
primero: un fallo posterior de auditoría devuelve error, pero no restaura la
sesión. No hay una transacción distribuida entre PostgreSQL y Redis.

## Expedientes y asignaciones

| Método y ruta | Acceso | Resultado |
| --- | --- | --- |
| `POST /api/v1/cases` | Owner o Litigante | Crea expediente y asigna al creador atómicamente; `201`. |
| `GET /api/v1/cases?limit=50&offset=0` | Bearer | Array de metadatos visibles, ordenado por UUID; `200`. |
| `GET /api/v1/cases/{uuid}` | Owner o miembro vigente | Metadatos del expediente; `200`. |
| `PUT /api/v1/cases/{uuid}/members/{user_uuid}` | Owner | Asigna un usuario activo; idempotente; `204`. |
| `DELETE /api/v1/cases/{uuid}/members/{user_uuid}` | Owner | Retira la asignación; idempotente; `204`. |

La creación y asignación inicial confirman su auditoría en la misma transacción.
Los cambios de miembros también revalidan al Owner y registran la mutación
atómicamente. La creación recibe JSON con `title` y `reference`, ambos
obligatorios. Se recortan espacios externos y se rechazan caracteres de control; los límites
son 200 y 100 caracteres Unicode, respectivamente. `reference` es una etiqueta
libre, no única. No se aceptan campos adicionales de actor, rol ni creador.
Las respuestas de creación y detalle tienen esta forma:

```json
{"id":"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa","title":"Defensa inicial","reference":"NUC-123","created_by":"bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"}
```

Owner ve todos los expedientes. Litigante, Paralegal y Cliente solo ven los que
tienen asignados. Un litigante creador queda asignado automáticamente, pero no
puede gestionar miembros. Solo Owner asigna y retira usuarios. Si se retira al
creador litigante, también pierde acceso. El detalle de un UUID ajeno responde
igual que uno inexistente: `404 case_not_found`.

La pertenencia se consulta en PostgreSQL en cada lectura y antes de paginar;
no se conserva en el token. Tras retirar una asignación, la siguiente petición
con la misma sesión ya no ve el expediente. Las lecturas concurrentes que ya
habían comenzado pueden finalizar con su instantánea anterior. El listado
admite `limit` entre 1 y 100 y `offset` entero no negativo; no ofrece una
instantánea entre páginas si otros usuarios cambian los datos.

PUT y DELETE no necesitan cuerpo. Asignar un usuario desconocido o inactivo
produce `404 user_not_found`; un expediente desconocido produce `404
case_not_found`. El cuerpo de creación tiene límite de 16 KiB.

## Documentos y permisos

Todas las operaciones documentales incluyen el expediente. La carga binaria
admite hasta 16 MiB y requiere `X-Document-Name`; la carga con clasificación
usa multipart con los límites descritos abajo. El actor se toma de
la sesión vigente; `X-Actor` no forma parte del contrato.

| Método y ruta | Permiso | Resultado |
| --- | --- | --- |
| `GET /api/v1/cases/{case_id}/documents` | Leer documentos | Página autorizada, filtros por nombre, sellado y clasificación actual; `200`. |
| `GET /api/v1/cases/{case_id}/documents/{document_id}` | Leer documentos | Detalle de metadatos persistidos, sin descifrar ni verificar evidencia; `200`. |
| `POST /api/v1/cases/{case_id}/documents` | Crear documento | Cifra y persiste la versión 1 asociada al expediente; `201`. |
| `POST /api/v1/cases/{case_id}/documents/with-metadata` | Crear y clasificar documento | Carga y clasificación inicial atómicas; `201`. |
| `POST /api/v1/cases/{case_id}/documents/{document_id}/seal` | Sellar documento | Firma el digest y emite/verifica el sello local; `200`. |
| `POST /api/v1/cases/{case_id}/documents/{document_id}/verify` | Verificar documento | Reporta integridad, firma, certificado/CRL y sello; `200`. |
| `GET /api/v1/cases/{case_id}/documents/{document_id}/evidence` | Exportar evidencia | Descarga ZIP con `X-Document-Digest`; `200`. |
| `GET /api/v1/audit/verify` | Verificar auditoría | Verifica la cadena PostgreSQL completa y reporta el primer índice roto. |

Carga, sellado y detalle devuelven `case_id`, `id`, `version`, `name`, `digest`
hexadecimal y `sealed` en el mismo objeto JSON. Listado, detalle actual y ambas
cargas añaden `current_metadata` con `metadata_revision`, `document_type`,
`classification` y `tags`. Anexar una versión también devuelve esta proyección
desde su propia transacción. Listado y detalle actual muestran la versión máxima.
Detalle exacto, historial de contenido y sellado conservan su formato de contenido,
sin incorporar clasificación actual a la evidencia histórica. Las antiguas rutas
`/api/v1/documents` y `/api/v1/documents/{document_id}/...` responden `404`; no
son alias de la API por expediente.

Owner puede operar sobre cualquier expediente. Litigante y Paralegal necesitan
asignación vigente además del permiso de la siguiente matriz:

| Acción | Owner | Litigante | Paralegal | Cliente |
| --- | --- | --- | --- | --- |
| Listar y consultar metadatos documentales | sí | sí | sí | no |
| Crear documento | sí | sí | sí | no |
| Clasificar y consultar su historial | sí | sí | sí | no |
| Añadir versión y consultar historial | sí | sí | sí | no |
| Sellar documento | sí | sí | no | no |
| Verificar documento | sí | sí | sí | no |
| Exportar evidencia | sí | sí | sí | no |
| Verificar auditoría completa | sí | no | no | no |
| Crear usuario | sí | no | no | no |

Cliente sigue sin acceso documental incluso asignado. Paralegal recibe `403`
al intentar sellar, también fuera de sus expedientes. Para un rol con permiso,
un documento que no pertenece al expediente indicado responde
`404 document_not_found`, incluso si el usuario es Owner o pertenece a ambos
expedientes. Una carga o listado sobre un expediente inexistente o ajeno responde
`404 case_not_found`.

Carga, sellado, verificación y exportación autentican antes de preparar el
resultado y otra vez antes de confirmarlo. Listado y detalle autentican la
sesión antes de consultar. La transacción comprueba rol activo, pertenencia y asociación
exacta. Retirar la asignación surte efecto con la misma sesión: si la revocación
se confirma primero, impide una confirmación documental posterior. Si la
operación documental obtiene primero el bloqueo, la revocación espera; una
respuesta ya confirmada puede terminar de transmitirse después de la revocación.

Sellar otra vez un documento sellado responde `409 document_already_sealed`
sin reemplazar su evidencia. Dos preparaciones concurrentes pueden solicitar
sellos, pero solo una confirma y la otra recibe conflicto. El contenido nuevo
se añade como otra versión; no se sobrescribe contenido ni se reasigna expediente.

### Listado y búsqueda de metadatos

El listado acepta `limit` de 1 a 100 (predeterminado 50), `offset` entero sin
signo de 32 bits (predeterminado 0), `name` opcional y `sealed` opcional con
valor `true` o `false`. `name` se recorta en los extremos, admite hasta 200
caracteres y rechaza controles; una cadena vacía omite el filtro. La búsqueda
es una subcadena literal del nombre; `%`, `_` y la barra inversa no actúan como
comodines. No se distingue entre mayúsculas y minúsculas en los nombres ASCII
admitidos por el contrato de carga. No se busca en el contenido cifrado.

`document_type`, `classification` y `tag` son filtros opcionales por igualdad
exacta, combinados mediante AND con nombre y sellado. Sus valores se validan y
recortan como al clasificar. Tipo y clasificación vacíos omiten el filtro;
una etiqueta vacía es inválida. Cada `tag` representa una etiqueta individual:
`tag=a%2Cb` busca la etiqueta `a,b`, no dos etiquetas. Distinguen mayúsculas,
acentos y formas de normalización Unicode. Se toma la última revisión de
clasificación antes de filtrar; una etiqueta histórica retirada no coincide.

```http
GET /api/v1/cases/{case_id}/documents?limit=25&offset=0&name=contrato&sealed=false
Authorization: Bearer <access_token>
```

La respuesta tiene forma `{"documents": [...], "has_more": false}`. Los
elementos usan el mismo formato del detalle. Se elige primero la versión actual
de cada documento y después se aplican los filtros antes de paginar; el orden
es UUID ascendente. Una versión anterior sellada no aparece como resultado
sellado si la actual está pendiente.
`has_more` informa si existía otro resultado en esa consulta, no el total del
despacho. La paginación por desplazamiento no mantiene una fotografía entre
peticiones: una inserción concurrente puede desplazar páginas posteriores.

Las consultas seleccionan únicamente metadatos y el indicador de evidencia
presente. No cargan el contenido cifrado, las firmas ni los certificados. Un
detalle `sealed: true` indica evidencia capturada, no una verificación integral
recién aprobada. La operación `POST .../verify` conserva esa responsabilidad.

Listado y detalle confirman `document.listed` y `document.read`, respectivamente,
en la misma transacción que revalida el acceso. Un fallo de auditoría impide
entregar el resultado. Los valores de búsqueda no se copian a la bitácora.
Límites o nombres inválidos responden `422`; parámetros desconocidos o con tipos
inválidos responden `400`. Una consulta válida sin resultados devuelve una lista
vacía, mientras que un expediente no autorizado conserva su respuesta `404`.

### Historial y versiones exactas

Base de las rutas siguientes:
`/api/v1/cases/{case_id}/documents/{document_id}`.

| Método y sufijo | Resultado |
| --- | --- |
| `POST /versions?expected_version=N` | Añade N+1 con UUID conservado; cuerpo binario hasta 16 MiB y `X-Document-Name`; devuelve metadatos con `201`. |
| `GET /versions?limit=50&before_version=N` | Historial descendente de metadatos, con cursor exclusivo y `200`. |
| `GET /versions/{version}` | Metadatos de la versión exacta; `200`. |
| `POST /versions/{version}/seal` | Sella solamente esa versión; metadatos con `200`. |
| `POST /versions/{version}/verify` | Reporte con `case_id`, `id`, `version` y los componentes/veredicto habituales; `200`. |
| `GET /versions/{version}/evidence` | ZIP exacto con `X-Document-Id`, `X-Document-Version` y `X-Document-Digest`; `200`. |

El historial devuelve `{"versions": [...], "has_more": true,
"next_before_version": 2, "first_available_version": 1}`. `limit` admite
1–100; `before_version` es opcional y positivo. Si hay otra página,
`next_before_version` identifica el cursor a enviar; en caso contrario es
`null`. La primera versión disponible puede ser mayor que uno para un origen
importado; no se fabrican sus revisiones previas. Las nuevas versiones aparecen
al refrescar la primera página, sin desplazar las páginas ya recorridas.

Una versión de ruta debe ser un entero positivo de 32 bits; valores inválidos
producen `400 invalid_document_version`. Un cursor cero, límite fuera de rango
o `expected_version=0` produce `422`; parámetros desconocidos, tipos inválidos
y ausencia de `expected_version` producen `400`. Archivo y cabecera de nombre
conservan las validaciones de la carga inicial. Una versión inexistente y una
asociación documental ajena conservan la misma respuesta `404 document_not_found`.

Si la cabeza ya no coincide con `expected_version`, el servidor responde
`409 document_version_conflict`. No cambia ninguna versión ni confirma un evento
de esa tentativa. El cliente conserva el archivo, refresca y permite revisarlo
antes de otro envío; no aumenta el número esperado automáticamente. Si se
agota el rango, responde `409 document_version_exhausted`.

Las rutas `/seal`, `/verify` y `/evidence` sin número siguen resolviendo una
única versión disponible. Si al resolver existen varias, responden
`409 document_version_required`. Una vez resuelta, la operación mantiene ese
snapshot exacto incluso si se añade otra revisión mientras se prepara. La web
utiliza siempre las rutas explícitas de la versión seleccionada.

Cada versión conserva su cifrado y evidencia independiente. Nombre, digest,
vault y asociación no se sobrescriben. Firma y TSA cubren el digest del contenido;
los metadatos HTTP no amplían lo firmado. La verificación histórica se evalúa
en la fecha de ejecución, con los certificados y CRL capturados y la política
vigente del verificador. Un indicador `sealed` no almacena un veredicto permanente.
Véase [ADR-0019](adr/0019-immutable-document-versions.md).

### Clasificación actual e historial independiente

Base: `/api/v1/cases/{case_id}/documents/{document_id}`.

| Método y sufijo | Resultado |
| --- | --- |
| `GET /metadata` | Clasificación actual, `200`. |
| `PUT /metadata` | Reemplazo completo con revisión esperada; clasificación confirmada, `200`. |
| `GET /metadata/history?limit=50&before_revision=N` | Revisiones descendentes con cursor exclusivo, `200`. |

GET y PUT devuelven `case_id`, `id`, `metadata_revision`, `document_type`,
`classification` y `tags` en un objeto plano. Tipo y clasificación ausentes son
`null`; etiquetas ausentes, `[]`. Un documento sin decisiones de clasificación
tiene revisión cero, sin inventar autor, fecha o fila histórica.

```json
{
  "expected_metadata_revision": 0,
  "document_type": "Escrito",
  "classification": "Penal",
  "tags": ["audiencia", "a,b"]
}
```

PUT requiere `expected_metadata_revision` entero sin signo de 32 bits y `tags`;
los otros valores pueden omitirse o ser `null`. Es un reemplazo, no un parche.
Se rechazan campos desconocidos, duplicados y JSON mal formado con
`400 invalid_json`. El cuerpo admite 8 KiB; excederlo produce
`413 metadata_too_large`. Los valores con controles son inválidos incluso en
los extremos. Se recorta espacio Unicode antes de admitir hasta 80 escalares
por tipo/clasificación y de 1 a 40 por etiqueta. Se aceptan hasta 20 etiquetas
recibidas, antes de eliminar duplicados exactos y ordenar por bytes UTF-8. Un
valor inválido produce `422 invalid_document_metadata`.

Cada reemplazo confirmado incrementa la revisión, incluso si repite valores o
los deja vacíos. Si la revisión esperada quedó atrás, responde
`409 document_metadata_conflict`; si se agotó el rango,
`409 document_metadata_revision_exhausted`. Ambos conservan estado y auditoría.
El cliente debe consultar y revisar los cambios antes de enviar otra revisión;
no debe incrementar automáticamente la esperada. La autorización se comprueba
antes del conflicto: un documento ajeno no revela su revisión.

El historial devuelve `case_id`, `id`, `revisions`, `has_more` y
`next_before_revision`. Cada fila contiene los cuatro campos de clasificación,
`metadata_digest` hexadecimal SHA-256, `changed_at` RFC 3339 en UTC y
`changed_by: {"id": "...", "email": "..."}` capturado en ese cambio.
`limit` admite 1–100; `before_revision` es opcional y positivo. El siguiente
cursor es la última revisión devuelta si hay más resultados; de lo contrario
es `null`. No se fabrica una fila para la revisión cero.

Lectura, historial y reemplazo confirman respectivamente
`document.metadata_read`, `document.metadata_history_listed` y
`document.metadata_changed`. Un fallo de auditoría impide entregar la lectura
o confirmar el cambio. El digest canónico identifica valores organizativos;
no amplía el contenido firmado. Cambiar clasificación preserva todas las
versiones, cifrado, firmas, sellos y archivos ZIP. Véase
[ADR-0020](adr/0020-audited-document-classification.md).

### Carga inicial con clasificación atómica

`POST /api/v1/cases/{case_id}/documents/with-metadata` requiere exactamente una
parte `file` y una parte `metadata` JSON, completas y en cualquier orden. El
JSON usa tipo, clasificación y etiquetas del contrato anterior, sin revisión
esperada. `X-Document-Name` es obligatorio y prevalece sobre el filename multipart.

```bash
curl --fail-with-body "$BASE_URL/api/v1/cases/$CASE_ID/documents/with-metadata" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H 'X-Document-Name: escrito.pdf' \
  -F 'file=@escrito.pdf;type=application/pdf' \
  -F 'metadata={"document_type":"Escrito","classification":"Penal","tags":[]};type=application/json'
```

Archivo: hasta 16 MiB. JSON: hasta 8 KiB. Cuerpo completo, incluidos cabeceras,
delimitadores y epílogo: hasta 16 MiB más 32 KiB. El servidor consume el cuerpo
completo con ese límite antes de interpretar sus partes y preparar criptografía;
no depende de Content-Length. Una interrupción del transporte no confirma la
carga. Partes faltantes, desconocidas, repetidas o incompletas responden
`400 invalid_multipart`; el tamaño produce `413 document_too_large`,
`metadata_too_large` o `upload_too_large`, según el límite alcanzado.

Raíz, contenido versión uno, clasificación revisión uno y los dos eventos de
éxito se confirman juntos. Si falla cualquiera, no queda una carga parcial.
La respuesta es el overview confirmado con `201`, incluso si los campos
opcionales están vacíos. La carga binaria anterior mantiene revisión de
clasificación cero. Qadra usa una sola carga multipart y conserva el formulario
ante fallos; una respuesta incierta no dispara reintentos automáticos.

## Persistencia y límites

Los usuarios viven en PostgreSQL con correo normalizado, hash PHC Argon2id,
rol, estado activo, secreto TOTP cifrado, códigos de recuperación hasheados y
revisión optimista. El secreto TOTP usa AES-256-GCM bajo `KEK_BASE64` y queda
ligado al UUID del usuario mediante datos autenticados.

La migración `0003_case_documents_audit.sql` añade documentos cifrados, evidencia
capturada, auditoría compartida y recibos de importación. El vault conserva el
formato `DVLT1` y su contexto autenticado de UUID y versión; el expediente se
protege mediante asociación inmutable y autorización transaccional. No se
reescribe el cifrado antiguo para añadir el expediente al AAD.
`0004_document_versions.sql` agrega raíces inmutables y cambia la clave de los
snapshots a `(id, version)`, preservando sus filas. `serve` exige ese esquema;
la actualización administrativa requiere detener escritores y conservar respaldo.
`0005_document_metadata.sql` añade revisiones de clasificación inmutables y
validación canónica con SHA-256 nativo. Migración y arranque exigen una base UTF-8.
El runtime no puede actualizar, borrar ni truncar estas revisiones. El arranque
comprueba inventario, permisos y coherencia; no autentica definiciones SQL
modificadas por un administrador de la base.

Las mutaciones documentales, de expedientes y de identidad comparten
transacción con su evento PostgreSQL. Verificación y exportación revalidan
acceso y estado documental y confirman su evento antes de devolver el
resultado. Un bloqueo común ordena las confirmaciones y la cabeza de auditoría;
la criptografía y las llamadas TSA se preparan fuera de esa transacción. Los
hashes usan el formato canónico existente y conservan las marcas temporales
con precisión de nanosegundos. El rol operativo puede leer y anexar eventos,
pero no actualizarlos, borrarlos ni truncarlos. Administradores y propietarios
de PostgreSQL siguen siendo parte de la base de confianza; el anclaje externo
de la cabeza permanece pendiente.

Los archivos `documents/<uuid>.json` y `audit.jsonl` corresponden al origen
legacy y al flujo offline. `database import --data-dir DIR --mapping MAPA`
inspecciona sin escribir; `--apply` importa un mapa completo a expedientes
existentes, conserva bytes cifrados, evidencia e historia y registra un recibo
transaccional. La inspección comprueba firma y sello; evalúa la cadena y CRL
del firmante en el instante autenticado del sello. Material del firmante
malformado, no confiable, revocado o fuera de vigencia en ese instante impide
importar. La confianza en la TSA conserva la política del verificador OpenSSL
actual, que evalúa su cadena en la fecha de ejecución: una TSA ya expirada
puede impedir importar evidencia histórica. Se conserva el origen para revisión
manual sin regenerar evidencia. La repetición reconcilia sin duplicar.
Los marcadores posteriores
al commit bloquean escritores actuales sobre el origen y pueden reconstruirse
si se interrumpe el proceso. Los ejecutables antiguos deben permanecer
detenidos. El procedimiento completo, sus precondiciones y la recuperación
están en [operaciones de base de datos](database-operations.md).

Los clientes PostgreSQL y Redis son síncronos. El arranque operativo abre
conexiones sin migrar el esquema. Redis usa plazos de cinco segundos para
conexión TCP y E/S después del handshake; el handshake del driver actual
todavía no tiene ese límite. La compensación de credenciales Redis ante fallos
de auditoría no extiende la atomicidad PostgreSQL a ambos servicios.

## Límites HTTP y sobrecarga

El servidor comparte un presupuesto entre identidad, documentos y expedientes:
como máximo ocho peticiones admitidas y dos operaciones bloqueantes concurrentes
por defecto. Puede configurarlos con `--max-in-flight-requests` y
`--max-blocking-operations`; ambos requieren enteros positivos. Cada hash Argon2id
utiliza 256 MiB, por lo que aumentar el segundo valor requiere medir capacidad.

Si no hay cupo se responde `503 server_busy`, sin encolar trabajo indefinido.
El permiso del trabajo bloqueante permanece ocupado hasta que este termina,
aunque el cliente cancele su petición. La respuesta no implica cancelación de
una mutación que ya estaba en curso. `/healthz` permanece fuera de admisión.

Los cuerpos JSON de identidad y expedientes tienen límite de 16 KiB y rechazan
campos desconocidos. Los documentos mantienen 16 MiB. Se rechazan cabeceras
Authorization múltiples o tokens con espacios; el esquema Bearer no distingue
mayúsculas. Las respuestas API incluyen `Cache-Control: no-store`.

Estos límites acotan cuerpo y ejecución, pero aún hacen falta límites de
conexiones, cuerpos lentos, TLS y apagado ordenado antes de despliegue público.

## Errores

La envoltura es estable:

```json
{"error":{"code":"permission_denied","message":"permission denied"}}
```

- `400`: UUID inválido.
- `401`: credenciales, segundo factor o sesión inválidos.
- `403`: rol autenticado sin permiso.
- `404`: documento, usuario o expediente inexistente; también expediente ajeno
  o documento fuera del expediente indicado.
- `409`: bootstrap cerrado, usuario duplicado, carrera optimista o transición
  documental incompatible.
- `422`: correo, contraseña, rol, nombre, metadatos de expediente, límite de
  página o cabecera inválidos.
- `429`: ventana de login bloqueada.
- `500`: fallo interno sin exponer detalles del backend ni secretos.
- `503`: presupuesto de peticiones u operaciones bloqueantes agotado (`server_busy`).

El contrato por expediente y las transacciones se definen en
[ADR-0016](adr/0016-case-document-transactions.md). Esta decisión sustituye la
autorización documental global y la persistencia HTTP en archivos descritas en
[ADR-0011](adr/0011-local-document-workflow.md) y
[ADR-0012](adr/0012-revocable-sessions-and-rbac.md). También aplican las reglas de
[pertenencia](adr/0014-case-membership-and-isolation.md) y
[límites de ejecución](adr/0015-backend-concurrency-and-invariants.md).

## Pruebas con persistencia real

```bash
bash scripts/test-backends.sh
```

El guion crea PostgreSQL y Redis temporales, usa bases distintas para identidad,
expedientes y documentos y elimina los servicios al terminar. También admite
un comando,
por ejemplo `bash scripts/test-backends.sh cargo test -p infrastructure --test
case_backends`. Una ejecución directa de Cargo sin
`IDENTITY_TEST_DATABASE_URL`, `IDENTITY_TEST_REDIS_URL`,
`CASE_TEST_DATABASE_URL` y `DOCUMENT_TEST_DATABASE_URL` omite las pruebas de
los servicios cuyas variables falten.
