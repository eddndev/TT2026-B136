# Base de datos y migración de documentos

El servidor usa PostgreSQL para usuarios, expedientes, participantes, etapas,
audiencias y sus resultados declarados, calendarios jurisdiccionales, documentos
y una sola cadena de auditoría. Redis conserva las sesiones y controles efímeros. La decisión
está en [ADR-0016](adr/0016-case-document-transactions.md).

## Preparar un despliegue nuevo

Crear previamente una base UTF-8 y un rol de conexión sin privilegios administrativos.
Por ejemplo, desde una sesión de administración PostgreSQL:

```sql
CREATE ROLE tt_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE;
```

Configurar su autenticación en PostgreSQL según el entorno. Ejecutar la migración
con `DATABASE_URL` administrativa; el comando no crea usuarios PostgreSQL:

```bash
cargo build --workspace
export DATABASE_URL='postgresql://administrador@localhost/despacho'
target/debug/despacho-cli database migrate --runtime-role tt_runtime
```

Para `serve`, cambiar `DATABASE_URL` por la del rol operativo. El arranque valida
sus privilegios y no ejecuta DDL. Conservar KEK, certificados, claves y configuración
TSA fuera del repositorio y preparar Redis. Antes de arrancar `serve`, instalar
el validador obligatorio de soportes PDF/DOCX en Linux x86_64:

```bash
export DOCUMENT_QPDF_LIBRARY="$(bash scripts/setup-document-formats.sh)"
```

El instalador verifica el artefacto fijado de qpdf 12.4.1. El servidor comprueba
la biblioteca y el worker antes de escuchar; no omitir esa comprobación ante
fallos. Proteger la instalación y conservar sus bibliotecas acompañantes. El
argumento `--qpdf-library` sustituye la variable. Los argumentos criptográficos
de `serve --help` siguen vigentes. Véase
[operación del validador](document-format-operations.md).

Comprobar `SHOW server_encoding` en la base de destino: debe devolver `UTF8`.
Las comprobaciones canónicas de clasificación usan escalares Unicode y SHA-256
nativo de PostgreSQL; no requieren `pgcrypto`. Una base con otra codificación
debe migrarse mediante el procedimiento administrativo de conversión y
restauración antes de aplicar el esquema; no basta con cambiar `client_encoding`.

## Actualizar la base a versiones documentales

Detener todos los escritores y respaldar la base antes de ejecutar
`database migrate --runtime-role` con el nuevo ejecutable y credenciales
administrativas. `migrations/0004_document_versions.sql` agrega `document_series`
y cambia la clave de `documents` a `(id, version)`; conserva los vaults y la
evidencia existentes. La migración puede repetirse, pero no permite mantener
escritores antiguos activos ni volver a un servidor que supone UUID único.

Validar antes de abrir tráfico: cada raíz corresponde al mismo expediente y
primera versión existente; los bytes cifrados, evidencia y prefijo de auditoría
coinciden con el respaldo. Comparar una exportación sellada anterior con la ruta
explícita `/versions/{version}/evidence`. No pedir sellos nuevos para hacer la
migración ni renumerar snapshots importados.

El rol operativo conserva SELECT/INSERT sobre `document_series` y `documents`,
sin privilegios para actualizar raíces, borrar historia o modificar contexto;
solo puede actualizar `documents.evidence`. `serve` valida esquema, privilegios
y coherencia de metadatos al abrir conexiones, sin ejecutar DDL. La comprobación
de secuencias debe medirse al dimensionar el arranque con volúmenes grandes.

## Actualizar la clasificación documental

Con escritores detenidos y respaldo completo, ejecutar nuevamente
`database migrate --runtime-role`. `migrations/0005_document_metadata.sql`
añade `document_metadata_revisions` y funciones puras de validación canónica.
Los documentos existentes permanecen sin clasificación, con revisión lógica
cero; no se inventan fechas ni autores. Los bytes cifrados y la evidencia de
cada snapshot permanecen intactos.

