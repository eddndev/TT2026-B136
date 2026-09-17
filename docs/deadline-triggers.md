# Extracción exacta de tiempos para el cálculo de plazos

Estado: componente puro de dominio implementado; la campaña de verificación
integrada está aprobada localmente. Este contrato cubre la selección y extracción de un
tiempo declarado y su conexión con la aritmética. No implementa un perfil
normativo, una evaluación persistente ni un vencimiento operativo.

La decisión se conserva en [ADR 0033](adr/0033-exact-deadline-trigger-extraction.md).
El alcance restante está en la [matriz de producto](product-completion.md).

## API y entradas

El módulo [domain::deadline_triggers](../crates/domain/src/deadline_triggers/mod.rs)
expone dos funciones:

- `extract_trigger_time(requirement, selection, material)` devuelve
  `Result<TriggerExtraction, TriggerIntegrityError>`.
- `evaluate_triggered_arithmetic(requirement, selection, material, rule, calendar)`
  devuelve `Result<TriggeredArithmetic, TriggerIntegrityError>`.

La selección contiene el expediente, una declaración de fuente conocida o
desconocida y una calificación temporal opcional. La fuente conocida identifica
una revisión exacta; no significa cabeza actual ni selección autorizada.

| Familia de fuente | Referencia seleccionada | Material suministrado |
| --- | --- | --- |
| `Resolution` | ID y revisión de resolución | Raíz, revisión, valores y huellas |
| `Notification` | ID y revisión de notificación, más ID y revisión de su resolución | Raíz de notificación, revisión, valores y huellas |
| `HearingResult` | ID de audiencia, ID y revisión de resultado, acuerdo opcional | Expediente, audiencia, resultado, revisión, valores y huellas |

Las raíces y los valores reutilizan [hechos declarados](procedural-facts.md) y
[resultados de audiencia](hearing-results-api.md). `TriggerMaterial` toma los
valores prestados; el resultado conserva copias propias de la selección y de la
procedencia necesaria. No importa modelos de aplicación ni consulta puertos.

## Orden de comprobación

1. Fuente desconocida sin material: `UnknownSource`, sin captura de fuente.
   Si recibe material, devuelve `UnexpectedMaterial`.
2. Fuente conocida sin material: `MissingMaterial`.
3. Comprobar expediente, familia, identidades y revisiones contra el material.
   Verificar además el padre exacto de notificación o la pertenencia del acuerdo
   seleccionado a esa revisión del resultado.
4. Solo después, comparar la familia requerida con la fuente resuelta y evaluar
   el campo o la calificación solicitados.

Una incompatibilidad semántica no oculta un material de otro expediente o una
revisión discordante. La comprobación del padre de notificación compara raíz,
valores y selección: no fabrica ni resuelve una segunda captura de resolución.

| Error de integridad | Causa |
| --- | --- |
| `MissingMaterial` | Fuente conocida sin material |
| `UnexpectedMaterial` | Material suministrado para una fuente desconocida |
| `CaseMismatch` | El material pertenece a otro expediente |
| `SourceMismatch` | Familia, identidad o revisión no coinciden con la selección |
| `ParentMismatch` | Padre de notificación discordante entre raíz, valores y selección |
| `MissingAgreement` | El acuerdo seleccionado no pertenece al resultado exacto |
| `InvalidTime` | La conversión de un tiempo consumido no conserva componentes representables |

Estas discrepancias devuelven un error, no una evaluación bloqueada válida.
`InvalidTime` pertenece a la conversión del campo temporal consumido; las
comprobaciones de identidad anteriores preceden a los bloqueos semánticos.

## Campos y desconocimiento

`SourceField` nombra exactamente uno de los siguientes campos:

| Campo | Valor consumido |
| --- | --- |
| `ResolutionIssuedAt` | `issued_at` |
| `NotificationPracticedAt` | `practiced_at` |
| `NotificationReceivedAt` | `received_at` |
| `NotificationStatedEffectAt` | `stated_effect.at` |
| `HearingSessionEventTime` | `event_time` del resultado declarado |

No hay sustitución entre emisión, práctica, recepción, efecto declarado o tiempo
de sesión. Un campo opcional ausente produce `AbsentField(campo)`. Un campo
presente con precisión `Unknown` produce `Extracted { at: Unknown }`: se conoce
cuál es el campo seleccionado, aunque su tiempo siga siendo desconocido.

La conversión de audiencia conserva fecha local y desfase originales. `Date`
permanece fecha; `Instant` se convierte en `Second` con hora, minuto y segundo
originales. No utiliza límites del día, programación, fecha de captura ni reloj.
Tampoco interpreta resumen o acuerdos, ni convierte `Occurred` o `Concluded`
en una declaración del final de audiencia. Se conserva la precisión definida
en [procedural-time.md](procedural-time.md); no cambian HRES1 ni sus valores.

## Calificación temporal declarada

