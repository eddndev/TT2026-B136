# API de calendarios jurisdiccionales por revision

Estado: implementacion en curso; dominio y aplicacion verificados localmente.
Persistencia, HTTP, Qadra y cierre integrado pendientes de verificacion.
Decision: [ADR-0030](adr/0030-versioned-jurisdictional-calendars.md).

## Alcance

Catalogo global del despacho para clasificar fechas civiles segun un ambito
expresamente declarado. Owner publica, reemplaza y retira; Owner, Litigator y
Paralegal consultan. Client queda denegado. No exige asignacion a expedientes y
no recibe sus documentos, participantes, audiencias ni identificadores.

Cada revision conserva cobertura, patron semanal, excepciones y referencias
publicas declaradas. No calcula vencimientos, no interpreta relatos de audiencia,
no acredita autenticidad normativa y no envia avisos. El calendario completo con
plazos, reevaluacion y alertas sigue pendiente. La configuracion versionada es un
insumo para ese trabajo; no sustituye los criterios aprobados del manuscrito.

## Valores

Todos los UUID, incluido cero, son validos; rechazar duplicados en su coleccion.
Revision u32 positiva, inicial 1 y sucesora exacta sin wrap. Operacion UUID unica
en toda esta familia. No prometer unicidad con operaciones de otros modulos.

Texto: recortar espacio exterior, normalizar CRLF a LF, rechazar controles excepto
LF en campos multilineales. Cotas por escalares Unicode; canon por bytes UTF-8.
No normalizar Unicode interior. El JSON y codigo fuente conservan sus politicas
existentes; Rust puede representar texto Unicode aunque las fuentes sean ASCII.

| Campo | Restriccion |
| --- | --- |
| `scope.title` | Texto requerido 1..200, una linea. |
| `scope.jurisdiction` | `federal` o `local`, eleccion expresa. |
| `scope.entity_codes` | 1..32 strings de dos digitos ASCII, `01`..`32`; unicos y ordenados. No corregir espacios, numeros, nombres o claves mal formadas. |
| `scope.authority`, `scope.organ`, `scope.territory` | Cada uno requerido 1..200, una linea. |
| `scope.use_description` | Requerido 1..1000, multilineal. |
| `coverage.from`, `coverage.through` | Fechas civiles inclusivas; cobertura de 1..1096 dias. |
| `sources` | 0..16 referencias, UUID unico y orden canonico por bytes UUID. |
| `sources[].id` | UUID. |
| `sources[].title`, `sources[].issuer` | Cada uno requerido 1..200, una linea. |
| `sources[].official_url` | HTTPS absoluto, ASCII de 1..2048 bytes; host no vacio, sin userinfo, espacios, controles ni barra inversa. Validar estructura, conservar texto recortado; no descargar ni afirmar oficialidad. |
| `sources[].published_on` | Fecha civil opcional; ausente/null normaliza a null. Si existe, no posterior a `consulted_on`. |
| `sources[].consulted_on` | Fecha civil declarada requerida; no se convierte en hora ni se equipara con captura del servidor. |
| `sources[].locator` | Texto requerido 1..512, una linea. |
| `weekly_pattern` | Exactamente siete entradas `{weekday,classification,source_ids,explanation}`; weekday entero 1=lunes ..7=domingo, una vez cada uno. |
| `classification` | `countable`, `excluded` o `unresolved`; seleccion expresa. |
| `source_ids` | 0..16 UUID unicos, presentes en sources de ESTA revision, ordenados. Countable/excluded requieren al menos uno. |
| `explanation` | Requerida 1..256, multilineal, para cualquier clasificacion. |
| `exceptions` | 0..64 objetos `{id,from,through,classification,source_ids,explanation}`. Intervalos inclusivos dentro de cobertura; IDs unicos; sin solapamiento; ordenar `(from,through,id)`. |

El ambito completo queda fijo al publicar R1. No resolver aplicabilidad por
comparacion de nombres. Cambiar sustancialmente el ambito requiere otra raiz;
reemplazar conserva el ambito y publica valores completos nuevos. Titulos
iguales en raices distintas estan permitidos y se distinguen por identidad.