El rol operativo necesita SELECT/INSERT sobre la tabla y EXECUTE sobre sus
funciones de comprobación. No debe ser propietario ni disponer de UPDATE,
DELETE o TRUNCATE. El comando administrativo aplica estos permisos y el
arranque los comprueba junto con el inventario y la continuidad de revisiones.
Las revisiones capturan el correo y UUID del actor, sin reconstruirlos a partir
de su perfil actual. No corregir historia con UPDATE ni borrar filas para
resolver un conflicto de revisión; el cliente debe consultar y enviar la
revisión esperada vigente.

El guard de esquema comprueba catálogo y coherencia, no autentica el cuerpo de
funciones que un administrador hubiera sustituido. La administración de la base
sigue dentro del límite de confianza descrito en
[ADR-0020](adr/0020-audited-document-classification.md).

Las funciones de clasificación guardan referencias explícitas al esquema donde
se instalaron, de modo que los CHECK también funcionan durante `pg_restore`,
que ejecuta con un `search_path` vacío. Usar clientes `pg_dump` y `pg_restore`
compatibles con la versión del servidor. Las pruebas aisladas de backend
requieren ambos ejecutables y comprueban una restauración real con restricciones
y permisos activos.

## Actualizar el directorio de participantes

Detener escritores y obtener un respaldo completo antes de ejecutar
`database migrate --runtime-role` con el nuevo binario y conexión administrativa.
`migrations/0006_case_participants.sql` añade `case_participants` y
`case_participant_revisions`, sin derivar personas de las cuentas asignadas.
Los expedientes existentes empiezan con el directorio vacío y sus documentos
mantienen todas las versiones y evidencias.

El rol operativo necesita SELECT/INSERT sobre ambas tablas y las funciones puras
de comprobación, sin propiedad, UPDATE, DELETE ni TRUNCATE. El arranque comprueba
esquema, privilegios y coherencia. Las raíces exigen revisión activa inicial;
cada sucesor conserva valores, SHA-256, actor y fecha. No reparar conflictos con
UPDATE ni borrar historia: el usuario debe revisar y enviar la revisión vigente.
Las funciones fijan su esquema para permitir restauración con `search_path` vacío.

Respaldar ambas tablas junto con usuarios, expedientes, membresías y auditoría.
Las claves foráneas incluyen una referencia diferida de la raíz a su primera
revisión; no cargar raíces huérfanas ni fabricar autores al restaurar. Comparar
filas completas y secuencias, incluida la revisión actual e historia de cada
ficha. Los textos son metadatos autorizados sin cifrado de archivo; las copias
requieren los controles operativos correspondientes. Véase
[ADR-0021](adr/0021-audited-case-participants.md).

## Actualizar el perfil y la administración de expedientes

Con los escritores detenidos y respaldo completo, ejecutar
`database migrate --runtime-role` mediante una conexión administrativa.
`migrations/0007_case_administration.sql` añade
`case_administration_revisions`, `case_initial_stage_registrations` y
`cases.required_initial_revision`. Las raíces anteriores conservan el marcador
NULL: se leen como revisión cero sin inventar perfil, autor, fecha ni etapa.
Las nuevas altas normales tienen marcador 1 y exigen su revisión 1 por clave
foránea diferida. El alta básica no inicializa una etapa; el alta penal completa
sí registra Investigación, vinculada a su primera revisión exacta.

El rol operativo inserta únicamente `id`, `title`, `reference` y `created_by`
en la raíz; no puede insertar el marcador ni `created_at`. Necesita SELECT e
INSERT sobre las nuevas tablas y EXECUTE sobre las funciones de comprobación,
sin propiedad, UPDATE, DELETE ni TRUNCATE. No conceder permisos heredados o
asumibles que eludan esas restricciones. El arranque comprueba columnas,
restricciones, funciones, privilegios e inventario sin reparar datos.

Las revisiones preservan valores normalizados, huella CADM1, UUID/correo del
autor y fecha capturados. Los identificadores se comparan entre cabezas vigentes,
incluidos expedientes cerrados. La auditoría común y las escrituras usan READ
COMMITTED explícito, incluso si la conexión tiene otro valor predeterminado;
los triggers rechazan escrituras administrativas bajo otro aislamiento. Las
funciones fijan referencias al esquema para admitir `pg_restore` con
`search_path` vacío. PostgreSQL debe usar UTF8.

