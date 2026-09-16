# Modelo puro de hechos declarados de resolucion y notificacion

Estado: valores y canones del dominio implementados. La evidencia de ejecucion
se registra en el [informe de verificacion](verification-report.md).
Este contrato describe identidades y validaciones puras. Los formatos binarios
se detallan en [PFRES1 y PFNOT1](procedural-facts-canonical.md). El
[servicio de aplicacion](procedural-facts-application.md), los
[canones de fuentes y recibos](procedural-facts-receipts.md) y la
[persistencia PostgreSQL](procedural-facts-persistence.md) se describen por separado.
El backend esta verificado localmente; API HTTP y Qadra siguen pendientes.
Este modelo puro no define un formato JSON de entrada HTTP.

La separacion de familias y referencias corregibles procede de
[ADR-0031](adr/0031-declared-procedural-facts.md). El modelo no acredita actos,
identidad civil, representacion, notificacion eficaz ni aplicabilidad juridica.
No activa recursos, plazos, alertas o trabajos de reevaluacion.

## 1. Identidades, raices y referencias

`ResolutionId`, `NotificationId` y `FactOperationId` conservan UUID exactos.
Permiten generar identificadores nuevos o construirlos desde un UUID, incluido
el UUID cero. Son tipos distintos; el valor por si solo no prueba existencia o
unicidad en almacenamiento. No se deduplican hechos por fecha, nombre o texto.

`FactRevision` contiene un u32 positivo. `initial()` representa 1; `new(0)` se
rechaza. `next()` produce el sucesor o `None` al alcanzar u32::MAX, sin wrap.
Esto no implementa una secuencia persistente ni comprueba una revision esperada.

- `ResolutionRoot::new(id, case_id)` fija identidad y expediente.
- `NotificationRoot::new(id, case_id, resolution_id)` fija ademas la raiz de
  resolucion a la que pertenece la captura.
- `FactResolutionRef { id, revision }` selecciona una revision exacta de resolucion
  dentro de los valores de notificacion. No contiene proyeccion ni digest resueltos.
- `NotificationRoot::validate_values` rechaza valores con otra raiz de resolucion;
  permite seleccionar otra revision de la misma raiz. Su comprobacion no consulta
  la resolucion, su expediente o su estado.

Cada raiz conserva su familia por su tipo. Varias notificaciones pueden referir
la misma resolucion y tener valores identicos. Una futura correccion expresa
podra cambiar referencias personales y revisiones seleccionadas conservando el
pasado. Una nueva practica debe tener otra raiz; cambiar la raiz de resolucion
vinculada tambien requiere otra notificacion. Distinguir correccion de nueva
actuacion es responsabilidad del futuro flujo, no una inferencia de estos valores.

## 2. Texto y declaraciones explicitas

| Tipo | Contenido y limites |
| --- | --- |
| `FactLabel` | Texto obligatorio de una linea, 1..200 escalares Unicode despues del recorte exterior. |
| `FactText` | Texto obligatorio, 1..1000 escalares Unicode, con multiples lineas. Normaliza CRLF a LF y recorta espacio exterior. |
| `FactDeclaration<T>` | `Known(T)` o `Unknown(FactText)`, con motivo no vacio. No hay inferencia ni valor conocido predeterminado. |

No componer ni descomponer Unicode; conservar espacios interiores. `FactLabel`
rechaza controles, incluidos saltos de linea y tabuladores, aun en los extremos
que se recortarian. `FactText` admite LF y normaliza CRLF; rechaza CR aislado y
los demas controles, tambien en los extremos. Un valor obligatorio que queda
vacio se rechaza. Las cotas cuentan escalares, no bytes UTF-8 ni grafemas.

Los subtipos opcionales usan `Option<FactLabel>`. La ausencia es `None`; un texto
vacio no constituye un `FactLabel` valido. La conversion de entradas externas
vacias o ausentes pertenece a un contrato HTTP posterior.

## 3. Catalogos descriptivos

Todos los catalogos se seleccionan expresamente. Sus variantes no certifican
clases normativas, competencia, firmeza, eficacia o consecuencias procesales.
`Other(FactLabel)` conserva una denominacion sin interpretarla ni convertirla en
otra variante. El desconocimiento se representa mediante `FactDeclaration`.

| Tipo | Variantes conocidas |
| --- | --- |
| `ResolutionClass` | `Order`, `Judgment`, `Other(FactLabel)` |
| `NotificationCharacter` | `Personal`, `Publication`, `Other(FactLabel)` |
| `NotificationMedium` | `InPerson`, `Electronic`, `Other(FactLabel)` |
| `NotificationContext` | `InHearing`, `OutsideHearing`, `Other(FactLabel)` |
| `NotificationOutcome` | `Practiced`, `Attempted` |