Los codigos de entidad se fijan conforme al catalogo INEGI 2025, claves `01`..`32`:
[publicacion oficial, apartado 3.1](https://www.inegi.org.mx/contenidos/productos/prod_serv/contenidos/espanol/bvinegi/productos/nueva_estruc/889463924036.pdf)
y [definicion del servicio](https://www.inegi.org.mx/servicios/catalogounico.html).
Un subconjunto o las 32 claves se permiten para ambos fueros. La seleccion de las
32 debe ser explicita; no hay sentinel nacional ni entidad predeterminada.
Los codigos geograficos no acreditan competencia de una autoridad judicial.

Fechas exactas `YYYY-MM-DD`, gregorianas, anos 1..9999; no admitir hora, offset,
zona ni fechas normalizadas desde dias invalidos. No comparar cobertura futura
con el reloj de captura. Las fechas declaradas de referencia no son evidencia de
una consulta de red. Horas, IANA/tzdb y reglas de vencimiento se fijaran en el
contrato de evaluacion cuando exista esa operacion.

Cada excepcion aplica al ambito completo y sustituye el patron semanal en su
intervalo. Rechazar solapamientos; no usar orden de filas ni ultimo gana. Un
supuesto parcial, condicionado u horario no representable se registra como
`unresolved` con explicacion, o en otro ambito que pueda describirse fielmente.
No inferir disponibilidades laborales, guardias o notificaciones de estos datos.

## Clasificacion de fechas

Funcion pura sobre una revision exacta y una fecha:

1. Fuera de cobertura: `outside_coverage`, sin regla ni fuentes aplicadas.
2. Dentro de una excepcion: usar esa regla completa.
3. En otro caso: usar la entrada semanal expresa del dia correspondiente.

La salida incluye fecha, estado, origen `weekly_pattern` o `exception`, weekday
o exception_id segun origen, explicacion y source_ids. Una fecha fuera de
cobertura tiene origen, explicacion y referencias ausentes/vacias segun DTO.
Una referencia ausente en estado almacenado es inconsistencia, no fallback.
`unresolved` no equivale a `excluded`; countable significa clasificacion declarada
para ese ambito, no certificacion universal de dia habil.

Una revision retirada sigue siendo consultable por identidad exacta. La consulta
no promete que esa revision sea seleccionable para nuevos plazos. Ningun lookup
resuelve fuentes remotas, cambia segun zona del navegador o usa la cabecera actual
cuando se solicito una revision historica.

### Perfil acotado de URL

La referencia admite el esquema literal `https://` y un host DNS ASCII de al
menos dos etiquetas. Cada etiqueta tiene1..63 caracteres, extremos alfanumericos
y guion solo interior; host completo hasta253 bytes y ultima etiqueta de2..63
letras ASCII. Rechazar etiquetas que comiencen `xn--`, sin distinguir mayusculas.
No admitir IP, IPv6, puerto, userinfo ni autoridad con escapes. Es un perfil de
referencias de este catalogo, no un parser universal de URL/IRI.

Despues del host puede terminar la URL o comenzar `/`, `?` o `#`. El resto solo
admite alfanumericos ASCII, `-._~!$&'()*+,;=:@/?#` y secuencias `%HH` validas.
Como maximo un `#`; el signo `?` puede aparecer en query o fragmento. No admitir
comillas, llaves, corchetes, espacios, controles o barra inversa. Conservar el
texto recortado, sin normalizar host, rutas ni escapes; no realizar consultas DNS.
Este perfil permite validacion identica en dominio y SQL sin convertir metadatos
automaticamente ni dar por verificada la naturaleza publica de un enlace.

## Fuentes publicas declaradas

Las referencias son metadatos visibles al personal autorizado del despacho.
La URL no se visita en servidor ni durante una consulta de calendario. Se abre
en navegador solo por accion explicita, con texto escapado y sin acceso al opener.
No admitir soporte de expediente ni SHA aportado como si se hubieran archivado
bytes de una disposicion. El digest de valores cubre la referencia declarada,
no el contenido remoto, su autenticidad ni su aplicabilidad.

La interfaz indica que la copia no esta archivada. Una biblioteca futura de
fuentes preservadas necesitara autorizacion global, formato/admisibilidad,
contenido inmutable, digest calculado, auditoria y restauracion propios. No
reutilizar permisos ni datos privados de expedientes para ese catalogo.

## Comandos y recibos

```json
{
  "operation_id":"00000000-0000-4000-8000-000000000101",
  "calendar_id":"00000000-0000-4000-8000-000000000102",
  "change":{"action":"publish","expected_revision":0,"values":{}}
}
```

El ejemplo muestra la envoltura; `values` requiere todos los campos anteriores.
Variantes estrictas:

- `publish`: action, expected_revision=0 y values completos.
- `replace`: action, expected_revision positiva, values completos y reason 1..1000
  multilineal. Scope debe ser identico al de R1, incluidos titulo y descripciones.
- `retire`: action, expected_revision positiva y reason; sin values.

Publicar/reemplazar producen `published`; retirar produce `retired`, terminal,
copiando exactamente valores y fuentes de la base. No borrar, reactivar ni
reinterpretar el retiro como derogacion de una norma. Contenido igual y nueva
operacion siguen siendo otra revision explicita; no deduplicar por texto.

Preparar no reserva UUID, revision ni estado servidor. Devuelve actor_id, comando
normalizado, result_revision, values/values_digest, scope inicial informativo y
submission_digest. Actor procede de sesion, nunca del comando editable.
Confirmar recibe `{command,expected_submission_digest}` y recalcula el recibo.
Reautenticar despues del trabajo preparatorio, antes de entrar a commit.

El recibo vincula actor, operacion, raiz, accion, revision esperada, digest de
valores y motivo. Captura real y autor/email se registran despues del bloqueo
transaccional; no forman parte del digest solicitado como reloj futuro.
Conciliar envio incierto mediante revision exacta y todas las identidades del
recibo, sin reenviar automaticamente. Un 404 especifico de revision objetivo
permite seguir consultando; denegacion o sesion caducada se propagan.

## Canon de valores JCAL1

Enteros big-endian; UUID 16 bytes; texto u32 de longitud UTF-8 seguido de bytes.
Fecha civil i32: diferencia en dias desde 1970-01-01, sin conversion de zona.
No almacenar un JSON serializado arbitrariamente como representacion canonica.

Orden fijo:

1. ASCII JCAL1.
2. Scope: title; jurisdiction u8 (0 federal, 1 local); count u8 de entidades y
   cada codigo como u8 1..32; authority; organ; territory; use_description.
3. Cobertura from/through como fechas.
4. Cantidad u8 de fuentes; para cada fuente ordenada: UUID, title, issuer,
   official_url, opcion u8 de published_on (0 ausente,1 presente mas fecha),
   consulted_on y locator.
5. Siete reglas semanales, cada una: weekday u8 y regla.
6. Cantidad u8 de excepciones; cada una: UUID, from, through y regla.

Regla: classification u8 (0 countable,1 excluded,2 unresolved), count u8 de
referencias y UUID ordenados; explanation como texto. No hay count para las
siete reglas semanales. Valores fuera de rango o bytes posteriores se rechazan.
Siete vectores independientes versionados en
`crates/domain/tests/fixtures/judicial_calendar_vectors.json` fijan el minimo de
99 y el maximo de 191910 bytes. Su generador Python reproduce los bytes y
proyecciones; el dominio Rust coincide con los siete vectores.

Canon JCTX1 de envio: ASCII JCTX1, actor UUID, operation UUID, calendar UUID,
action u8 (0 publish,1 replace,2 retire), expected_revision u32, values_digest
32 bytes, opcion u8 reason (0 ausente,1 presente y texto). Publicacion carece
de motivo; reemplazo/retiro lo requieren. La revision resultante es expected+1,
verificada sin overflow. JCTX1 tiene minimo 91 bytes y maximo 4095 bytes.

## HTTP y consultas

Prefijo `/api/v1/judicial-calendars`, bearer obligatorio y presupuesto compartido.

| Metodo | Sufijo | Resultado |
| --- | --- | --- |
| POST | `/prepare` | Preparacion 200 |
| POST | vacio | Publicacion 201 |
| GET | vacio | Raices paginadas |
| GET | `/{id}` | Cabeza actual |
| PUT | `/{id}` | Reemplazo 201 |
| POST | `/{id}/retirement` | Retiro 201 |
| GET | `/{id}/history` | Resumenes descendentes |
| GET | `/{id}/revisions/{revision}` | Detalle exacto |
| GET | `/{id}/revisions/{revision}/days` | Clasificacion exacta de rango |

Lista: status=all|published|retired (por defecto published), jurisdiction opcional
federal|local, entity_code opcional01..32, limit1..100 (20), after_id exclusivo.
Resolver cabezas y permisos ANTES de filtrar/paginar. Sin busqueda libre inicial.
Respuesta calendars,has_more,next_after_id; resumen incluye id,revision,status,
scope,coverage,values_digest y un indicador de reglas unresolved (no conteo
inventado de plazos afectados ni garantia de cobertura normativa completa).

Historia: limit1..20 (10), before_revision positiva exclusiva; revisions,
has_more,next_before_revision. Resumen con identidad/revision/estado/motivo,
values_digest,recibo,autor/instante; no multiplicar fuentes/excepciones por fila.
Detalle incluye valores completos y esa metadata. Days requiere from/through
inclusivos, rango1..62 dias; respuesta calendar_id,revision,values_digest,days.
No consulta por celda. Puede incluir fechas fuera de cobertura con estado propio.
No fabricar ano10000 para consultar 9999-12-31.

Cuerpos JSON limitados a 1MiB=1048576 bytes, incluida envoltura/espacios. Esta es
una cota de transporte separada de la cardinalidad de valores. Probar el comando
maximo con Unicode literal y escapado; no prometer aceptar espacios ilimitados.
El comando maximo compacto de los vectores mide 232723 bytes con UTF-8 literal
y 517267 bytes escapando Unicode a ASCII, ambos menores que el limite.
Objetos, enums, campos, enteros y queries estrictos; rechazar claves duplicadas,
queries desconocidas y cuerpos posicionales. En rutas sin consulta, solo query
ausente/vacia es valida. Seleccion historica exclusivamente en ruta.

Errores estructurados: consulta invalida400 `invalid_query`, sesion invalida401,
permiso403; raiz/revision ausente404
`judicial_calendar_not_found`; datos invalidos422; conflicto de revision,
operacion o retiro previo409; contador agotado409 con codigo propio. Fuentes
pertenecen al valor propuesto y no producen404 de otro recurso. Scope distinto
en replace causa422, nunca crea otra raiz implicitamente. Corrupcion almacenada
se informa como error interno sin devolver una proyeccion parcialmente validada.

## Transaccion y recuperacion

Dos tablas: judicial_calendars y judicial_calendar_revisions. Raiz fija R1
mediante FK diferida; revision tiene PK(raiz,revision), operacion unica, canon y
proyeccion derivada, recibo, actor/correo/instante. Scope de toda revision debe
coincidir exactamente con R1. Inmutabilidad y grants del rol operativo siguen
las protecciones actuales; solo SELECT/INSERT y ejecucion de helpers puros.

Commit adquiere bloqueo comun de auditoria y revalida cuenta activa/rol Owner,
base, operacion, scope inmutable y retiro terminal. Captura Clock entonces e
inserta raiz/revision/evento atomicamente. Lecturas son autorizadas y auditadas;
no se usan IDs de caso para filtrar el catalogo global. Inicio comprueba catalogo,
permisos, canon/digests, proyecciones, secuencia, recibos y scope desde R1.
El inventario incluye UUID cero, rechaza filas/campos incompatibles y no ejecuta DDL.

Respaldo/restauracion conserva raices y todas sus revisiones con auditoria,
valores, metadatos declarados y recibos. No depende de disponibilidad de URL.
No introducir trabajos de reevaluacion sin plazos existentes. Antes de integrar
plazos, la publicacion debe crear una intencion durable en la misma transaccion;
la evaluacion revalida la cabeza elegible al confirmar y el worker usa CAS.
Asi se evitara que un plazo creado mientras se publica un calendario quede fuera
de la reevaluacion. Ese flujo y sus alertas no estan implementados por este contrato.

## Aceptacion y Qadra

TDD: calendario civil/bisiesto/extremos, claves de entidad, limites/textos/URL,
normalizacion, referencias, patron completo, intervalos y clasificacion sin
fallback. Vectores independientes minimos/maximos y malformed para JCAL1/JCTX1.
Aplicacion/PG: permisos antes/despues del lock, conflicto entre Owners, operacion
compartida entre raices, retiro terminal, scope inmutable, rollback y restore.
HTTP: limites byte exactos, Unicode maximo, entradas estrictas, paginacion sobre
cabezas, Client denegado y ausencia de filtracion de datos de expedientes.

Qadra: acceso desde Agenda y Administracion, ruta global judicial-calendars,
lista y filtros expresos; formulario Owner con fuentes, ambito, patron, excepciones
y revision previa a publicar. Mes/lista accesible, leyenda textual y detalle de
dia/fuentes. Historia exacta, borrador conservado ante conflicto, conciliacion
sin reenvio, sesion/seleccion tardias descartadas. No depender de selectedCase.
Conservar marca/tokens/componentes; prueba real de permisos, conflicto, recibo,
recuperacion y escritorio/movil. Estas son pruebas requeridas, aun no ejecutadas.