No convertir una raíz nueva en baseline, borrar una revisión ni cambiar su autor
para resolver un conflicto o preparar una importación. Una corrección de perfil
se registra con revisión esperada; una reapertura usa el comando de estado.
El cierre administrativo bloquea mutaciones documentales, de participantes y
de etapas, pero mantiene lecturas, evidencia y revocación de miembros por Owner.
Conserva las etapas y los estados de participantes. Véase
[ADR-0022](adr/0022-audited-penal-case-administration.md).

Respaldar y restaurar ambas tablas junto con raíces, asignaciones, usuarios,
documentos, clasificación, participantes, auditoría y recibos. Comparar filas
completas, revisión vigente, historial y registro inicial; no basta igualar
conteos. No importar solo raíces con marcador 1 sin su revisión. El perfil es
metadato autorizado en PostgreSQL y no forma parte del archivo cifrado DVLT1.

## Actualizar adopción y transiciones de etapa

Detener escritores, obtener un respaldo completo y ejecutar
`database migrate --runtime-role` con credenciales administrativas. El comando
aplica `migrations/0008_case_stages.sql`, `0008_case_stage_values.sql` y
`0008_case_stage_guards.sql`. Añaden `case_stage_revisions`, comprobaciones
canónicas y triggers; conservan `case_initial_stage_registrations` y no fabrican
etapas para los expedientes anteriores. No ejecutar fragmentos de migración
por separado ni mantener escritores de la versión anterior.

El rol operativo necesita SELECT/INSERT sobre la tabla y EXECUTE sobre las
funciones de comprobación, sin propiedad, UPDATE, DELETE o TRUNCATE. `serve`
comprueba esquema, permisos e inventario sin ejecutar DDL. La secuencia de etapa
combina el registro inicial y las revisiones nuevas; R1 inicial y R1 de adopción
son excluyentes. La revisión administrativa es independiente. No reparar
conflictos borrando filas, sustituyendo R1 o deshabilitando restricciones.

Cada entrada conserva valores CSTG1 y digest, revisión administrativa exacta,
fechas declaradas con precisión/desfase, actor y captura del sistema, más nombre,
UUID, versión, digest y política de sus soportes. Las claves foráneas alcanzan
las versiones exactas y la administración referenciada. La preparación valida
integridad/formato; el commit auditado vuelve a comprobar autorización, estado
activo, perfil completo, revisión esperada y evidencia preparada. Un sellado
concurrente exige validación explícita de nuevo; un sello o append posterior
conserva la entrada histórica.

Los triggers usan READ COMMITTED y referencias de esquema explícitas para
restauración con `search_path` vacío. Respaldar la tabla junto con registros
iniciales, administración, documentos/versiones, usuarios, membresías y auditoría.
Comparar filas completas, precisión temporal, procedencia, ambas clases de R1
y vínculos exactos; no basta comparar conteos o la etapa actual. Los valores de
etapa son metadatos autorizados sin cifrado de archivo; proteger sus copias.
Véanse [ADR-0023](adr/0023-audited-case-stage-transitions.md),
[contrato de etapas](case-stages-api.md) y
[admisión de formatos](document-format-operations.md).

## Migrar un almacenamiento local existente

1. Detener todos los escritores, incluidas versiones anteriores del servidor y
   comandos `audit append` que apunten al archivo compartido. Conservar respaldo
   de la base existente, directorio local, KEK y material criptográfico.
2. Aplicar el esquema nuevo sobre la base que ya contiene usuarios y expedientes.
   No iniciar todavía el servidor: el primer import exige documentos,
   clasificación, participantes, revisiones administrativas, registros iniciales,
   revisiones de etapa, calendarios jurisdiccionales y auditoría vacíos para
   conservar la cadena original
   como prefijo.
   Preparar los expedientes de destino administrativamente como baselines con
   marcador NULL y hechos conocidos; no crearlos por HTTP y borrar sus eventos
   o revisiones. Una ficha cargada directamente también impide esa importación
   inicial. La conciliación de un recibo existente sigue permitiendo
   las operaciones posteriores válidas.
3. Elaborar el mapa explícito. Cada JSON documental debe aparecer exactamente una
   vez y el expediente debe existir en la base. No se infieren asociaciones:

```json
{
  "documents": [
    {
      "document_id": "11111111-1111-4111-8111-111111111111",
      "case_id": "22222222-2222-4222-8222-222222222222"
    }
  ]
}
```