Caracter, medio y contexto son independientes. Una publicacion puede conservar
su medio electronico; no se reemplaza uno de esos datos por otro. El dominio no
aplica una matriz juridica de combinaciones permitidas. `Practiced` expresa lo
declarado por quien captura; no acredita recepcion ni efectos. `Attempted` no se
transforma en `Practiced` al conocer otro campo o al transcurrir el tiempo.

## 4. Personas y representacion

`FactPerson` tiene dos formas:

- `Participant(FactParticipantRef { id, revision })`: ficha historica exacta,
  con `ParticipantId` exacto y `ParticipantRevision` positiva.
- `Unlinked { label: FactLabel, description: FactText }`: identificacion declarada
  sin seleccion de una ficha del directorio.

No existe un `case_id` redundante dentro de estas referencias. El expediente de
la raiz delimita el contexto que la futura aplicacion debera comprobar. El sujeto
ligado, los digests y las proyecciones historicas tampoco se fabrican en el dominio;
se resolveran conforme al [directorio tipificado](typed-participants-api.md).
`Unlinked` no crea una ficha ni acredita identidad; no se empareja por nombre.

Destinatario pretendido y receptor material son dos `FactDeclaration<FactPerson>`
independientes, con exactamente un valor de cada funcion. No son listas ni cuentas
de acceso. Seleccionar la misma persona en ambas funciones no crea representacion.

`FactRepresentation` distingue:

- `NotRecorded(FactText)`: motivo de que no se haya registrado una relacion; no
  afirma que esa relacion no exista.
- `Declared { represented, representative, scope, provenance }`: ambos extremos
  son `FactPerson`, el alcance es `FactText` y la procedencia es
  `Box<FactProvenance>`. La indirección limita el tamaño de la variante en memoria;
  conserva los mismos valores y bytes canónicos.

La relacion declarada no se deduce de los perfiles, del receptor o de asistentes.
Los extremos pueden coincidir; el modelo conserva esa declaracion sin certificarla.
No se exige que el representado sea el destinatario o que el representante sea el
receptor. Tampoco se inventa un extremo desconocido para construir `Declared`.

## 5. Procedencia y soportes directos

`FactProvenance` es obligatoria y distingue:

| Variante | Datos |
| --- | --- |
| `OperatorNote` | `note: FactText`; sin soporte directo en esta variante. |
| `ExternalReference` | `reference: FactText`, `support: Option<FactEvidence>`. |
| `HearingResult` | `reference: FactHearingRef`, `locator: FactLabel`, `support: Option<FactEvidence>`. |

`FactHearingRef` conserva `hearing_id`, `result_id`, `revision` y un
`agreement_id` opcional. Son identidades exactas, no una copia del relato o una
lectura de la cabeza actual. La existencia del resultado y la pertenencia del
acuerdo a esa revision no se pueden comprobar sin un puerto. El
[contrato de resultados](hearing-results-api.md) conserva sus formatos y canones.
Una resolucion externa a audiencia no necesita inventar una programacion.

`FactEvidence::new(reference, digest, locator)` contiene `DocumentVersionRef`,
`Sha256Digest` y `FactLabel`. Expone `reference()`, `digest()` y `locator()`.
`FactSupportRef` conserva solo version documental y digest para reunir soportes.
Los valores no descifran, recalculan hashes, comprueban pertenencia o admiten
formatos. Un digest en ellos sigue siendo un dato que la aplicacion debe cotejar.

Una resolucion contiene como maximo un soporte directo, en su procedencia.
Una notificacion contiene como maximo dos: procedencia primaria y procedencia de
representacion. `direct_supports()` reune esas referencias para trabajo posterior:

- La misma version documental con el mismo digest se incluye una sola vez en el
  lote; cada funcion conserva su propio localizador en los valores completos.
- La misma version con digests diferentes se rechaza al construir la notificacion
  mediante `InvalidProceduralFact("support_digest")`.
- Versiones diferentes de un mismo documento son referencias diferentes.
- El soporte de representacion se conserva aunque no haya soporte primario.

El lote no incluye recursivamente documentos de la resolucion referida, fichas,
sujetos o resultados enlazados. Tampoco prueba admision por su nombre o existencia.
Las cotas del modelo fijan cantidad directa. El presupuesto de bytes, la admision
de formatos y la carga acotada de antecedentes estan implementados en
[aplicacion y almacenamiento](procedural-facts-persistence.md); no los ejecutan
estos valores puros.

## 6. Valores de resolucion

`ResolutionValues::new(ResolutionValuesInput)` construye valores con estos campos:

