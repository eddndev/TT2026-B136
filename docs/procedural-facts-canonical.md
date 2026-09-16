# Canones de valores declarados PFRES1 y PFNOT1

Estado: codificadores de valores implementados en el dominio; comprobacion global
local aprobada. Este contrato no define recibos, persistencia, decodificador publico,
API HTTP o autorizacion. Complementa el [modelo puro](procedural-facts.md) y
[ADR-0031](adr/0031-declared-procedural-facts.md).

`ResolutionValues::canonical_bytes()` produce PFRES1 y
`NotificationValues::canonical_bytes()` produce PFNOT1. Los formatos conservan
valores ya construidos y normalizados por el dominio; no interpretan texto ni
verifican que las referencias existan. HRES1 y los formatos de etapas conservan
sus contratos y bytes. No se usa su codificacion temporal como sustituto.

## 1. Convenciones binarias

La salida es concatenacion exacta en el orden indicado, sin separadores, padding,
terminadores NUL o longitud global. Cada tag ocupa un byte. Los enteros de varios
bytes se escriben en big-endian (BE): byte mas significativo primero.

| Simbolo | Codificacion |
| --- | --- |
| `u8` | Un byte sin signo. |
| `u16` | Dos bytes BE sin signo; se usa para el anio civil. |
| `u32` | Cuatro bytes BE sin signo; revisiones, versiones y longitudes. |
| `i32` | Cuatro bytes BE con signo en complemento a dos; desfase en segundos. |
| `UUID` | Los 16 bytes exactos de `Uuid::as_bytes()`, sin texto ni guiones. |
| `Digest` | Los 32 bytes de SHA-256, sin hexadecimal ni prefijo de longitud. |
| `Text(s)` | `u32(longitud UTF-8 en bytes)` seguido por esos bytes UTF-8. |
| `Optional(v)` | Tag 0 para ausencia, sin payload; tag 1 seguido por el valor presente. |
| `Declaration(v)` | Tag 0 seguido por `Text(motivo)` para Unknown; tag 1 seguido por el valor Known. |

`Declaration` no es un opcional: Unknown conserva un `FactText` obligatorio.
Un tag de presencia no sustituye el tag interno de una variante. UUID cero se
conserva; revisiones y versiones solo proceden de sus tipos positivos validos.
Los digests documentales incluidos siguen siendo expectativas declaradas, no
resultados de integridad que este codificador haya calculado.

`FactLabel` admite hasta 200 escalares y `FactText` hasta 1000, conforme al
[contrato de texto](procedural-facts.md). El prefijo cuenta bytes: los payloads
maximos son 800 y 4000 bytes respectivamente. El minimo es un byte no vacio.
Normalizacion CRLF/recorte ocurre antes, en los constructores. No normalizar
Unicode interior, ordenar texto o serializar escapes JSON para producir el canon.

## 2. Tags de catalogos conocidos

Los siguientes tags van dentro de `Declaration` cuando esta es Known:

| Tipo | Tag 0 | Tag 1 | Tag 2 |
| --- | --- | --- | --- |
| `ResolutionClass` | Order | Judgment | Other seguido por `Text(label)` |
| `NotificationCharacter` | Personal | Publication | Other seguido por `Text(label)` |
| `NotificationMedium` | InPerson | Electronic | Other seguido por `Text(label)` |
| `NotificationContext` | InHearing | OutsideHearing | Other seguido por `Text(label)` |
| `NotificationOutcome` | Practiced | Attempted | No definido |

Caracter, medio y contexto conservan sus posiciones independientes. La presencia
de Publication no elimina Electronic. Los tags no asignan eficacia juridica a
una declaracion ni habilitan un recurso o calculo.

## 3. Tiempo declarado

Codificar [DeclaredProceduralTime](procedural-time.md) mediante componentes
locales y el desfase original, nunca mediante un timestamp UTC normalizado.
`Civil` significa `u16(anio) + u8(mes) + u8(dia)`.