4. Inspeccionar con la KEK original. El modo predeterminado no crea archivos ni
   escribe en la base; valida contexto de cifrado, digest, evidencia y cadena:

```bash
export KEK_BASE64='valor-de-la-kek-original'
target/debug/despacho-cli --json database import \
  --data-dir /ruta/a/runtime-data --mapping /ruta/a/mapping.json
```

5. Revisar conteos, huella de fuentes y cabeza histórica. Aplicar sobre las mismas
   fuentes detenidas y la misma base administrativa:

```bash
target/debug/despacho-cli --json database import \
  --data-dir /ruta/a/runtime-data --mapping /ruta/a/mapping.json --apply
```

6. Conservar fuentes y salida de reconciliación. El import añade un evento propio;
   `audit_entries` en el recibo cuenta solamente entradas históricas. Una repetición
   verifica el recibo y no duplica documentos ni eventos. Si el proceso termina
   después del commit y antes de escribir ambos marcadores, repetirlo recupera
   estos marcadores. Las barreras `.migration-pending` se escriben antes de
   insertar y conservan bloqueados ambos orígenes incluso si falla el commit.
   Un fallo posterior al commit lo indica explícitamente: reparar la causa de
   archivos y repetir las mismas fuentes y mapa. No borrar barreras para volver
   a escribir sobre un origen de estado incierto.
7. Arrancar con el rol operativo y `--data-dir` apuntando al origen preservado.
   El servidor comprueba ambos marcadores, hashes y recibo, y reconcilia documentos,
   asociaciones y cadena completa; rechaza restauraciones parciales. Validar
   roles, expedientes y exportaciones antes de habilitar tráfico.

Los marcadores `.migrated` no son una protección frente a ejecutables antiguos o
un administrador que los borre. No reiniciar escritores antiguos sobre el origen.
Después de empezar a servir, no reutilizar una base vacía ni volver al servidor
antiguo como recuperación: se perderían cambios posteriores al corte.

Cada archivo legacy representa un snapshot con su número original. Si contiene
la versión 7, la primera disponible será 7 y la siguiente 8; no se inventan las
versiones 1–6. La reconciliación compara el UUID y versión originales, conservando
bytes, recibo y prefijo de auditoría aunque se añadan versiones posteriormente.

## Actualizar programación de audiencias

Detener escritores, conservar el respaldo y ejecutar `database migrate
--runtime-role` con la conexión administrativa. Se aplica el conjunto
`0011_hearings.sql`, `0011_hearings_values.sql`, `0011_hearings_receipts.sql`
y `0011_hearings_guards.sql`. No ejecutar fragmentos por separado. Se añaden
`case_hearings` y `case_hearing_revisions`; no se generan citas retrospectivas.
El servidor operativo comprueba esquema, permisos e inventario sin ejecutar DDL.

Las revisiones conservan los bytes canónicos de valores y operación, sus
SHA-256, proyecciones y recibos. La secuencia y las fuentes de administración,
etapa, participantes y soporte pertenecen al mismo expediente. Cancelación
conserva el contexto original y registra la administración actual por separado.
El rol operativo puede leer e insertar; no actualizar, borrar ni truncar ese
historial. El UUID de operación es único entre las revisiones de programación y no se
reutiliza para reintentar una escritura de resultado incierto. La familia de
sesiones declaradas tiene su propia unicidad y su recibo HRTX1.

Respaldar las dos tablas junto con todas sus fuentes históricas y la auditoría.
Restaurar únicamente las citas actuales pierde recibos y referencias exactas.
El inventario resuelve cada revisión y sus fuentes antes de admitir el esquema;
un hash de valores válido no sustituye la consistencia entre tablas. Una
inconsistencia exige recuperar datos coherentes, sin fabricar revisiones ni
relajar restricciones. Administradores de PostgreSQL siguen dentro de la base
de confianza; el inventario no autentica definiciones SQL alteradas por ellos.

Las fechas preservan su desfase comunicado y la agenda consulta intervalos UTC.
Ni la restauración ni el paso del tiempo cambian automáticamente el estado de
una cita o crean plazos. Los metadatos de audiencias no usan el cifrado de los
archivos documentales; proteger sus respaldos. Véanse
[ADR-0028](adr/0028-audited-hearing-scheduling.md) y
[contrato HTTP de audiencias](hearings-api.md).

