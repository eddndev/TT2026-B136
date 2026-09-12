# ADR-0016: Documentos por expediente y auditoría transaccional

## Status

Accepted.

## Context

El servidor persistía usuarios y expedientes en PostgreSQL, documentos cifrados
como JSON y eventos en un archivo separado. Un fallo entre escrituras podía
confirmar un cambio sin su evento. La autorización documental dependía del rol
global aplicado por HTTP y no comprobaba el expediente del recurso.

Los registros existentes contienen UUID, versión, contexto de cifrado, firma,
sello y certificados capturados. Cambiar ese material al migrar impediría
comparar la evidencia anterior. La bitácora usa un formato canónico con marcas
de tiempo que pueden incluir nanosegundos.

## Decision

### Frontera de aplicación

`CaseDocumentService` recibe sesión, expediente e identidad documental. Comprueba
el permiso antes de preparar contenido y vuelve a autenticar antes de confirmar.
`CaseDocumentStore` revalida usuario activo, rol, pertenencia y asociación exacta
en la transacción. Owner mantiene acceso a todos los expedientes; Litigator y
Paralegal requieren asignación vigente. Paralegal no sella y Client conserva
denegadas todas las operaciones documentales, incluso asignado.

Las rutas documentales requieren `/api/v1/cases/{case_id}/documents`. Se retiran
las rutas globales. Un documento ajeno al expediente se responde como ausente,
incluso para Owner. Verificación y exportación confirman su evento y una última
comprobación de acceso antes de devolver el resultado. Una respuesta confirmada
antes de una revocación puede terminar de transmitirse después de ella.

`DocumentProcessor` comparte la preparación criptográfica entre el workflow
actual y las pruebas del workflow local. AES, RSA, ZIP y TSA se ejecutan fuera
de la transacción. El commit no reintenta automáticamente una llamada externa.
Dos preparaciones de sellado pueden competir; solamente una persiste evidencia.
La que pierde recibe conflicto y no reemplaza la ganadora. La versión actual
no ofrece actualización del contenido ni reasignación de expediente.

### Persistencia y orden

`migrations/0003_case_documents_audit.sql` añade documentos, eventos y recibos de
importación. Toda transacción mutante del backend toma primero el mismo bloqueo
consultivo de transacción: documentos, expedientes, asignaciones, usuarios y
anexado de eventos de identidad. Después revalida y bloquea las filas de usuario
y pertenencia necesarias. La revocación que se confirma primero impide un
commit posterior; si la operación obtiene primero el bloqueo, la revocación
espera. Los cambios directos de cuenta y pertenencia también deben respetar
los bloqueos de fila de PostgreSQL.

El alta inicial, creación de usuarios y consumo de recuperación guardan su evento
con su mutación. Los eventos sin otra escritura durable usan la misma cadena
PostgreSQL. Redis conserva sesiones, desafíos y controles efímeros; no forma una
transacción distribuida con PostgreSQL. Los fallos de auditoría intentan retirar
el desafío o sesión recién creado y nunca devuelven su token. Una revocación ya
realizada no se deshace por un fallo posterior de auditoría. Si Redis falla también
durante la compensación, la expiración es el límite de limpieza de esa clave.

La secuencia y el hash se calculan usando el dominio existente. El timestamp se
guarda como texto canónico para conservar precisión, y las consultas verifican
la cadena desde su génesis. Un trigger y permisos por columna hacen inmutables
UUID, expediente, versión, nombre, digest, vault y evidencia ya sellada.

### Administración y rol operativo

`database migrate --runtime-role` aplica DDL con credenciales administrativas y
otorga permisos a un rol existente. `serve` usa conexiones sin DDL y rechaza
roles que puedan administrar o reescribir la bitácora, documentos, recibos o
la función que protege evidencia. También rechaza esquemas activos modificables
por ese rol. El rol operativo puede
leer y anexar eventos, pero carece de UPDATE, DELETE y TRUNCATE sobre ellos.
Los propietarios y administradores siguen siendo parte de la base de confianza.

La elección se apoya en las reglas de
[bloqueos de PostgreSQL](https://www.postgresql.org/docs/16/explicit-locking.html)
y [privilegios de PostgreSQL](https://www.postgresql.org/docs/16/ddl-priv.html).
Los bloqueos consultivos solamente coordinan participantes que los adquieren;
los permisos del propietario no se eliminan simplemente revocando un GRANT.

### Corte y migración

La importación se realiza con escritores detenidos y un mapa explícito completo
de documento a expediente ya existente. Por defecto inspecciona archivos y destino
sin escrituras. Descifra para comprobar digest y contexto, valida firma y sello
capturados, y rechaza asociación ausente, duplicados, archivos inesperados,
identidades incoherentes, historia rota y referencias documentales huérfanas.
Evalúa la cadena y CRL del firmante en el instante del sello y rechaza estados
inválidos en ese instante. El verificador TSA conserva su política OpenSSL
actual: una TSA expirada hoy puede impedir importar, aunque el material quede
preservado para resolución manual.

El primer import requiere documentos y auditoría de destino vacíos. Conserva
cada entrada histórica y sus hashes; añade un evento con la huella del mapa y
fuentes, y un recibo, en la misma transacción que inserta documentos. Una segunda
ejecución reconcilia contenido inmutable, evidencia preexistente y cadena completa.
Nunca antepone historia a eventos nuevos ni reemite firmas o sellos.

La operación bloquea los escritores de archivos y comprueba que las fuentes no
hayan cambiado desde la inspección. Antes de insertar crea barreras durables
`.migration-pending` para ambos almacenes; permanecen si hay un fallo. Tras el
commit instala ambos marcadores finales y retira las barreras pendientes. Un
fallo de marcadores posterior al commit se informa como importación confirmada,
con fuentes todavía bloqueadas; repetir la misma importación reconcilia el
recibo y completa el corte. Versiones antiguas del ejecutable
no conocen esos marcadores: mantenerlas detenidas es condición del corte.
`serve` rechaza fuentes locales pendientes o alteradas y reconcilia recibo,
asociaciones, documentos y cadena completa en una lectura consistente de la
base. Un recibo solo no basta para aceptar una restauración parcial. El procedimiento y ensayo de restauración están en
`docs/database-operations.md`.

## Consequences

Las mutaciones documentales y de asignaciones ahora comparten confirmación con
su historia; los resultados no publicados por un fallo pueden prepararse de
nuevo sin cambiar evidencia ya confirmada. El bloqueo global simplifica el orden
de la bitácora a costa de serializar commits breves; debe medirse antes de aumentar
carga. Las operaciones caras no retienen ese bloqueo.

Se conserva el contexto AES existente (UUID y versión). La pertenencia al
expediente se protege con la asociación inmutable y comprobaciones transaccionales;
no se afirma que el vault antiguo incluya criptográficamente el expediente.

Siguen pendientes anclaje externo de la cabeza, TLS, pooling, plazos operativos,
pruebas de carga, apagado ordenado y la revisión de firma RSA expuesta por HTTP.
Una copia válida de la cadena tampoco prueba que no se haya truncado su extremo;
conservar y comparar recibos y respaldos ayuda a reconciliar, pero no reemplaza
un anclaje independiente.
