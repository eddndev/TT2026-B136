# Codificacion de fuentes y recibos de hechos declarados

## Estado y frontera

Este documento fija los canones de fuentes `PFSRC1` y operaciones `PFTXN1` de
[hechos declarados](procedural-facts-application.md). El contraste de vectores y
las comprobaciones del servicio se registran por separado en el informe de
verificacion. No constituye evidencia de un adaptador persistente ni de una
operacion confirmada por HTTP. Los canones de valores
[PFRES1 y PFNOT1](procedural-facts-canonical.md) permanecen independientes.

`fact_sources_bytes(&FactSources)` valida la forma de la coleccion y devuelve
sus bytes o `ProceduralFactError::StoredInconsistent`. `fact_sources_digest`
valida los mismos datos y solicita a `DocumentHasher` su SHA-256. La comprobacion
estructural no acredita existencia, autorizacion, pertenencia al expediente del
comando ni derivacion desde fuentes autenticas. Esas comprobaciones corresponden
al servicio y al adaptador, conforme a [ADR-0031](adr/0031-declared-procedural-facts.md).

## PFSRC1: primitivas y orden general

- `UUID`: 16 bytes originales, incluido UUID cero.
- `U32`: entero de cuatro bytes sin signo, orden big-endian.
- `TEXT`: longitud en bytes UTF-8 como U32, seguida de esos bytes; no terminador.
- `OPTION(X)`: byte 0 para ausencia; byte 1 seguido de X para presencia.
- `DIGEST`: 32 bytes de SHA-256, no texto hexadecimal.
- `STATUS`: byte 0 para `Recorded` o `Active`, segun la familia; byte 1 para
  `Withdrawn` o `Archived`. No convierte un estado historico en permiso vigente.

El orden completo es:

```text
"PFSRC1" (6 bytes)
OPTION(resolution)
U32(participants.len) + participants
U32(hearing_results.len) + hearing_results
U32(direct_supports.len) + direct_supports
```

La coleccion vacia ocupa 19 bytes; el maximo estructural es 36 847 bytes, con
texto UTF-8 de hasta cuatro bytes por escalar en los campos que lo admiten.
Se admite como maximo una resolucion, cuatro
participantes, dos selecciones de resultados y dos documentos directos. Las
listas deben estar estrictamente ordenadas; se rechazan tanto duplicados como
orden inverso, sin ordenar ni normalizar silenciosamente los datos recibidos:

| Lista | Clave exacta ascendente |
| --- | --- |
| Participantes | UUID de ficha, revision |
| Resultados | UUID de audiencia, UUID de resultado, revision, acuerdo opcional |
| Documentos | UUID documental, version |

En las claves opcionales, ausencia precede a presencia. `None` y `Some(UUID cero)`
son distintos. Dos acuerdos de una revision pueden producir dos selecciones;
sus campos comunes (expediente, digests, estado, ocurrencia, tiempo y resumen)
deben coincidir, tambien al conservar el resultado y cambiar el acuerdo durante
una correccion. Distintas revisiones y versiones nunca se fusionan.

Antes de codificar, las vistas tienen que corresponder uno a uno con sus
referencias compactas, en el mismo orden. El canon vincula los campos legibles
incluidos expresamente; no acepta una vista independiente con otra identidad.

## Resolucion

```text
case UUID, resolution UUID, revision U32
values DIGEST, submission DIGEST, STATUS
class declaration, issuer declaration, issued_at, summary TEXT
```

La vista debe seleccionar exactamente el mismo UUID y revision. Su clase usa
byte 0 mas `TEXT(reason)` para `Unknown`, o byte 1 mas la variante conocida:
0 `Order`, 1 `Judgment`, 2 `Other` seguido de `TEXT(label)`.
El emisor usa byte 0 mas `TEXT(reason)` o byte 1 mas `TEXT(label)`.

`issued_at` conserva exactamente la codificacion temporal de PFRES1: 0 Unknown,
1 Date, 2 Minute o 3 Second; fecha local como anio U16 big-endian, mes y dia de
un byte; hora/minuto y segundo solo cuando su precision los declara. Al final
de un tiempo conocido se escribe `OPTION(I32(offset_seconds))`, con signo y
big-endian. Ausencia de desfase no equivale a UTC. Unknown no agrega componentes.
No se fabrica un instante ni se redondea precision.

## Participante

```text
case UUID, participant UUID, revision U32, values DIGEST, STATUS
OPTION(subject UUID + revision U32 + values DIGEST)
display_name TEXT, procedural_role TEXT, OPTION(organization TEXT)
OPTION(kind byte)
```

Expediente, UUID, revision, estado y sujeto exacto deben coincidir entre snapshot
y vista. `kind` existe si y solo si existe sujeto; cuando existe, el texto del rol
es exactamente `ParticipantKind::as_str()`. Su byte usa `ParticipantKind::tag()`:
0 Defendant, 1 Victim, 2 DefenseCounsel, 3 Prosecutor, 4 VictimCounsel,
5 ControlJudge, 6 TrialCourt, 7 Expert, 8 Police, 9 PrecautionarySupervisor,
10 Other.

