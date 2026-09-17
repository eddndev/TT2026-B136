# ADR 0033: Extracción temporal vinculada a fuentes exactas

## Status

Accepted. La decisión y el componente puro de dominio están implementados;
la verificación integrada está aprobada localmente. Los perfiles normativos, la evaluación
persistente y el flujo operativo de plazos permanecen pendientes.

## Context

La [aritmética explícita](0032-explicit-deadline-arithmetic.md) recibe operandos
válidos sin decidir qué hecho origina el cálculo. Resoluciones, notificaciones
y resultados de audiencia contienen tiempos con significados y precisiones
distintos. Elegir otro campo cuando falta el solicitado produciría un inicio
no declarado, aunque la suma matemática fuera correcta.

Los [hechos declarados](0031-declared-procedural-facts.md) conservan raíces y
revisiones exactas. El inicio ordenado de un periodo y la terminación de una
audiencia necesitan una declaración temporal identificable; no se obtienen de
un resumen, del estado `Concluded` ni de la hora genérica de sesión. Las fuentes
históricas deben poder reproducirse sin depender de sus cabezas actuales.

## Decision

Introducir `domain::deadline_triggers`, con selección de expediente y fuente
exacta, material explícito y una exigencia de campo o calificación temporal.
Resolver coherencia de expediente, identidades, revisiones, padre y acuerdo
antes de permitir que un bloqueo semántico oculte una discrepancia del material.
Distinguir los errores de integridad de los bloqueos por información insuficiente
o por exigencias incompatibles.

Conservar separadamente campo ausente y tiempo expresamente desconocido.
Transformar tiempos de audiencia manteniendo los componentes originales:
fecha sigue siendo fecha, e instante conserva segundos y desfase. No completar
componentes ni interpretar el texto o el estado de celebración como otro tiempo.

La calificación declara propósito, tiempo, texto y localizador sobre la misma
fuente seleccionada. El propósito no decide su familia ni acredita efecto
jurídico. Copiar selección y procedencia relevante para que cambios posteriores
a la entrada no modifiquen el resultado ya obtenido.

Capturar las huellas suministradas sin verificarlas en este componente. Resolver
existencia, recibos, hashes, permisos, elegibilidad e implicaciones de corrección
o retiro corresponde a la aplicación y a la futura política del evaluador.
No acoplar el dominio a snapshots de aplicación ni a servicios de persistencia.

Coordinar la extracción con la aritmética existente usando regla y calendario
explícitos. Una extracción bloqueada no inicia cálculo; un tiempo extraído
`Unknown` llega intacto a la aritmética y conserva su bloqueo correspondiente.
El calendario solo afecta la operación matemática. El contrato está en
[deadline-triggers.md](../deadline-triggers.md).

## Consequences

Las pruebas pueden distinguir errores de material, ausencias y bloqueos, y
comprobar referencias, textos, precisión y resultados aritméticos sin servicios
externos. El componente no consulta reloj, base de datos ni estado global.
No modifica los cánones de hechos, resultados o calendarios.

La extracción histórica es reproducible, pero construir sus tipos no demuestra
que una fuente sea auténtica, autorizada o utilizable para un cálculo operativo.
Se necesita una capa que valide esas condiciones y persista el conjunto exacto
de insumos y su evaluación con auditoría.

Continúan pendientes calificación jurídica completa, perfiles con corpus de
aplicabilidad, reevaluación durable, responsables, atención, alertas e interfaz.
La [investigación normativa](../deadline-rule-research.md) conserva sus límites;
un candidato aritmético no establece vencimiento ni sustituye criterios del TT.
