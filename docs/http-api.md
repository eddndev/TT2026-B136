# API HTTP local autenticada

La [API de identidades representadas y participantes tipificados](typed-participants-api.md)
detalla revisión de identidad, perfiles, declaraciones internas, proyecciones
manuales y tipificadas, y consultas de evidencia histórica.
La [API de audiencias](hearings-api.md) define programación, reemplazo,
cancelación organizativa, historial exacto y agenda autorizada.
La [agenda combinada](agenda-api.md) reúne audiencias y vencimientos operativos
mediante `GET /api/v1/agenda`, con autorización y observación comunes por página.
La [API de sesiones y resultados declarados](hearing-results-api.md) añade
registro, rectificación, retiro e historia de comparecencias y acuerdos con
fuentes exactas. Está implementada y verificada localmente; sus mediciones se
registran separadas de las entregas anteriores en el informe de verificación.
La [API de calendarios jurisdiccionales](judicial-calendars-api.md) incorpora un
catálogo global de revisiones para clasificar fechas civiles. La API y la
restauración se verificaron localmente con servicios reales; la interfaz Qadra
y la cobertura también tienen verificación local registrada en el
[informe de verificación](verification-report.md).

La [API de hechos declarados de resolución y notificación](procedural-facts-api.md)
añade familias por expediente, padre fijo, preparación, confirmación, retiro e
historia con fuentes exactas. Sus pruebas focales, la suite global y el recorrido
HTTP con servicios reales y restauración pasaron localmente. La
[interfaz Qadra de hechos](../web/README.md#resoluciones-y-notificaciones-declaradas)
permite capturar, consultar y conciliar estas declaraciones.

La [API del catálogo de perfiles de plazo](deadline-profiles-api.md) adapta el
catálogo global y el de expediente, con publicación, reemplazo, retiro e
historia exacta. La [API de plazos](deadlines-api.md) registra la evaluación,
el responsable y la atención, con correcciones, retiro e historia inmutable.
La base V1 y su interfaz Qadra están integradas; sus campañas de restauración
y navegador conservan su alcance en el [informe de verificación](verification-report.md).

La ampliación humana/HTTP V2 está implementada y verificada localmente.
Alta y corrección exigen políticas explícitas; atención y retiro conservan
seguimiento y cálculo. Las lecturas representan evidencia V1/V2, autoría humana
o técnica y continuidad sin sustituir el historial. El detalle actual y el
listado comprueban la vigencia de las dependencias bajo lectura autorizada y
auditable. Véase el [contrato de seguimiento](deadline-tracking-api.md).

El [consumidor local](deadline-worker.md) conserva su puerto interno y está
compuesto en `serve`. Qadra V2 aprobó 88 pruebas Node, 36 recorridos con HTTP
controlado y una campaña separada de 25 recorridos con backend real, incluidos
dos escenarios Follow a 1440 y 390 píxeles; se inspeccionaron seis capturas.
La campaña API real comprobó reevaluación, cierre TERM/INT, reinicios y
restauración de R1-R5; la aceptación API final también aprobó. El cierre global
y la integración en `main` continúan; el informe conserva los resultados exactos.
La agenda conjunta incorpora su [contrato independiente](agenda-api.md);
la aceptación de cada entrega se registra en el informe. Las
[alertas personales](alerts-api.md) tienen servicio, router y cliente Qadra,
compuestos en `serve` con generación interna y transporte opcional;
la aceptación integrada de persistencia y runtime sigue pendiente. Véanse
[el alcance completo](deadline-lifecycle.md) y
[el presupuesto JSON del catálogo](deadline-profile-json-budget.md).

Contrato revisado el 2026-09-18. PostgreSQL conserva usuarios, expedientes,
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
- los listados y detalles excluyen expedientes no asignados, salvo para Owner;
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
export DOCUMENT_QPDF_LIBRARY="$(bash scripts/setup-document-formats.sh)"
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

El validador nativo requiere Linux x86_64 y qpdf 12.4.1. El arranque comprueba
la biblioteca en un proceso acotado; véase
[operación del validador](document-format-operations.md).

El servidor no ejecuta DDL y rechaza roles que puedan administrar o reescribir
la auditoría. `database migrate` aplica las migraciones de identidad,
expedientes, documentos/auditoría, calendarios jurisdiccionales, hechos declarados,
eventos de fuentes, perfiles y plazos persistentes. `--data-dir` señala el origen local
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

La creación básica, su revisión administrativa 1 sin perfil penal y la asignación
inicial confirman su auditoría en la misma transacción. No se registra una etapa
inicial para esta alta. Título y referencia de las respuestas son los vigentes.
Los cambios de miembros también revalidan al Owner y registran la mutación
atómicamente. La creación recibe JSON con `title` y `reference`, ambos
obligatorios. Se recortan espacios externos y se rechazan caracteres de control; los límites
son 200 y 100 caracteres Unicode, respectivamente. `reference` es una etiqueta
libre, no única. No se aceptan campos adicionales de actor, rol ni creador.
Las respuestas de creación y detalle tienen esta forma:

```json
{"id":"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa","title":"Defensa inicial","reference":"INTERNA-123","created_by":"bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"}
```

Owner ve todos los expedientes. Litigante, Paralegal y Cliente solo ven los que
tienen asignados. Un litigante creador queda asignado automáticamente, pero no
puede gestionar miembros. Solo Owner asigna y retira usuarios. Si se retira al
creador litigante, también pierde acceso. El detalle de un UUID ajeno responde
igual que uno inexistente: `404 case_not_found`.

El actor, rol y pertenencia se consultan en PostgreSQL dentro de cada lectura
auditable y antes de paginar; no se conservan en el token. El bloqueo común de
auditoría y READ COMMITTED revalidan los cambios confirmados durante la espera.
Tras retirar una asignación, la siguiente petición con la misma sesión ya no ve
el expediente. Una lectura que confirmó antes de la revocación puede entregar
su resultado. El listado
admite `limit` entre 1 y 100 y `offset` entero no negativo; no ofrece una
instantánea entre páginas si otros usuarios cambian los datos.

PUT y DELETE no necesitan cuerpo. Asignar un usuario desconocido o inactivo
produce `404 user_not_found`; un expediente desconocido produce `404
case_not_found`. El cuerpo de creación tiene límite de 16 KiB.

## Perfil penal y administración

El personal del despacho dispone de `POST /api/v1/penal-cases`,
`GET /api/v1/case-administrations`, GET/PUT
`/api/v1/cases/{id}/administration`, GET sobre su `/history` y PUT
`/api/v1/cases/{id}/administrative-status`. Owner administra todos; Litigator
asignado administra y Paralegal asignado consulta. Client conserva la proyección
básica de cuatro campos y no accede a estas seis operaciones.

El [contrato de administración](case-administration-api.md) detalla perfil,
revisiones esperadas, historial, filtros, respuestas, errores y cuerpo JSON
completo de 64 KiB. Distingue alta penal completa con registro inicial de
Investigación, alta básica con perfil pendiente y raíces anteriores sin historia.

Cerrar administrativamente bloquea edición del perfil, todas las mutaciones de
participantes, adopción/transiciones de etapa y carga, nuevas versiones,
clasificación y sellado documental.
Conserva lectura, historia, verificación, evidencia y cambios de asignación
por Owner. El servidor revalida el cierre al confirmar cada mutación; un caso
cerrado autorizado produce `409 case_closed`. El cierre no termina el proceso
judicial ni modifica etapas o plazos.

## Etapas procesales

El [contrato de etapas](case-stages-api.md) define GET `/stage`, GET
`/stage/history`, POST `/stage/adoption` y POST `/stage/transitions`, bajo
`/api/v1/cases/{case_id}`. Las consultas responden 200 y las mutaciones 201.
Owner gestiona todos; Litigator asignado gestiona; Paralegal asignado consulta;
Client no accede. Los cuerpos JSON tienen límite de 32 KiB.

Adopción registra la etapa conocida de un perfil completo sin registro previo.
Los avances ordinarios son Investigación a Intermedia e Intermedia a Juicio.
Cada soporte identifica documento, versión y digest; se comprueban cifrado,
evidencia capturada y formato PDF/DOCX antes de confirmar. Fecha declarada,
precisión y desfase se conservan aparte del instante y autor del registro.
La historia incluye el origen inicial sin reescribirlo. La etapa tiene revisión
propia; un conflicto exige comparación explícita y no genera reenvío automático.

La admisión de soportes utiliza [un worker acotado](document-format-operations.md).
No completa la validación de toda carga general ni acredita un acto judicial.
La programación de audiencias tiene su [contrato independiente](hearings-api.md).
Las sesiones y resultados declarados disponen de su
[contrato separado](hearing-results-api.md). El cálculo y la historia de plazos
usan [su propia colección](deadlines-api.md). La reevaluación compuesta en servidor
cuenta con recorridos API y navegador reales. Recursos y activación automática
conservan operaciones pendientes propias; las [alertas personales](alerts-api.md)
tienen su propio contrato y aceptación integrada pendiente.

## Audiencias y agenda

Owner y Litigator asignado pueden programar, reemplazar y cancelar citas;
Paralegal asignado consulta. Client no accede al módulo. El perfil completo,
las revisiones esperadas del expediente y su etapa, la actividad de las nuevas
fichas seleccionadas y la integridad del soporte se vuelven a comprobar al
confirmar. Las referencias retenidas conservan su revisión histórica exacta.
Cancelar copia la programación anterior aunque haya cambiado la etapa; exige
expediente activo y revisión esperada de audiencia.

Las rutas por expediente parten de `/api/v1/cases/{case_id}/hearings` y la agenda
de audiencias usa GET `/api/v1/hearings`, con intervalo UTC explícito y paginación
por instante y UUID. El [contrato completo](hearings-api.md) incluye cuerpos,
proyecciones, límites y errores. Cada revisión conserva un recibo propio para
conciliar respuestas perdidas sin repetir automáticamente la escritura.

Qadra usa [la agenda combinada](agenda-api.md) para las vistas diaria, semanal y
mensual, con rango personalizado opcional. Sus páginas reúnen audiencias y
plazos vigentes, preservan nanosegundos y pueden ser parciales aun sin filas.

El módulo de programación conserva las citas; el registro de sesiones declarado
se consulta por separado. Ninguno activa términos ni sustituye el calendario judicial.

## Sesiones y resultados declarados

Las rutas bajo `/api/v1/cases/{case_id}/hearings/{hearing_id}/results` ofrecen
preparación, alta, listado, detalle, rectificación, retiro, revisión exacta e
historia. Cada sesión tiene raíz propia, ancla inmutable a una revisión de
programación y continuidad opcional a un resultado exacto preexistente del mismo
expediente. Una rectificación conserva la raíz; una continuación crea otra.
Retirar conserva contenido e historia y es terminal, sin anular el acto.

Owner y Litigator asignado gestionan, Paralegal asignado consulta y Client queda
denegado. Las escrituras requieren expediente activo, revalidado tras obtener
el bloqueo común. Se admiten captura tardía, etapa posterior y referencias
históricas archivadas o retiradas. La administración observada al preparar es
informativa; la confirmación captura la vigente y su reloj, sin CAS de etapa
o administración. La historia devuelve resúmenes de hasta veinte revisiones;
seleccionar una revisión permite recuperar sus valores y fuentes completos.

El [contrato de resultados](hearing-results-api.md) define HRES1/HRTX1, límites,
códigos y proyecciones. Preparar no reserva filas; confirmar recalcula el recibo
y reautentica. Una respuesta incierta se concilia contra la revisión exacta y su
operación, sin reenvíos automáticos. Comparecencias, acuerdos y procedencia son
declarados por el operador; no infieren notificación, resolución ni plazos.

## Calendarios jurisdiccionales

El backend, la API y Qadra están verificados localmente, incluida la restauración
y los recorridos con servicios reales. La integración remota y el manuscrito
siguen pendientes. Su recurso global usa
`/api/v1/judicial-calendars`: Owner publica, reemplaza y retira; Owner,
Litigator y Paralegal consultan sin asignación a un expediente. Client queda
denegado. El comando no recibe identificadores de expedientes ni soportes
privados y el calendario no altera el perfil penal.

| Método y sufijo | Operación |
| --- | --- |
| `POST /prepare` | Devuelve comando normalizado, revisión resultante y digest del envío; no reserva filas ni UUID. |
| `POST` | Publica la revisión inicial; `201`. |
| `GET` | Lista cabezas con filtros de estado, fuero y clave de entidad, y cursor exclusivo por UUID. |
| `GET /{id}` | Consulta la cabeza actual. |
| `PUT /{id}` | Reemplaza valores completos con revisión esperada y motivo; `201`. |
| `POST /{id}/retirement` | Retira la raíz con revisión esperada y motivo; `201`. |
| `GET /{id}/history` | Consulta resúmenes descendentes de hasta veinte revisiones. |
| `GET /{id}/revisions/{revision}` | Recupera valores y recibo de una revisión exacta. |
| `GET /{id}/revisions/{revision}/days` | Clasifica el intervalo civil inclusivo `from`/`through`, de uno a 62 días. |

Cada raíz fija su ámbito completo en R1. Las revisiones conservan cobertura de
uno a 1096 días, siete reglas semanales explícitas, hasta 64 excepciones sin
solapamiento y hasta dieciséis referencias públicas declaradas. Las fechas
usan `YYYY-MM-DD`, años 1..9999, sin hora, desfase ni zona. La clasificación
puede ser `countable`, `excluded` o `unresolved`; fuera de cobertura devuelve
`outside_coverage`. Ni la ausencia de una fuente ni la falta de cobertura se
convierten en día hábil. Una excepción sustituye la regla semanal completa.

JCAL1 fija los valores y JCTX1 vincula actor, operación, raíz, revisión esperada,
digest y motivo. Confirmar recibe `{command,expected_submission_digest}`.
El servicio reautentica antes de confirmar; la transacción auditada vuelve a
comprobar el actor vigente, sus permisos, la revisión y el contenido. Retirar copia los valores anteriores y es terminal;
las revisiones exactas permanecen consultables. Una respuesta incierta se
concilia con su recibo completo, sin repetir automáticamente la escritura.

Las referencias guardan URL HTTPS y metadatos declarados. El servidor no visita
el enlace ni archiva una copia de su contenido. El digest no acredita
oficialidad, vigencia normativa ni aplicabilidad jurídica. Este catálogo no
calcula vencimientos, no selecciona calendarios por nombres de autoridades,
no infiere notificaciones desde audiencias y no crea alertas o tareas de
reevaluación. El [contrato completo](judicial-calendars-api.md) fija el perfil
acotado de URL, cuerpos estrictos de hasta 1 MiB, filtros, proyecciones y errores;
[ADR-0030](adr/0030-versioned-jurisdictional-calendars.md) delimita la decisión.

## Hechos declarados de resolución y notificación

La [API específica](procedural-facts-api.md) adapta el servicio de aplicación
al presupuesto compartido de HTTP. Usa dos bases:
`/api/v1/cases/{case}/resolutions` y
`/api/v1/cases/{case}/resolutions/{resolution}/notifications`.
Owner consulta y gestiona; Litigator asignado consulta y gestiona; Paralegal
asignado solo consulta; Client queda denegado. Un expediente actualmente cerrado
conserva lectura autorizada y rechaza mutaciones.

Cada base admite `GET` para listar y `POST /prepare` para preparar un comando
normalizado de su familia. `POST` confirma alta, `PUT /{id}` confirma corrección
y `POST /{id}/withdrawal` confirma retiro. Preparar devuelve `200`; confirmar
cualquiera de las tres acciones devuelve `201`. Confirmar recibe
`{command, expected_submission_digest}` y revalida el comando antes del commit.
Las consultas `GET /{id}`, `/{id}/revisions/{revision}` y `/{id}/history`
conservan cabeza, revisión exacta e historial ligero, respectivamente.

Los cuerpos JSON tienen un límite de 512 KiB y rechazan claves desconocidas,
repetidas y arreglos en lugar de objetos. Familia, identidad, padre y acción
deben coincidir con la ruta. Los listados admiten `limit` de 1 a 100 (defecto 20),
`after_id` exclusivo y `status` all/recorded/withdrawn. El historial usa `limit`
de 1 a 20 (defecto 10) y `before_revision` exclusivo. Las otras rutas no admiten
parámetros de consulta. No se sustituye una selección histórica por la cabeza.

Las respuestas incluyen valores, fuentes legibles acotadas, recibo y captura
administrativa discriminada. Una base sin revisión no fabrica revisión,
digest, autor ni fecha. Fecha, minuto, segundo y desfase conservan la precisión
declarada; no se infieren efectos, destinatarios o instantes. Retirar conserva
los valores y soportes admitidos, es terminal y no declara nulidad jurídica.
Las pruebas focales y la campaña integrada con servicios reales y restauración
tienen evidencia separada en el informe de verificación. La
[interfaz Qadra](../web/README.md#resoluciones-y-notificaciones-declaradas)
permite capturar, consultar y conciliar estas declaraciones.

## Recursos procesales declarados

La [API de recursos](procedural-resources-api.md) expone revocación y apelación
bajo `/api/v1/cases/{case_id}/procedural-resources`. Ofrece listado, detalle,
revisión exacta, historia, preparación y confirmación de registro, corrección,
actos, archivo y reactivación. Las escrituras conservan referencias exactas y
requieren un recibo preparado; no causan una transición de etapa ni activan
plazos por inferencia. Owner y Litigator asignado gestionan; Paralegal asignado
consulta y Client queda denegado. El cierre administrativo conserva las lecturas.

La interfaz Qadra comparte esos comandos y distingue corrección del recurso,
corrección de un acto y archivo organizativo. La asociación posterior con
audiencias, términos y alertas permanece pendiente. El alcance de aceptación
por capa y las comprobaciones reales se registran en el informe de verificación.

## Participantes del expediente

La ficha de participante tiene UUID propio y no es una cuenta ni una asignación
de acceso. Owner puede leer y gestionar todos los directorios; Litigator requiere
asignación vigente para leer y gestionar, y Paralegal para leer. Client no tiene
acceso a participantes, aunque puede consultar los metadatos básicos de su
expediente asignado. El rol procesal escrito en una ficha no concede permisos.

Base: `/api/v1/cases/{case_id}/participants`.

| Método y sufijo | Resultado |
| --- | --- |
| `POST /` | Crea ficha activa con revisión 1; snapshot confirmado, `201`. |
| `GET /` | Página de fichas actuales autorizadas, `200`. |
| `GET /{participant_id}` | Snapshot actual, `200`. |
| `PUT /{participant_id}` | Reemplaza todos los valores con revisión esperada; snapshot confirmado, `200`. |
| `PUT /{participant_id}/directory-status` | Cambia solamente el estado organizativo sobre los valores vigentes; snapshot confirmado, `200`. |
| `GET /{participant_id}/history` | Página de snapshots históricos descendentes, `200`. |

Los sufijos `/` de colección representan la ruta base sin barra final. No hay
eliminación física. La creación acepta este JSON:

```json
{
  "display_name": "Ana",
  "procedural_role": "Testigo",
  "organization": "Despacho",
  "legal_status": "Dato registrado por el equipo"
}
```

Nombre y rol son obligatorios. Organización y estatus jurídico pueden omitirse
o ser `null`; vacíos tras recortar espacios se convierten a `null`. Sus máximos
son respectivamente 200, 80, 200 y 160 escalares Unicode. Se rechazan controles
antes de recortar espacios exteriores, incluso si luego desaparecerían. Se
preservan espacios internos, acentos, mayúsculas, puntuación y formas Unicode.
Estos textos son datos manuales, no comprobaciones de identidad o legitimación.
Se permiten homónimos y el servidor fuerza `directory_status: "active"` al crear.

El reemplazo completo exige `expected_revision`, nombre, rol y
`directory_status`; los opcionales omitidos quedan vacíos. El cambio de estado
acepta exclusivamente:

```json
{"expected_revision": 3, "directory_status": "archived"}
```

Solo son válidos `active` y `archived`. Archivar o reactivar no cambia el texto
vigente, la situación jurídica, el expediente ni sus asignaciones. El servidor
lee el snapshot y comprueba la revisión dentro de la misma transacción. Una
revisión desactualizada produce `409 participant_revision_conflict`; un contador
agotado, `409 participant_revision_exhausted`. No se crea revisión ni evento
exitoso en esos casos. Una sustitución aceptada, incluso idéntica, agrega una
revisión. El cliente debe revisar los datos actuales y reenviar explícitamente;
no debe reintentar silenciosamente con una revisión nueva.

La respuesta de creación, detalle y ambas mutaciones contiene:

```json
{
  "case_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  "id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
  "revision": 4,
  "display_name": "Ana",
  "procedural_role": "Testigo",
  "organization": null,
  "legal_status": null,
  "directory_status": "archived",
  "values_digest": "7a9b26dd8018c0c32f1d51be87d6859821930796d55e168e193d1c28d68b76d0",
  "changed_at": "2026-09-14T23:30:00Z",
  "changed_by": {"id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", "email": "actor@example.com"}
}
```

El digest identifica los valores canónicos mediante SHA-256, descrito en [ADR-0021](adr/0021-audited-case-participants.md). No firma
por sí solo al autor. `changed_by` conserva UUID y correo capturados al cambiar
esa revisión; no reconstruye historia con el perfil actual. `changed_at` se
devuelve en UTC. Las revisiones ordenan los cambios; el reloj no garantiza
instantes crecientes bajo concurrencia.

El listado acepta `limit` entre 1 y 100 (50 predeterminado), `after_id` UUID
exclusivo, `name`, `procedural_role` y `status` (`active` predeterminado,
`archived` o `all`). Nombre busca una subcadena literal y distingue mayúsculas;
rol exige igualdad exacta. Ambos filtros recortan espacios exteriores, rechazan
controles y tienen los máximos de sus campos; un filtro vacío se omite. `%` y `_`
no son comodines. Se elige la revisión actual antes de filtrar y paginar, por
UUID ascendente. La respuesta tiene `participants`, `has_more` y `next_after_id`;
este último es `null` cuando no hay otra página. No ofrece un total global ni
una instantánea común entre peticiones.

Historia acepta `limit` con los mismos límites y `before_revision` positivo,
exclusivo. Devuelve `revisions` con los snapshots completos, `has_more` y
`next_before_revision`, `null` al terminar. `before_revision=1` devuelve
página vacía para un recurso existente y autorizado. La revisión cero es
inválida; no representa una ficha sin historia.

Todas las entradas JSON rechazan campos desconocidos o repetidos con `400
invalid_json`. UUID o parámetros mal formados producen `400`. Los valores
inválidos producen `422 invalid_participant_values`; revisión esperada cero,
`422 invalid_participant_revision`; límites o filtros inválidos, `422
invalid_input`. El cuerpo JSON completo admite hasta 8 KiB y excederlo produce
`413 participant_body_too_large`. Los máximos de texto caben incluso con
escalares astrales escapados; espacios de formato y demás bytes también cuentan.

Las consultas confirman auditoría antes de devolver datos; cada mutación
confirma ficha y evento juntos. Sesión inválida da `401`; rol sin permiso, `403`.
Crear o listar sobre expediente inexistente u oculto da `404 case_not_found`.
Las rutas de ficha no distinguen participante ausente, expediente oculto o
asociación ajena: `404 participant_not_found`. La autorización precede al
conflicto de revisión. Datos persistidos incoherentes producen `500
internal_error` sin exponer valores internos. Todas las respuestas usan
`Cache-Control: no-store` y el presupuesto HTTP común.

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

`0006_case_participants.sql` añade raíces y revisiones inmutables de participantes,
con autoría capturada, validación canónica y primera revisión activa obligatoria.
Su estado organizativo no modifica membresías ni evidencia documental.

`0007_case_administration.sql` añade revisiones del perfil y estado del expediente,
registro inicial de etapa separado y una referencia diferida a R1 para altas
nuevas. Las raíces anteriores permanecen sin historia administrativa inventada.
La comparación de identificadores actuales y los cambios usan READ COMMITTED
explícito bajo el bloqueo común de auditoría. Véase el
[contrato del perfil penal](case-administration-api.md).

`0011_hearings.sql` y sus archivos auxiliares añaden raíces de audiencias y
revisiones inmutables. Se comprueban el canon de valores y de operación, sus
proyecciones, la secuencia y las referencias históricas. El inventario de
arranque recorre las revisiones y resuelve sus fuentes exactas. No se generan
audiencias a partir de fechas anteriores ni se modifican documentos o etapas.
El conjunto `0012_hearing_results*.sql` añade raíces y revisiones de resultados,
canon HRES1, recibo HRTX1 y fuentes históricas exactas. No deriva sesiones de las
citas existentes. El arranque verifica catálogo, privilegios, secuencias,
referencias e inventario; la captura comparte transacción con la auditoría.

El conjunto `0013_judicial_calendar_*.sql` añade `judicial_calendars` y
`judicial_calendar_revisions`, canon JCAL1 y recibos JCTX1. La raíz exige R1
mediante una clave foránea diferida; las revisiones preservan el ámbito inicial,
la secuencia, la unicidad de operación y el retiro terminal. Las proyecciones
SQL se derivan de los bytes canónicos y se contrastan al leer. Los calendarios
anteriores no se deducen de citas ni de metadatos de expedientes.

Las mutaciones documentales, de participantes, de audiencias y sus resultados,
de calendarios, de expedientes y de identidad comparten
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

El servidor comparte un presupuesto entre identidad, documentos, participantes,
etapas, audiencias, resultados declarados, calendarios y expedientes:
como máximo ocho peticiones admitidas y dos operaciones bloqueantes concurrentes
por defecto. Puede configurarlos con `--max-in-flight-requests` y
`--max-blocking-operations`; ambos requieren enteros positivos. Cada hash Argon2id
utiliza 256 MiB, por lo que aumentar el segundo valor requiere medir capacidad.

Si no hay cupo se responde `503 server_busy`, sin encolar trabajo indefinido.
El permiso del trabajo bloqueante permanece ocupado hasta que este termina,
aunque el cliente cancele su petición. La respuesta no implica cancelación de
una mutación que ya estaba en curso. `/healthz` permanece fuera de admisión.

Los cuerpos JSON de identidad y alta básica de expedientes tienen límite de 16 KiB y rechazan
campos desconocidos. Participantes y clasificación JSON tienen límites de 8 KiB.
Etapas admiten 32 KiB; programación de audiencias y administración penal, 64 KiB.
Los resultados declarados admiten 512 KiB por JSON y conservan el mismo
presupuesto compartido; su historia devuelve hasta veinte resúmenes por página.
Los calendarios admiten 1 MiB, incluidos envoltura y espacios; las cotas de los
valores son independientes de ese límite de transporte. Rechazan campos o
queries desconocidos y claves repetidas, también en rutas sin consulta.
Los documentos mantienen 16 MiB. Se rechazan cabeceras
Authorization múltiples o tokens con espacios; el esquema Bearer no distingue
mayúsculas. Las respuestas API incluyen `Cache-Control: no-store`.

Estos límites acotan cuerpo y ejecución, pero aún hacen falta límites de
conexiones, cuerpos lentos, TLS y apagado ordenado antes de despliegue público.

## Plazos registrados por expediente

El [contrato de plazos](deadlines-api.md) define la colección
`/api/v1/cases/{case_id}/deadlines`: preparación y confirmación con recibo,
corrección completa, declaración de atención, retiro terminal y consulta de
revisiones exactas. Owner y Litigator escriben; el personal autorizado consulta,
Client queda denegado y el cierre conserva lecturas. Cada cuerpo admite 1 MiB y
utiliza el presupuesto de ejecución compartido.

Las respuestas conservan perfil, fuentes, observaciones y resultado histórico,
con bloqueos y traza estructurada. Consultar no vuelve a ejecutar la aritmética.
Los instantes preservan segundos, nanosegundos y desfase explícitos; las
declaraciones conservan su precisión. El detalle actual y el listado añaden
`operational` con `freshness`, `checked_at`, `changed_dependencies` y `due_at`.
La revisión exacta y la respuesta de confirmación conservan `not_checked` y no
acreditan vigencia actual. El listado separa `calculation_due_at` y
`calculation_blocked` de esa proyección operativa.

La frescura se determina comparando evidencia verificada con las observaciones
capturadas según las políticas. Ni trabajos pendientes ni un resultado
`Completed` acreditan o descartan por sí solos esa vigencia. El contrato
[de seguimiento](deadline-tracking-api.md) define los estados y la conservación
de V1. Las campañas históricas de API/restauración corresponden a V1; las
pruebas focales actuales de V2 tienen evidencia separada.

`GET /responsibles` ofrece cuentas activas elegibles, con `limit` de 1 a 100
(por defecto 20), `after_id` exclusivo y orden UUID ascendente. Owner es elegible
sin asignación; Litigator y Paralegal necesitan membresía vigente al expediente.
Client queda excluido como lector y como candidato. La respuesta entrega
`case_id`, `responsibles:[{id,email,role}]`, `has_more` y `next_after_id`;
la consulta autoriza el expediente y confirma su evento de auditoría. No crea
una membresía ni reserva elegibilidad para registrar o corregir después.

La colección mantiene el prefijo `/api/v1`; la versión del recibo se declara
por separado. El contrato humano actual exige `change.tracking` en registro
y corrección. Las políticas son explícitas para perfil, fuente y calendario;
atención y retiro no reciben políticas de reemplazo. El servidor obtiene el
autor humano de la sesión y rechaza autoría técnica o comandos de trabajador
aportados por el cliente. Las consultas sí representan recibos históricos V1,
capturas V2 y la procedencia técnica de revisiones confirmadas por el consumidor.

Las campañas históricas de Qadra con servicios reales corresponden a V1.
La interfaz V2 aprobó 88 pruebas Node y 36 recorridos con HTTP controlado.
La campaña posterior con backend real aprobó 25 recorridos, incluidos dos Follow
de escritorio y móvil; seis capturas se inspeccionaron visualmente. La campaña
API real comprobó revisiones R1-R5, cierre TERM/INT, reinicios y restauración.
Estos resultados ejercitan el [consumidor compuesto](deadline-worker.md) y se
registran por separado en el [informe de verificación](verification-report.md).
La aceptación API final aprobó. El cierre global y la integración en `main`
siguen en curso. La agenda de audiencias y vencimientos reunidos tiene
[un contrato propio](agenda-api.md); las alertas personales se describen a continuación.

## Alertas personales

El [contrato de alertas](alerts-api.md) incorpora `GET/PUT /api/v1/alert-preferences`,
`GET /api/v1/alerts`, detalle por identificador y `POST /api/v1/alerts/{id}/read`.
Son recursos personales de staff, sin suscripciones ni acceso Client. Owner
no obtiene bandejas ajenas por su rol. Los cuerpos, recibos, filtros y cursores
se validan antes de proyectar una respuesta; el servicio reautentica al principal.

Las preferencias admiten anticipaciones configurables, inicialmente 48/24 horas,
y canales por los cuatro motivos de aviso. Leer no declara atención; el origen
histórico no acredita vigencia operativa. `email.kind=accepted` sólo confirma
aceptación por el proveedor y `disabled` expresa transporte no configurado.
El router y el servicio de aplicación están compuestos en `serve` junto con
Qadra. La [configuración del servidor](alerts-api.md#configuración-y-aceptación-del-servidor)
detalla el correo opcional y los límites del consumidor. Los recorridos API y
navegador real están preparados; su ejecución integrada sigue pendiente y se
registra por separado de la verificación con HTTP controlado.

## Errores

La envoltura es estable:

```json
{"error":{"code":"permission_denied","message":"permission denied"}}
```

- `400`: UUID, JSON o parámetro tipado inválido según el contrato de la ruta.
- `401`: credenciales, segundo factor o sesión inválidos.
- `403`: rol autenticado sin permiso.
- `404`: documento, participante, audiencia, calendario, usuario o expediente inexistente; también
  recurso oculto o fuera del expediente indicado.
- `409`: bootstrap cerrado, usuario duplicado, carrera optimista, expediente
  cerrado, etapa incompatible o soporte cambiado durante la preparación; también
  revisión, operación, retiro terminal o contador agotado de calendario.
- `422`: correo, contraseña, rol, nombre, metadatos de expediente, límite de
  página, cabecera, fecha o soporte procesal inválidos.
- `413`: cuerpo mayor que el límite de la ruta.
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
