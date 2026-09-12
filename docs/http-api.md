# API HTTP local autenticada

La aplicación integra identidad persistida, sesiones revocables y el flujo de
evidencia documental. PostgreSQL conserva usuarios, expedientes y asignaciones; Redis conserva desafíos,
sesiones, límites de intentos y reclamos de códigos TOTP; los documentos siguen
en el repositorio local cifrado. La TSA OpenSSL local emite sellos RFC 3161 y la
ejecución no consulta Cincel.

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
- un `paralegal` puede cargar y verificar, pero no sellar;
- los listados y detalles de expedientes excluyen los no asignados;
- retirar una asignación surte efecto usando la misma sesión;
- asignar un Cliente permite consultar metadatos, pero no documentos;
- logout invalida el token de inmediato;
- un código de recuperación funciona una sola vez;
- PostgreSQL no contiene la contraseña en claro;
- Redis no usa el bearer token en claro como clave;
- el documento persistido no contiene el texto claro;
- el ZIP exportado se verifica fuera de la aplicación con OpenSSL.

## Inicio manual

Puede iniciar los servicios de desarrollo con Podman Compose:

```bash
podman-compose up -d postgres redis
export DATABASE_URL='postgresql://despacho:local-development-only@127.0.0.1:5432/despacho'
export REDIS_URL='redis://127.0.0.1:6379/'
export KEK_BASE64="$(openssl rand -base64 32)"
```

Prepare la CA, el certificado firmante, la CRL y la TSA con los scripts de
`pki/`. Después arranque el proceso:

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

Las migraciones `migrations/0001_identity.sql` y `migrations/0002_cases.sql`
se aplican de forma idempotente al conectar. El servidor escucha solamente en `127.0.0.1:3000` por defecto.

## Autenticación

Todas las rutas usan `/api/v1`, salvo `/healthz`. El alta inicial se permite
solo mientras PostgreSQL no contiene usuarios; la inserción del primer owner se
protege con un bloqueo de tabla para conservar esa condición ante concurrencia.

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
15 minutos. Los errores de credenciales no revelan si la cuenta existe. Un
código TOTP válido se reclama atómicamente en Redis y no puede reutilizarse en
la ventana aceptada; cualquier segundo factor rechazado consume el desafío.

## Expedientes y asignaciones

| Método y ruta | Acceso | Resultado |
| --- | --- | --- |
| `POST /api/v1/cases` | Owner o Litigante | Crea expediente y asigna al creador atómicamente; `201`. |
| `GET /api/v1/cases?limit=50&offset=0` | Bearer | Array de metadatos visibles, ordenado por UUID; `200`. |
| `GET /api/v1/cases/{uuid}` | Owner o miembro vigente | Metadatos del expediente; `200`. |
| `PUT /api/v1/cases/{uuid}/members/{user_uuid}` | Owner | Asigna un usuario activo; idempotente; `204`. |
| `DELETE /api/v1/cases/{uuid}/members/{user_uuid}` | Owner | Retira la asignación; idempotente; `204`. |

La creación recibe JSON con `title` y `reference`, ambos obligatorios. Se
recortan espacios externos y se rechazan caracteres de control; los límites
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

Los documentos se reciben como cuerpo binario, con límite de 16 MiB. El actor
de auditoría se toma de la sesión; `X-Actor` ya no forma parte del contrato.

| Método y ruta | Permiso | Resultado |
| --- | --- | --- |
| `POST /api/v1/documents` | Crear documento | Cifra y persiste la versión 1; requiere `X-Document-Name`; `201`. |
| `POST /api/v1/documents/{uuid}/seal` | Sellar documento | Firma el digest y emite/verifica el sello local. |
| `POST /api/v1/documents/{uuid}/verify` | Verificar documento | Verifica integridad, firma, certificado/CRL y sello. |
| `GET /api/v1/documents/{uuid}/evidence` | Exportar evidencia | Descarga el ZIP y entrega `X-Document-Digest`. |
| `GET /api/v1/audit/verify` | Verificar auditoría | Verifica la cadena y reporta el primer índice roto. |

La matriz aplicada es conservadora:

| Acción | Owner | Litigante | Paralegal | Cliente |
| --- | --- | --- | --- | --- |
| Crear documento | sí | sí | sí | no |
| Sellar documento | sí | sí | no | no |
| Verificar documento | sí | sí | sí | no |
| Exportar evidencia | sí | sí | sí | no |
| Verificar auditoría completa | sí | no | no | no |
| Crear usuario | sí | no | no | no |

El cliente permanece sin acceso documental aunque tenga expedientes asignados.
Antes de habilitarlo, cada documento debe asociarse a un expediente y todas las
operaciones deben comprobar pertenencia. Las rutas documentales actuales del
personal todavía aplican permisos globales; conocer un UUID no concede acceso
documental al Cliente.

## Persistencia y límites

Los usuarios viven en PostgreSQL con correo normalizado, hash PHC Argon2id,
rol, estado activo, secreto TOTP cifrado, códigos de recuperación hasheados y
revisión optimista. El secreto TOTP usa AES-256-GCM bajo `KEK_BASE64` y queda
ligado al UUID del usuario mediante datos autenticados.

Cada documento vive en `DATA_DIR/documents/<uuid>.json`. Los campos binarios se
codifican en base64 y el texto claro permanece dentro del paquete cifrado
`DVLT1`. La bitácora vive en `DATA_DIR/audit.jsonl`. Documentos y auditoría aún
son archivos separados sin una transacción común; migrarlos a PostgreSQL y
asociar documentos a expedientes son incrementos posteriores. Los metadatos y
asignaciones de expedientes ya viven en PostgreSQL; sus mutaciones todavía no
se registran en la bitácora de archivo.

Los clientes PostgreSQL y Redis son síncronos para conservar una integración
pequeña. La frontera HTTP ejecuta los casos de uso en el pool bloqueante de
Tokio para no detener los workers asíncronos.

## Errores

La envoltura es estable:

```json
{"error":{"code":"permission_denied","message":"permission denied"}}
```

- `400`: UUID inválido.
- `401`: credenciales, segundo factor o sesión inválidos.
- `403`: rol autenticado sin permiso.
- `404`: documento, usuario o expediente inexistente; también expediente ajeno.
- `409`: bootstrap cerrado, usuario duplicado, carrera optimista o transición
  documental incompatible.
- `422`: correo, contraseña, rol, nombre, metadatos de expediente, límite de
  página o cabecera inválidos.
- `429`: ventana de login bloqueada.
- `500`: fallo interno sin exponer detalles del backend ni secretos.

Las decisiones se registran en
[`ADR-0011`](adr/0011-local-document-workflow.md) y
[`ADR-0012`](adr/0012-revocable-sessions-and-rbac.md) y
[`ADR-0014`](adr/0014-case-membership-and-isolation.md).

## Pruebas con persistencia real

```bash
bash scripts/test-backends.sh
```

El guion crea PostgreSQL y Redis temporales, usa bases distintas para identidad
y expedientes y elimina los servicios al terminar. También admite un comando,
por ejemplo `bash scripts/test-backends.sh cargo test -p infrastructure --test
case_backends`. Una ejecución directa de Cargo sin
`IDENTITY_TEST_DATABASE_URL`, `IDENTITY_TEST_REDIS_URL` y
`CASE_TEST_DATABASE_URL` omite las pruebas de esos servicios.