| Precision | Bytes, en orden | Longitud |
| --- | --- | --- |
| Unknown | Tag 0, sin mas campos. | 1 |
| Date | Tag 1, Civil, Optional(desfase). | 6 sin desfase; 10 con desfase |
| Minute | Tag 2, Civil, u8(hora), u8(minuto), Optional(desfase). | 8 sin desfase; 12 con desfase |
| Second | Tag 3, Civil, u8(hora), u8(minuto), u8(segundo), Optional(desfase). | 9 sin desfase; 13 con desfase |

El payload de desfase presente es `i32(segundos)`, aunque el valor valido exige
minutos enteros y rango -50400..50400 segundos. Ausencia es un tag 0; UTC declarado
es tag 1 seguido por cuatro bytes cero. Unknown temporal no lleva ni siquiera
un tag de desfase, y no es `FactDeclaration::Unknown` con motivo textual.

Minute no escribe un segundo cero. Date no escribe hora, minuto o segundo.
Dos valores Second que coincidan en UTC pero tengan distintos componentes locales
o desfases producen bytes distintos. Los limites locales/UTC y la validacion de
intervalos son los del valor temporal, sin nuevas reglas legales o comparaciones
con Clock en el codificador.

En un tiempo opcional, `None` produce un unico byte hexadecimal `00`, mientras
`Some(Unknown)` produce `01 00`. No colapsar ambas formas.

## 4. Personas y representacion

`Person` se escribe sin un `Declaration` adicional interno:

- Participant: tag 0, UUID de ficha, u32 de revision exacta.
- Unlinked: tag 1, Text(label), Text(description).

Destinatario y receptor envuelven cada `Person` en su propio `Declaration`.
Los extremos de una representacion declarada se codifican directamente como
`Person`; no se infieren ni se completan desde destinatario o receptor.

`Representation` tiene estas formas:

- NotRecorded: tag 0, Text(reason).
- Declared: tag 1, Person(represented), Person(representative), Text(scope),
  Provenance(provenance).

El orden representado/representante es significativo. La coincidencia de sus
referencias no suprime ningun extremo ni certifica la relacion.

## 5. Evidencia y procedencia

`Evidence` concatena, sin un tag propio:

1. UUID documental.
2. u32 de version.
3. Digest declarado de contenido.
4. Text(locator) de esa funcion documental.

`Provenance` concatena segun su variante:

| Variante | Orden completo |
| --- | --- |
| OperatorNote | Tag 0, Text(note). |
| ExternalReference | Tag 1, Text(reference), Optional(Evidence(support)). |
| HearingResult | Tag 2, UUID hearing_id, UUID result_id, u32 revision, Optional(UUID agreement_id), Text(locator), Optional(Evidence(support)). |

La referencia de resultado conserva su revision exacta; el acuerdo opcional se
ubica antes del localizador. No se serializa el relato HRES1, el documento cifrado,
la ficha completa, una proyeccion resuelta o un recibo del antecedente.

El lote derivado `direct_supports()` no se serializa. Si constancia y
representacion comparten documento/version/digest, cada procedencia sigue
incluyendo su evidencia y localizador propios. La deduplicacion de admision no
elimina una afirmacion del canon. El constructor rechaza digests contradictorios
para una misma version antes de que exista un `NotificationValues` valido.

## 6. PFRES1: orden completo

1. Prefijo ASCII `PFRES1`, exactamente 6 bytes.
2. Declaration(class), con los tags de `ResolutionClass`.
3. Optional(Text(subtype)).
4. Declaration(issuer), cuyo Known contiene Text(label).
5. Tiempo issued_at.
6. Text(summary).
7. Provenance(provenance).

No se incluyen UUID propio, expediente o revision de la raiz de resolucion.
Valores iguales de dos raices diferentes pueden tener el mismo PFRES1.

## 7. PFNOT1: orden completo