## Actualizar sesiones y resultados declarados de audiencia

La implementación y la recuperación fueron verificadas localmente; la integración remota permanece pendiente.
Con los escritores detenidos y respaldo completo, ejecutar `database migrate
--runtime-role` con la conexión administrativa y el nuevo binario. El conjunto
`0012_hearing_results*.sql` instala los decodificadores de tiempo, HRES1 y HRTX1,
las tablas `case_hearing_results` y `case_hearing_result_revisions` y sus guards.
No ejecutar archivos sueltos. Las citas existentes permanecen sin resultados
hasta una captura explícita; no se alteran HEAR1, HTXN1 ni la programación.

El rol operativo recibe SELECT/INSERT sobre ambas tablas y EXECUTE sobre las
funciones puras de proyección. No debe tener propiedad, UPDATE, DELETE,
TRUNCATE, TRIGGER, permisos por columna o roles heredados que permitan eludir
la inmutabilidad. El arranque verifica catálogo y permisos sin DDL y recorre
inventario, referencias exactas, secuencias y ciclos de continuidad. Incluye
raíces con UUID de valor cero; no se omiten por el cursor inicial. Los helpers SQL fijan
el esquema y conservan la fecha ISO independientemente de `DateStyle`.

La raíz fija audiencia, ancla exacta y continuidad opcional del mismo expediente.
Un antecedente debe existir antes de crear la nueva raíz; puede estar retirado.
Los asistentes pueden ser fichas históricas o archivadas. Rectificar conserva
fuentes fijas y vuelve a admitir el soporte; retirar conserva contenido y
admisión histórica y es terminal. Captura administrativa, reloj, autorización y
estado abierto se resuelven después del bloqueo común en READ COMMITTED.
La administración capturada no puede anteceder sus fuentes históricas exactas.
No resolver conflictos con UPDATE, borrar revisiones o desactivar guards.

Respaldar ambas tablas junto con programación, etapas, administración,
participantes manuales/tipificados, identidades, versiones documentales,
usuarios y auditoría. Comparar filas, cánones, recibos, proyecciones, autores e
instantes, además de referencias a anclas canceladas y antecedentes retirados.
Un registro de fecha o acuerdo no se transforma en plazo, alerta o resolución
por restaurar. Estos metadatos autorizados no usan el cifrado del archivo
DVLT1: proteger sus copias. Los administradores de PostgreSQL permanecen dentro
de la base de confianza; el inventario no autentica cuerpos SQL sustituidos por
ellos. Véanse [ADR-0029](adr/0029-declared-hearing-sessions.md) y
[contrato de resultados](hearing-results-api.md).

## Actualizar el catálogo de calendarios jurisdiccionales

La API y la restauración se verificaron localmente en servicios desechables.
La interfaz, la cobertura y el cierre de CI siguen pendientes. Detener escritores
y conservar un respaldo completo antes de ejecutar
`database migrate --runtime-role` con el nuevo binario y una conexión
administrativa. El conjunto `0013_judicial_calendar_*.sql` instala primitivas,
fuentes, valores, recibos, tablas y guards en ese orden. No ejecutar archivos
sueltos. Añade `judicial_calendars` y `judicial_calendar_revisions`; no crea
calendarios iniciales ni modifica perfiles, citas o resultados existentes.

La raíz fija su primera revisión mediante una clave foránea diferida. JCAL1
conserva el ámbito, cobertura, referencias y reglas; JCTX1 vincula actor,
operación, raíz, revisión esperada, digest y motivo. Los SHA-256 se comprueban
sobre los bytes canónicos y las proyecciones SQL son columnas generadas.
La secuencia exige sucesor exacto, ámbito idéntico a R1 y retiro terminal con
copia íntegra de los valores anteriores. UUID de valor cero está permitido;
el UUID de operación es único dentro de esta familia, incluidas otras raíces.