Nombre, rol y organizacion se validan con las reglas de `ParticipantValues`:
200, 80 y 200 escalares respectivamente; nombre y rol obligatorios, sin controles.
El resultado normalizado debe ser identico a la vista: se rechazan espacios
exteriores y organizacion presente pero vacia, en lugar de cambiarlos al codificar.
No se incluyen valores completos del sujeto, identificadores civiles, credenciales
ni metadatos de administracion. Tampoco se demuestra una relacion juridica por
codificar una ficha tipada.

## Resultado de audiencia

```text
case UUID, hearing UUID, result UUID, revision U32, OPTION(agreement UUID)
values DIGEST, submission DIGEST, STATUS
occurrence byte, event_time, summary TEXT, OPTION(agreement TEXT)
```

La referencia completa de la vista debe coincidir. La presencia e identidad del
acuerdo deben coincidir con la seleccion; su texto se escribe al final, sin
repetir el UUID. `occurrence` usa 0 Occurred o 1 NotStarted.

El tiempo conserva el formato de la declaracion de audiencia, no el del hecho:
precision 0 Date o 1 Instant, anio local U16 big-endian, mes y dia de un byte;
solo Instant agrega hora, minuto y segundo, de un byte cada uno. Ambas precisiones
terminan con desfase I32 en segundos, big-endian. El desfase es obligatorio en
este tipo existente; no se altera HRES1 ni se transforma una fecha en instante.

La validacion de este canon no comprueba que el acuerdo exista dentro del HRES1
original, ni recompone su digest a partir de material completo. La
validacion de fuentes lo comprueba antes de derivar esta vista. No se consultan
cabezas actuales ni se readmiten documentos historicos por codificar una fuente.

## Soporte directo

```text
document UUID, version U32, content DIGEST, name TEXT, format byte, policy byte
```

El nombre debe satisfacer `ArchiveEntry`: 1..128 bytes ASCII, primer caracter
alfanumerico, resto alfanumerico, punto, guion o guion bajo. No se aceptan rutas,
espacios, caracteres de control ni cambios de nombre implicitos.

Formato usa 0 Pdf o 1 Docx. Politica usa 0 PdfDocxV1. Estos valores conservan
la admision capturada; codificarlos no ejecuta un parser ni valida contenido.
El archivo en claro, el contenedor cifrado y las pruebas criptograficas no forman
parte de PFSRC1. Las funciones y localizadores directos permanecen en PFRES1/PFNOT1.

## PFTXN1: recibo de operacion

El servicio prepara una operacion sin reservarla. El canon de envio vincula la
identidad del actor, expediente, raiz fija y contenido exacto que se confirma:

```text
"PFTXN1" (6 bytes)
operation UUID, actor UUID, case UUID
target: 0 + resolution UUID
     or 1 + notification UUID + parent resolution UUID
action: 0 Record, 1 Correct, 2 Withdraw
expected_revision U32
values DIGEST, sources DIGEST
OPTION(reason TEXT)
```

Record exige revision esperada cero y motivo ausente. Correct y Withdraw exigen
revision positiva, motivo presente y sucesor representable en U32. El padre de
una notificacion forma parte de su identidad fija, incluso durante un retiro.
El digest de valores corresponde a PFRES1 o PFNOT1; el de fuentes a PFSRC1.
Las cotas son 141..4145 bytes para resoluciones y 157..4161 para notificaciones.

No incluye correo, reloj ni administracion observada. Estos campos se capturan
por separado en la transaccion auditada. Una actualizacion administrativa activa
no invalida por si sola la preparacion; el cierre actual si debe impedir la
mutacion. R0 conserva sus metadatos y no inventa una revision administrativa.
Las fechas declaradas mantienen su precision; el servicio no aplica una regla
general de futuro ni deriva efectos juridicos a partir de ellas.

## Verificacion y limites

El verificador de historia reconstruye el envio, comprueba sucesor, accion,
estado y motivo, y comprueba el canon administrativo cuando existe una revision.
El verificador de snapshot agrega valores y padre inmutable. El de detalle agrega
la union exacta de referencias, el expediente de cada fuente y el canon de sus
proyecciones legibles. Las capturas cerradas siguen siendo legibles.

La derivacion de fuentes comprueba las revisiones originales de participantes,
resultados y resoluciones. El padre conserva su soporte historico y su resultado
seleccionado, sin expandir un grafo de documentos para volver a admitirlos.
Las comprobaciones autocontenidas detectan incoherencias: no prueban que una fila
exista en la base ni sustituyen la autorizacion y lectura exacta del adaptador.

El retiro conserva valores, fuentes y vistas de la base, y rechaza material
nuevo. Una correccion puede seleccionar otras revisiones dentro del mismo padre,
pero debe conservar las proyecciones de las referencias exactas que retiene.
El lote directo completo se valida una vez por preparacion, con los limites de
16 MiB por archivo y 32 MiB combinados. El envio vuelve a preparar y admitir el
lote: un borrador anterior no sustituye esa comprobacion. Tras admitirlo se
reautentica el mismo actor antes de devolver el borrador o solicitar la confirmacion.

## Trabajo restante

El adaptador debe repetir autorizacion, membresia, caso activo, cabeza, unicidad
de operacion, material exacto y registros admitidos bajo el bloqueo comun de
la auditoria. Debe confirmar raiz, revision, recibo y auditoria atomicamente.
Persistencia, migraciones, restauracion, API y Qadra requieren implementacion y
pruebas propias. Estos canones no activan reglas juridicas, plazos ni alertas.