1. Prefijo ASCII `PFNOT1`, exactamente 6 bytes.
2. UUID de la raiz de resolucion seleccionada.
3. u32 de la revision de esa resolucion.
4. Declaration(character).
5. Declaration(medium).
6. Declaration(context).
7. Declaration(outcome).
8. Optional(Text(subtype)).
9. Tiempo practiced_at.
10. Optional(Tiempo received_at).
11. Optional(StatedEffect).
12. Declaration(Person(intended_recipient)).
13. Declaration(Person(actual_receiver)).
14. Representation(representation).
15. Text(summary).
16. Provenance(provenance).

`StatedEffect` no tiene otro tag propio: concatena Tiempo(at), Text(statement),
Text(locator). Su tag de presencia pertenece al Optional exterior. Conserva una
afirmacion que remite a la procedencia primaria, sin tercer soporte ni efecto
calculado. La revision de resolucion es parte de PFNOT1 y cambiarla cambia bytes.
No se incluyen UUID propio o revision de la raiz de notificacion, ni expediente.

## 8. Cotas exactas y corpus independiente

Las cotas corresponden a valores validos, incluyendo prefijo, tags y longitudes:

| Canon | Minimo | Maximo |
| --- | --- | --- |
| PFRES1 | 27 bytes | 17700 bytes |
| PFNOT1 | 67 bytes | 58671 bytes |

Los maximos requieren considerar los motivos Unknown de 1000 escalares: pueden
ocupar mas que Known(Other(label)). No basta maximizar solo variantes conocidas.
Cada escalar del fixture maximo ocupa cuatro bytes UTF-8.

El maximo PFRES1 se descompone como
`6 + 4005 + 805 + 4005 + 13 + 4004 + 4862 = 17700`.
Corresponde a prefijo, clase Unknown, subtipo, emisor Unknown, tiempo, resumen y
procedencia externa con soporte, respectivamente.

El maximo PFNOT1 se descompone como
`6 + 20 + 4*4005 + 805 + 13 + 14 + 4822 + 2*4810 + 18485 + 4004 + 4862 = 58671`.
Incluye referencias de resolucion, cuatro declaraciones Unknown, subtipo,
tiempos, efecto, personas Known(Unlinked), representacion declarada completa,
resumen y procedencia primaria externa. Ambas evidencias pueden ser distintas.

El [generador Python independiente](../crates/domain/tests/fixtures/generate_procedural_fact_vectors.py)
produce [28 vectores](../crates/domain/tests/fixtures/procedural_fact_vectors.json)
con bytes, hexadecimal y SHA-256, sin invocar Rust. El corpus incluye minimos,
maximos UTF-8, variantes temporales, procedencia y funciones documentales.
Los vectores son sinteticos; no prueban hechos juridicos o fuentes reales.
La evidencia de ejecucion y la comprobacion global se registran por separado,
sin presentar pruebas focales como aprobacion de CI o integracion HTTP.

Estas cotas binarias no fijan un limite HTTP: JSON puede tener otro tamano y sus
limites, presupuestos y validacion necesitaran un contrato propio.

## 9. Limites de confianza y recibos futuros

PFRES1/PFNOT1 vinculan declaraciones, no operaciones confirmadas. La codificacion
no calcula ni adjunta un digest propio, valida permisos, consulta fuentes o escribe
auditoria. Sus bytes no prueban que una referencia exista o pertenezca al expediente.
No hay decodificador publico ni validacion de bytes recibidos en este corte.

Identidad propia, expediente, actor, operacion, accion, expectativas y revision
resultante quedan fuera. Tambien quedan fuera los snapshots resueltos de resolucion,
resultado, ficha/sujeto, administracion, autor y tiempo de captura; los digests
aportados en Evidence no reemplazan esa verificacion futura.

La persistencia necesitara recibos separados que vinculen esos datos de operacion,
los valores y las fuentes exactas resueltas bajo la frontera auditada. Un hash de
PFRES1/PFNOT1 aislado no es un recibo ni permite conciliar una respuesta incierta.
No se define aqui su prefijo o formato, ni se reutiliza HRES1 o el recibo HRTX1.
