# Audiencias propias de recursos

## Estado y alcance

Implementación local en curso: el dominio y `ResourceHearingService` disponen de
preparación, confirmación con huella exacta y recuperación de una creación previa
mediante `ResourceHearingStore`. La aplicación comprueba la captura devuelta y su
origen. Estos recorridos se verifican con puertos controlados y memoria; todavía
no existe adaptador PostgreSQL para estas audiencias. La creación durable con su
asociación y auditoría, consultas propias, HTTP, agenda, alertas e interfaz siguen
pendientes dentro de esta misma entrega. No se atribuye atomicidad durable,
un nuevo despliegue ni CI global a esas comprobaciones.

La decisión está en [ADR-0069](adr/0069-resource-hearing-scheduling.md). Este
contrato complementa los [recursos](procedural-resources-api.md) y sus
[asociaciones con actividades](resource-activities-api.md), sin modificar la
programación ordinaria ni el formato `HEAR1`.

## Clasificación declarada y fuente normativa

| Tipo cerrado | Compatibilidad declarada |
| --- | --- |
| `AppealArguments` / `appeal_arguments` | Recurso `Appeal`, modalidad conocida `Written`. |
| `WrittenRevocation` / `written_revocation` | Recurso `Revocation`, modalidad conocida `Written`. |