El rol operativo necesita SELECT/INSERT sobre ambas tablas y EXECUTE sobre
los seis helpers puros de URL, fecha, fuente, regla, valores y recibo. No debe
poseer tablas o funciones, ejecutar guards ni tener UPDATE, DELETE, TRUNCATE,
TRIGGER o privilegios heredados que permitan reescribir la historia. Los guards
usan READ COMMITTED y el bloqueo común de auditoría; comprueban Owner activo y
su correo capturado después del bloqueo. La confirmación y su evento pertenecen
a la misma transacción. Conexiones operativas comprueban el esquema y su
inventario sin ejecutar DDL; una inconsistencia no se repara relajando controles.

Las fechas civiles y sus proyecciones ISO no dependen del `DateStyle` de la
sesión ni de una zona horaria. Los helpers fijan `pg_catalog` y califican las
dependencias del esquema para restaurar con `search_path` vacío. Las referencias
son texto autorizado, sin copia descargada ni evidencia normativa cifrada.
Proteger estos metadatos y sus respaldos; retirar un calendario no deroga una
norma ni elimina sus fuentes históricas.

Respaldar ambas tablas con usuarios y auditoría, además de los demás módulos.
Restaurar raíces y todas sus revisiones, conservar bytes JCAL1/JCTX1, hashes,
proyecciones, autor y captura, y comparar filas completas. El proceso no debe
consultar URLs ni regenerar referencias. La importación inicial del almacenamiento
legacy rechaza destinos con filas en `judicial_calendars` o
`judicial_calendar_revisions`, para preservar el prefijo original de auditoría.
La conciliación de un recibo de importación existente permite conservar los
calendarios creados posteriormente.

El recorrido `scripts/api-demo.sh` incorpora los helpers
`scripts/api-judicial-calendars-demo.py` y `.sh`. La campaña integrada terminó
correctamente en 210.700 s: dos raíces y cuatro revisiones de calendario, una
con los valores Unicode máximos, permisos sin asignación, denegación de Client,
conflicto entre Owners, ámbito R1, recibos, días exactos e historia. Tras
restaurar comparó diez respuestas completas y las filas de ambas tablas.
Estas diez respuestas se suman a las 21 de programación y resultados de
audiencia, conservadas por el mismo recorrido. Los resultados y el alcance de
las demás campañas se distinguen en el [informe de verificación](verification-report.md).

El catálogo no programa tareas vacías de reevaluación ni activa plazos o
alertas. La evaluación futura requerirá hechos, reglas aplicables y coordinación
transaccional con la revisión del calendario. Véanse
[el contrato](judicial-calendars-api.md) y
[ADR-0030](adr/0030-versioned-jurisdictional-calendars.md).

## Respaldo y restauración

Las migraciones `0009_participant_credential_trust.sql` y el conjunto `0010_`
añaden confianza, identidades, perfiles tipificados, revisiones de selección y
evidencia de declaración. Ejecutar `database migrate --runtime-role` con los
escritores detenidos y una conexión administrativa. El requisito de primera
revisión de participante pasa a un trigger diferido que admite una única R1
manual o tipificada; no se eliminan ni convierten las revisiones manuales.
El arranque verifica columnas, restricciones y triggers, sus permisos y el
inventario completo, incluidas las relaciones entre pruebas firmadas y fichas.
Una discrepancia exige restaurar datos consistentes, no relajar esos controles.

Las tablas nuevas son `case_subjects`, `case_subject_revisions`,
`case_participant_typed_revisions`, `subject_identity_reviews`,
`participant_identity_reviews` y `participant_credential_evidence`, además de
`participant_credential_authority` y `participant_credential_trust_revisions`.
Conservarlas junto con usuarios, expedientes, documentos y auditoría. El rol
operativo puede consultar la confianza publicada, pero no publicarla ni alterar
la autoridad. No conceder propiedad o permisos indirectos que eludan esa separación.

