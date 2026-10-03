# Audiencias propias de recursos

## Estado y alcance

Implementación local en curso: existen valores de dominio y preparación mediante
`ResourceHearingService` y `ResourceHearingStore`. La preparación produce una
revisión en memoria; no crea todavía una audiencia ni una asociación persistida.
Persistencia, creación idempotente, consultas propias, ruta HTTP, agenda, alertas
e interfaz siguen pendientes dentro de esta misma entrega. Las comprobaciones
focales no acreditan un nuevo despliegue ni CI global.

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
normaliza CRLF a LF. El dominio no contiene etapa, raíz de audiencia, autor,
recibo ni estado persistido. Tampoco decide si el instante ya pasó.

## Comando y material del puerto

`ResourceHearingCommand` conserva operación, identidad prevista de audiencia,
identidad prevista de asociación, revisión esperada del recurso, captura exacta
del recurso, acto opcional y valores. La identidad prevista no demuestra que se
haya reservado o creado ninguno de esos objetos.

La captura del recurso contiene identidad, revisión y `capture_digest`. El acto
opcional añade su identidad/revisión, la revisión del recurso que lo contiene y
su huella. La revisión esperada corresponde a la cabeza actual; no sustituye la
revisión histórica elegida.

`ResourceHearingStore::prepare(actor, case, resource, command)` debe:

- Autorizar pertenencia y acceso vigentes al expediente antes de cargar material.
- Resolver el recurso y acto exactos y, por separado, la cabeza actual del recurso.
- Resolver la administración del expediente y las revisiones seleccionadas de
  participantes, exigiendo que estas últimas sean actuales.
- Limitar los participantes y devolver los soportes ya admitidos dentro de las
  capturas seleccionadas. No realizar escrituras.

El puerto devuelve `ResourceHearingMaterial`. Aún no existe su adaptador de
persistencia: las obligaciones de acceso vigente y cabeza actual de participantes
son parte del contrato del puerto, no evidencia de una comprobación PostgreSQL
ya ejecutada por este módulo.

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

`submission_digest` identifica esta revisión para su inspección. No es un recibo
de confirmación, ni hay decodificador o API de importación de estas dos
representaciones en este módulo. La futura persistencia debe definir su propio
contrato de confirmación y conciliación antes de tratar el borrador como creado.

## Comprobación y trabajo siguiente

Los casos de [dominio](../crates/domain/tests/resource_hearing_values.rs) incluyen
catálogo cerrado, compatibilidad, cupo y duplicados, vector independiente y
variación de campos canónicos. Las pruebas del
[servicio](../crates/application/tests/resource_hearing_service.rs) usan puertos
controlados para comparar fuentes, soporte, cabeza, permisos y reautenticación.
Los resultados ejecutados se registran en [el informe técnico](verification-report.md).
No se reutilizan como evidencia de persistencia, transporte o navegador.

Los siguientes pasos de la entrega son resolver material con autorización real,
confirmar audiencia y asociación atómicamente, conservar historia y conciliación
idempotente, y conectar consultas, HTTP, agenda, alertas y Qadra. Sus garantías
no se deducen del éxito de la preparación local.