Los artículos 476–477 contemplan una audiencia de alegatos aclaratorios en
apelación. El artículo 466 distingue la revocación oral en audiencia de la
escrita y contempla citación a audiencia por complejidad en esta última.
Esta clasificación no determina procedencia ni calcula el señalamiento.
Fuente consultada el **3 de octubre de 2026**:
[CNPP oficial, páginas 132–136](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf#page=132),
que declara última reforma DOF **28-11-2025**.

La fecha y la necesidad de audiencia se declaran expresamente con su soporte.
El módulo no elige una regla de cómputo del artículo 476 ni deriva una audiencia
separada de la revocación oral. Compatibilidad entre valores declarados no equivale
a calificación jurídica del caso.

## Valores de programación

`ResourceHearingValues::new` recibe los campos siguientes:

| Campo | Representación y límite |
| --- | --- |
| `kind` | Uno de los dos tipos cerrados anteriores. |
| `scheduled_at` | `HearingTime`: instante de segundos enteros, desfase explícito múltiplo de minuto hasta ±14 h; años local y UTC entre 1 y 9999. Conserva el desfase original; no completa una fecha parcial. |
| `modality` | `InPerson` o `Videoconference`. |
| `venue` | `HearingVenue`: texto no vacío, una línea, hasta 500 escalares Unicode; no es una URL ejecutable. |
| `note` | Nota opcional de hasta 1000 escalares, con saltos de línea admitidos. |
| `participants` | De cero a 32 referencias `HearingParticipantRef`, cada una con identidad UUID y revisión positiva. Se ordenan por UUID y se rechaza repetir una identidad, aun con otra revisión. UUID cero sigue siendo una identidad representable. |
| `scheduling_basis` | Declaración obligatoria `HearingNote` y `HearingSupportRef`: documento, versión exacta y digest. No es certificación judicial. |

Los textos recortan espacios exteriores y rechazan controles; el texto multilineal
normaliza CRLF a LF. Los valores no contienen etapa, autor,
recibo ni estado persistido. Tampoco deciden si el instante ya pasó. La familia
dispone de `ResourceHearingId`, `ResourceHearingOperationId` y
`ResourceHearingRevision` propios, separados de los de audiencia ordinaria. La
revisión comienza en uno, rechaza cero y no desborda al agotarse; su existencia
no incorpora todavía un flujo de reemplazo o cancelación.

## Comando y material del puerto

`ResourceHearingCommand` conserva operación, identidad prevista de audiencia,
identidad prevista de asociación, revisión esperada del recurso, captura exacta
del recurso, acto opcional y valores. La identidad prevista no demuestra que se
haya reservado o creado ninguno de esos objetos.

La captura del recurso contiene identidad, revisión y `capture_digest`. El acto
opcional añade su identidad/revisión, la revisión del recurso que lo contiene y
su huella. La revisión esperada corresponde a la cabeza actual; no sustituye la
revisión histórica elegida.

`ResourceHearingStore::prepare(actor, case, resource, command)` debe autorizar
pertenencia y acceso vigentes al expediente antes de la búsqueda. Devuelve una
de dos alternativas:

- `Ready(ResourceHearingMaterial)`: recurso y acto exactos, cabeza actual del
  recurso resuelta por separado, administración observada y revisiones actuales
  de los participantes seleccionados, limitadas a 32. Los soportes proceden de
  las capturas ya admitidas. Esta lectura no escribe.
- `Replay(ResourceHearingCreation)`: creación original identificada por su
  captura inmutable y su marcador de origen. La existencia aislada de una
  audiencia o asociación no acredita que proceda de la misma operación.

La recuperación conserva la creación original aunque su asociación ya no esté
vinculada; esa independencia debe sostenerla el marcador durable del adaptador.
No reconstruye la evidencia histórica desde cabezas nuevas.

`ResourceHearingStore::commit` recibe un `PreparedResourceHearing`. Su contrato
exige, bajo el bloqueo compartido de auditoría, reautorizar expediente y principal
completo, revalidar caso y recurso activos y las cabezas revisadas del recurso y
participantes, y comparar todo el material preparado. Debe escribir audiencia,
asociación inicial, marcador de origen y auditoría en **una transacción**. Un
conflicto no escribe; una carrera de la misma operación exacta devuelve la
creación original, incluida su fecha de registro.

Estas obligaciones delimitan el futuro adaptador. No hay migración SQL ni
comprobación transaccional PostgreSQL de este flujo. En particular, autorización
vigente por expediente y actualidad de las revisiones de participantes siguen
siendo obligaciones del puerto, no garantías demostradas por los dobles de prueba.

## Validación y revisión previa

El servicio autentica y admite sólo Owner o Litigator antes de invocar el puerto;
Paralegal y Client se rechazan sin cargar material. La validación de aplicación:

1. Comprueba expediente, identidad del recurso y recibo de su cabeza.
2. Contrasta administración actual con las capturas históricas, sin retroceso ni
   sustitución de una revisión exacta; rechaza el cierre administrativo registrado.
3. Exige cabeza activa y coincidencia con `expected_resource_revision`.
4. Verifica recibos y referencias del recurso y acto seleccionados. Rechaza
   capturas posteriores a la cabeza o distintas de ella bajo la misma revisión.
5. Exige tipo y modalidad escrita compatibles tanto en la selección del recurso
   como en su cabeza. Desconocimiento u oralidad no se completan automáticamente.
6. Localiza el soporte exacto en el recurso seleccionado o en los soportes del
   acto seleccionado. Una versión coincidente con digest o captura distintos se
   rechaza; no basta que exista un documento en el expediente.
7. Resuelve las proyecciones de participantes del mismo expediente, contrasta
   identidad/revisión y huellas de valores y rechaza fichas archivadas.

El resultado `ResourceHearingDraft` incluye comando, recurso, acto, soporte,
proyecciones de participantes, administración observada, referencia de la cabeza,
autor y `submission_digest`. Después de construirlo, el servicio autentica de
nuevo y compara el `Principal` completo. Un cambio de identidad, correo o rol no
permite devolver la revisión anterior; un rol ahora denegado conserva su rechazo.

No se exige ni se fabrica una etapa ordinaria. La comprobación de una referencia
histórica no la convierte en la cabeza ni demuestra que una autoridad haya
confirmado la declaración del usuario.

## Confirmación y recuperación explícita

`prepare_resource_hearing_change` ordena el material de participantes y construye
`PreparedResourceHearing` después de validar el comando y las fuentes. Sus campos
privados conservan revisión, material y principal; no puede construirse omitiendo
esa preparación. `into_creation(recorded_at)` produce y valida la captura inicial
en memoria. No escribe por sí mismo.

`submit`, cuando recibe material `Ready`, vuelve a preparar el comando y compara
`submission_digest` con la huella confirmada por el llamador. Una diferencia impide invocar `commit`. Si coincide,
reautentica el principal completo, realiza una sola llamada a `commit`, verifica
la creación devuelta y exige que su revisión previa sea exactamente la preparada.
Antes de devolverla vuelve a reautenticar. Un error posterior al commit no prueba
que no hubo escritura; el servicio no inicia un reintento automático.

`ResourceHearingCreation` reúne dos piezas:

- `ResourceHearingDetail`: revisión previa, material histórico exacto, revisión
  inicial de audiencia, instante de registro y `capture_digest`.
- `ResourceHearingOrigin`: expediente, recurso, audiencia, operación, asociación
  y huellas de envío y captura. Debe coincidir completamente con el detalle.

El verificador reconstruye la revisión desde el material histórico y contrasta
sus huellas. El registro debe ser UTC, representable entre los años 1 y 9999 y no
anterior a las fuentes capturadas. El servicio rechaza además una captura fechada
después de su reloj. La validación histórica no concede autorización de acceso.

Cuando el puerto devuelve `Replay`, el servicio comprueba creación, origen,
expediente, recurso, identidad del autor y comando completo. En `submit` exige
además la huella confirmada. Devuelve la captura original sin llamar a `commit`.
El correo histórico se conserva aunque haya cambiado antes de esta nueva llamada;
el principal actual debe permanecer idéntico durante la llamada y mantener un
rol permitido. Una autoridad perdida impide devolver la evidencia.

El llamador puede iniciar expresamente esa conciliación tras una respuesta
incierta usando la misma operación y contenido. Los ensayos en memoria conservan
una sola creación entre dos instancias del servicio; no demuestran recuperación
tras reiniciar un proceso ni supervivencia de datos en un servidor real.

## Representaciones canónicas

Todos los enteros usan big-endian. UUID y SHA-256 se conservan como bytes crudos.
`RHEAR1` codifica, en este orden: prefijo, tipo u8, segundos Unix i64, desfase i32,
modalidad u8, lugar, presencia de nota y nota si existe, número de participantes
u8 y pares UUID/revisión u32, declaración de señalamiento y soporte como UUID,
versión u32 y digest de 32 bytes. Los textos son UTF-8 con longitud de bytes u32.
Tipos y modalidades usan etiquetas 0/1 según el orden de las tablas anteriores;
la presencia usa 0/1. La lista ya está normalizada por el constructor.

`RHPR1` es la representación interna de la revisión preparada. Vincula expediente,
operación, audiencia, asociación, actor/correo, revisión esperada, captura elegida
y cabeza observada, acto opcional, `RHEAR1`, administración y proyecciones exactas
del soporte y de cada participante. Los bloques variables usan longitud u64.
Soporte y participantes reutilizan la validación y representación `PFSRC1`, sin
ampliar los cupos de fuentes de hechos para acomodar el cupo de esta audiencia.
La implementación está en
[`canonical.rs`](../crates/application/src/resource_hearings/canonical.rs).

`submission_digest` identifica la revisión que el llamador confirma. Por sí solo
no es un recibo durable ni reserva identificadores o un horario.

`RHCR1` vincula esa huella `RHPR1`, la revisión inicial y el instante de registro
con la procedencia histórica de cada participante. Los bloques se ordenan por
UUID y revisión, y cada uno incluye ambos campos para impedir intercambiar
procedencias entre fichas. Distingue ficha manual o tipificada e incorpora los datos de autoría y tiempo no proyectados en la revisión
previa; para fichas tipificadas incluye el envío y la referencia de origen de
credencial, cuando existe, y para sujetos vinculados su autoría y fecha. Esta
vinculación no certifica validez jurídica de la credencial. El detalle conserva
el material necesario para reconstruir y comprobar la revisión. La implementación
del recibo y del marcador de origen está en
[`receipt.rs`](../crates/application/src/resource_hearings/receipt.rs).

No existe decodificador ni API de importación de estas representaciones en este
módulo. Las huellas y el contrato de confirmación de aplicación no sustituyen la
persistencia transaccional que debe conservar sus capturas y origen.

## Comprobación y trabajo siguiente

Los casos de [dominio](../crates/domain/tests/resource_hearing_values.rs) incluyen
catálogo cerrado, compatibilidad, cupo y duplicados, vector independiente y
variación de campos canónicos. Las pruebas del
[servicio](../crates/application/tests/resource_hearing_service.rs) usan puertos
controlados para comparar fuentes, soporte, cabeza, permisos y reautenticación.
Los casos de [confirmación](../crates/application/tests/resource_hearing_submission.rs)
contrastan huella, captura completa, origen, reloj y cambios de principal. Los de
[recuperación](../crates/application/tests/resource_hearing_recovery.rs) simulan una
respuesta perdida y una conciliación explícita sin segunda escritura, preservan
el autor histórico y rechazan intentos distintos o autoridad perdida. La
regresión de procedencia altera autoría y orden de participantes sin permitir
que el intercambio conserve una captura válida.
Los resultados ejecutados se registran en [el informe técnico](verification-report.md).
No se reutilizan como evidencia de persistencia, transporte o navegador.

Los siguientes pasos son implementar el adaptador con autorización y transacción
reales, conservar historia y origen durable, y conectar consultas, HTTP, agenda,
alertas y Qadra. La asociación inicial prevista pertenece a este contrato de
creación; aún no se ha implementado su registro SQL ni se ha ampliado el flujo
genérico de asociaciones para crear estas audiencias. El soporte sólo puede ser
uno ya admitido en el recurso o acto seleccionado: no se incorpora una citación
nueva y arbitraria mediante esta confirmación.