| Campo | Tipo |
| --- | --- |
| `class` | `FactDeclaration<ResolutionClass>` |
| `subtype` | `Option<FactLabel>` |
| `issuer` | `FactDeclaration<FactLabel>` |
| `issued_at` | `DeclaredProceduralTime` |
| `summary` | `FactText` |
| `provenance` | `FactProvenance` |

Los getters conservan cada dato declarado. `issuer` no selecciona implicitamente
una ficha ni infiere un organo por etapa. La construccion no requiere documento
adjunto o audiencia y no declara que la captura sea suficiente para un consumidor.
El [alcance de recursos](procedural-resources-scope.md) necesitara comprobar sus
propias condiciones antes de utilizar esta resolucion.

## 7. Valores de notificacion

`NotificationValues::new(NotificationValuesInput)` devuelve valores o un error de
inconsistencia entre digests de soportes directos. Los campos son:

| Campo | Tipo |
| --- | --- |
| `resolution` | `FactResolutionRef` |
| `character` | `FactDeclaration<NotificationCharacter>` |
| `medium` | `FactDeclaration<NotificationMedium>` |
| `context` | `FactDeclaration<NotificationContext>` |
| `outcome` | `FactDeclaration<NotificationOutcome>` |
| `subtype` | `Option<FactLabel>` |
| `practiced_at` | `DeclaredProceduralTime` |
| `received_at` | `Option<DeclaredProceduralTime>` |
| `stated_effect` | `Option<FactStatedEffect>` |
| `intended_recipient` | `FactDeclaration<FactPerson>` |
| `actual_receiver` | `FactDeclaration<FactPerson>` |
| `representation` | `FactRepresentation` |
| `summary` | `FactText` |
| `provenance` | `FactProvenance` |

`FactStatedEffect { at, statement, locator }` conserva un tiempo declarado, un
`FactText` y un `FactLabel`. Su localizador remite a la procedencia primaria de la
notificacion; no incorpora un tercer soporte. Es una afirmacion expresa conservada,
no un resultado de calculo ni un efecto que el modelo haya reconocido.

## 8. Tiempo, ausencia y limites de inferencia

Todos los tiempos usan [DeclaredProceduralTime](procedural-time.md). `issued_at`
y `practiced_at` son campos obligatorios y permiten `Unknown`. No se completan
con la fecha del servidor, de upload, de programacion o de otra declaracion.

`received_at = None` y `Some(Unknown)` son valores diferentes: el primero no
registra ese dato adicional; el segundo conserva su desconocimiento expreso.
Lo mismo distingue ausencia de `stated_effect` de una afirmacion de efecto cuyo
tiempo es desconocido. Recepcion, practica y efecto no se rellenan entre si.

Fecha sin desfase y minuto sin segundos siguen siendo representables. No se
convierte una precision incompleta en un instante ni se compara con el presente.
Este modelo no impone orden cronologico entre los campos o con fuentes externas,
ni hereda las restricciones temporales de resultados o etapas.

## 9. Validacion pura y trabajo pendiente

Los constructores comprueban forma de datos, texto, revisiones y coherencia local
de soportes. `validate_values()` comprueba solamente la raiz de resolucion fijada.
No prueban existencia, pertenencia al expediente, autorizacion, actividad, estado
de fuentes, autenticidad, representacion o eficacia juridica. La aplicacion y el
adaptador resuelven las fuentes exactas y capturan digests/sujetos ligados sin
refrescar la historia; su comprobacion no atribuye eficacia juridica.

El [contrato de aplicacion](procedural-facts-application.md) incorpora comandos de
alta, correccion y retiro terminal, con comprobaciones puras y conservacion de
identidad. Los valores de dominio no ejecutan esas operaciones. El servicio
coordina preparacion, envio y comprobacion de recibos; PostgreSQL confirma
persistencia y auditoria atomica. Sus lecturas permiten consultar la revision
exacta para conciliar una operacion; la interfaz HTTP/Qadra sigue pendiente.
El retiro no declara nulidad.

Los [canones de valores](procedural-facts-canonical.md) no son recibos. Los
[formatos PFSRC1/PFTXN1](procedural-facts-receipts.md), los puertos, limites de
consulta, servicio y adaptador estan implementados fuera del dominio puro.
El cierre de backend no habilita por si solo integracion HTTP o de navegador.
No existe outbox sin consumidor ni activacion del
[conteo diario](deadline-day-counting.md). Los catalogos y datos declarados no
seleccionan una regla, recurso, calendario, responsable, canal o corte temporal.

Las comprobaciones del dominio deben cubrir texto Unicode y controles, UUID cero,
revision maxima, referencias exactas, independencia de funciones/tiempos y soportes
duplicados o contradictorios. La evidencia de ejecucion se registra por separado;
el modelo puro por si solo no demuestra el comportamiento de los adaptadores.