`Qualified` exige un propósito y una familia explícitos. Los propósitos actuales
son `HearingEnd` y `OrderedPeriodStart`. `QualifiedTriggerTime` conserva el
propósito, tiempo declarado, `statement` y `locator` con los tipos de texto del
modelo de hechos.

La calificación pertenece a la misma fuente y revisión seleccionadas. No posee
una referencia alternativa con la que sustituirlas. Su tiempo puede seguir
siendo desconocido. El resultado conserva también texto y localizador mediante
la copia de `TriggerSelection`; editar la entrada después no cambia esa copia.

| Condición semántica, tras resolver material íntegro | Resultado bloqueado |
| --- | --- |
| Familia requerida diferente de la seleccionada | `IncompatibleFamily` |
| `SourceField` acompañado de calificación | `UnexpectedQualification` |
| `Qualified` sin calificación | `MissingQualification` |
| Propósito declarado distinto del requerido | `QualificationMismatch` |

La familia requerida se compara antes de la presencia o propósito de la
calificación. Una etiqueta como `HearingEnd` no impone por sí sola una familia:
la exigencia de familia es explícita. Esta flexibilidad no certifica que una
resolución, notificación o resultado demuestre jurídicamente el propósito.

La calificación aquí implementada es temporal y declarativa. No es todavía el
contrato completo de calificación jurídica, duración ordenada, recepción,
representación o aplicabilidad necesario para activar un perfil de plazos.

## Captura conservada y límites de confianza

`TriggerExtraction` permite consultar `requirement()`, `selection()`, `source()`
y `outcome()`. Tras resolver una fuente conocida, conserva su captura incluso
cuando la exigencia semántica produce un bloqueo.

`TriggerSourceSnapshot` contiene expediente, referencia exacta, huellas,
procedencia y acuerdo seleccionado. La ausencia de acuerdo es distinta de un
acuerdo cuyo UUID es cero; si se selecciona este último, debe existir exactamente
en la revisión. El efecto declarado completo, incluido su texto y localizador,
se copia únicamente cuando se consume `NotificationStatedEffectAt` y existe.

Para hechos se capturan huellas de valores, fuentes y envío; para resultados de
audiencia, valores y envío. **El dominio conserva las huellas recibidas, pero no
las verifica**. No recalcula hashes, valida recibos, abre soportes ni comprueba
persistencia. La procedencia copiada es una declaración y no prueba autenticidad.

La aplicación que use esta API debe resolver las revisiones, verificar valores,
huellas y recibos, comprobar autorización y elegibilidad, y vincular el calendario
aplicable. Los estados de fuente y sus cabezas actuales no forman parte de esta
API pura. Repetir una extracción histórica no depende de una modificación o
retiro posterior; eso no autoriza usar automáticamente esa fuente para una nueva
evaluación operativa.

## Coordinación con la aritmética

El coordinador conserva la regla explícita y la extracción completa. Si la
extracción produce `Extracted`, llama a la [aritmética temporal](deadline-arithmetic.md)
con ese mismo tiempo, incluso cuando es `Unknown`. En ese caso existe un resultado
aritmético bloqueado por `UnknownAnchor`. Si la extracción está bloqueada, no
invoca la aritmética y `arithmetic()` devuelve `None`.

El calendario se pasa solo a la aritmética: puede cambiar su candidata o bloqueo,
pero no cambia la selección, procedencia ni extracción. Su ausencia no impide
extraer un tiempo; la regla aritmética determina si después es necesario. Los
errores de integridad se propagan sin fabricar una extracción o candidata válida.

No se eligen cantidades, inclusión, ajustes, calendarios o unidades a partir del
texto. Se conserva el ancla original, también cuando las horas producen una
candidata UTC o los meses bloquean por falta de homólogo. La fecha o el instante
resultante siguen siendo matemáticos, sin estados activo, atendido o vencido.

## Pruebas y trabajo pendiente

Los contratos comprobables están en las pruebas de
[contrato](../crates/domain/tests/deadline_trigger_contract.rs),
[integridad](../crates/domain/tests/deadline_trigger_integrity.rs),
[campos](../crates/domain/tests/deadline_trigger_fields.rs),
[audiencias](../crates/domain/tests/deadline_trigger_hearings.rs),
[calificación](../crates/domain/tests/deadline_trigger_qualification.rs) y
[coordinación](../crates/domain/tests/deadline_trigger_arithmetic.rs).
Los resultados de campañas se registran por separado en
[verification-report.md](verification-report.md).

Permanecen pendientes los perfiles normativos aplicables, persistencia de
calificaciones y evaluaciones, autorización al confirmar su escritura, seguimiento de
atención, reevaluación durable, entrega de alertas, HTTP y Qadra del módulo de
plazos. La [investigación normativa](deadline-rule-research.md) conserva los
extremos aún no resueltos. Este componente no sustituye el cálculo automático
exigido por el TT por fechas manuales ni declara cubierto su alcance completo.

La [resolución de insumos](deadline-inputs.md) implementa la lectura autorizada
en una transacción, los recibos y la observación separada de cabezas; su resultado
en memoria no constituye una evaluación operativa persistida.