La confianza de declaraciones de participantes se publica con conexión
administrativa mediante `credential-trust publish`; consulte
[el procedimiento PKI](../pki/README.md#publicar-confianza-para-declaraciones).
El rol operativo conserva sólo lectura sobre `participant_credential_authority`
y `participant_credential_trust_revisions`. No concederle escritura, propiedad
de las tablas ni ejecución de las funciones que protegen ese historial.
El respaldo debe incluir ambas tablas y preservar el UUID de despliegue, los
bytes públicos de raíz y CRL, sus revisiones y la procedencia administrativa.
Crear una autoridad nueva no sustituye una restauración de la original.

Respaldar PostgreSQL completo, conservar las fuentes originales y proteger KEK,
claves y certificados por separado. Una copia documental sin su KEK no basta.
Crear una base de restauración independiente antes de probar recuperación:

```bash
pg_dump "$DATABASE_URL" --format=custom --file=/ruta/segura/despacho.dump
pg_restore --dbname="$RESTORE_DATABASE_URL" --exit-on-error /ruta/segura/despacho.dump
```

Restaurar también roles/permisos según el procedimiento administrativo del entorno.
Comprobar conteos, contexto y bytes cifrados, evidencia sellada, secuencias y hashes
de auditoría y recibos. Verificar ZIP con OpenSSL y comparar con la exportación
anterior. `scripts/api-demo.sh` incluye un ensayo desechable de importación y
restauración con documentos realmente sellados. Después de importar añade una
segunda versión, conserva el ZIP de la primera, restaura ambas y compara sus
exportaciones. También carga un documento con clasificación inicial, reemplaza
y vacía los valores de otro, comprueba conflictos y filtros actuales, y compara
todas las revisiones, autores capturados y ambas evidencias tras restaurar.
El mismo ensayo crea participantes, comprueba conflicto concurrente, permisos,
archivo/reactivación y revocación; restaura sus raíces, historia, valores y
procedencia junto con el resto de la base. Añade altas penales, completa perfiles
pendientes, comprueba conflictos y cierre, y restaura las revisiones
administrativas y los registros iniciales con su procedencia exacta. El recorrido
de etapas añade adopción y ambos avances con PDF/DOCX, conflicto concurrente,
soporte histórico después de append, cierre y revocación; al restaurar compara
estado actual, historia, fechas, actores y evidencia ZIP original.
El recorrido tipificado crea una identidad y dos roles, prepara una declaración
de 218 bytes, la firma externamente con OpenSSL y comprueba el rechazo de una
firma alterada. Edita la identidad y archiva el rol firmado preservando sus
referencias originales. Después del respaldo compara diez respuestas completas,
las tablas nuevas y la confianza capturada; verifica de nuevo la firma con el
certificado público recuperado. La clave privada de esa fixture permanece fuera
del directorio de trabajo del servidor y no se envía por HTTP.
El recorrido de audiencias programa, reemplaza y cancela, retiene participantes
históricos y soportes sellados exactos, y consulta una agenda autorizada.
El guion `scripts/api_hearing_results_demo.py` amplía el recorrido con sesiones,
rectificación, retiro, historia, soportes y continuidad exactos. Después de
restaurar compara 21 respuestas completas: doce de la programación y su contexto,
y nueve de resultados. También compara todas las filas de `case_hearings`,
`case_hearing_revisions`, `case_hearing_results` y
`case_hearing_result_revisions`, incluidos recibos, autores, contexto y
referencias. Conserva valores HRES1, recibos HRTX1, fuentes y captura junto con
el inventario anterior; no valida una restauración únicamente por el total de
citas. Las pruebas PostgreSQL
incluyen una continuación vinculada a un antecedente exacto retirado y una nueva
rectificación autorizada después de restaurar y reabrir el expediente.
La reconstrucción del formato legacy es un fixture documental, no una conversión
íntegra del historial administrativo actual; la restauración posterior sí conserva
todas las tablas.
No utiliza datos del usuario. Incluir siempre raíces, todas las versiones,
`document_metadata_revisions`, `case_participants`, `case_participant_revisions`,
`case_administration_revisions`, `case_initial_stage_registrations`,
`case_stage_revisions`, `case_hearings`, `case_hearing_revisions`,
`case_hearing_results`, `case_hearing_result_revisions`, `judicial_calendars`,
`judicial_calendar_revisions` y auditoría; un respaldo incompleto no se repara creando
raíces o revisiones falsas. Comparar las filas completas y hashes, no
solo sus conteos.

Las sesiones Redis no sustituyen el estado durable. En una recuperación operativa
se deben invalidar sesiones anteriores y ensayar el nuevo acceso con MFA. El
respaldo y los recibos no resuelven por sí solos el anclaje externo de auditoría.
