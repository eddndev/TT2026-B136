# 0035. Perfiles versionados y evaluaciones explicitas de plazos

## Status

Accepted. La implementación del flujo persistente está en curso. Su aceptación
completa se describe en `docs/deadline-lifecycle.md`.

## Context

Los actos procesales, sus revisiones y los calendarios ya conservan evidencia
exacta. La extracción temporal y la aritmética de días, meses civiles y horas
no determinan por sí mismas qué norma corresponde a un expediente. Para activar
un plazo hacen falta una configuración identificable, una declaración de
aplicabilidad y suficiente precisión sobre el instante de vencimiento.

La duración ordenada puede diferir del máximo permitido. Una fecha civil no
identifica una hora de corte. Un cambio posterior de fuente no debe sustituir
los insumos de una evaluación histórica. Las consultas del catálogo tampoco
pueden divulgar una determinación particular a través de su colección global.

## Decision

### Configuración y aplicabilidad

El Owner publica perfiles con ámbito global declarado o restringido a un
expediente. El ámbito completo es inmutable desde la primera revisión. Reemplazo
y retiro requieren revisión esperada y motivo; el retiro es terminal y conserva
la definición y el algoritmo anteriores. El catálogo global excluye perfiles
privados incluso para Owner. Una consulta de expediente autoriza ese expediente
antes de incluir sus perfiles y los globales disponibles.

Cada definición conserva referencias públicas, requisito temporal, plantilla de
regla, política de finalización, condiciones y ejemplos reproducibles. La
plantilla fija y la cantidad ordenada son variantes distintas. Una cantidad
faltante produce bloqueo; no se sustituye por el máximo ni se recorta el exceso.
El catálogo admite configuraciones finitas del motor, no código o expresiones
suministradas por operadores.

La definición identifica la versión de algoritmo. V1 conserva la semántica
actual de extracción, conteo y aritmética. Una implementación futura de otro
algoritmo debe preservar los lectores y el comportamiento de V1 para su
historia. La reproducción de ejemplos demuestra correspondencia matemática con
la configuración; no certifica su interpretación jurídica ni el contenido de
una referencia pública.

La evaluación del expediente recibe declaraciones explícitas sobre ámbito,
condiciones e incidencias pendientes. Conserva el cálculo parcial aunque una
declaración impida activarlo. Verifica primero la integridad de los insumos:
un bloqueo semántico no puede ocultar otra identidad, un recibo alterado o un
calendario distinto. Condiciones duplicadas o ajenas al perfil son solicitudes
inválidas; condiciones ausentes, desconocidas o rechazadas son bloqueos legibles.

### Fecha candidata e instante final

El perfil horario usa segundo y desfase explícitos del ancla. Un perfil civil
puede conservar una candidata sin hora o aplicar un corte declarado con hora,
desfase propio, canal, referencia y cobertura inclusiva acotada. El desfase del
corte no se hereda del acto inicial. Ningún bloqueo permite producir un instante
operativo ni programar avisos relativos a él.

El cierre administrativo permite consultar la historia y bloquea mutaciones
manuales de perfiles privados. No constituye una suspensión jurídica. Tampoco
el retiro de una fuente determina por sí solo la invalidez de una evaluación.
La revisión de aplicabilidad y los efectos de nuevas fuentes deben permanecer
explícitos en el flujo de seguimiento.

### Persistencia y frontera de validación

DPRF1 conserva la definición completa, incluidos ejemplos y calendarios. DPTX1
vincula actor, operación, identidad del perfil, acción, revisión esperada,
algoritmo, huella DPRF y motivo. Los recibos son comprobaciones de integridad;
no acreditan una autoridad judicial externa.

El servicio reproduce el corpus y verifica declaraciones tipificadas. El
adaptador revalida identidad activa, permisos, colección, cierre, cabeza y
preparación dentro de la transacción auditable. Revisión, recibo, evento de
cambio y auditoría se confirman o revierten juntos. El reloj de captura se lee
dentro de esa transacción; preparar no reserva una operación.

PostgreSQL verifica tamaño, encabezado y proyección de ámbito, huellas,
estructura del recibo, autor, secuencia, retiro e inmutabilidad. No duplica el
motor aritmético de Rust. La lectura completa y el inventario de arranque usan
el decodificador estricto y reproducen el corpus de la versión almacenada.
Las listas e historias leen proyecciones derivadas del prefijo DPRF y recibos
ligeros; no cargan varios megabytes de ejemplos por cada fila. Una proyección
ligera no constituye por sí sola una reproducción completa del perfil. Antes
de calcular un plazo se resuelve y valida la definición completa exacta.

Un resolvedor interno recibe la transacción del adaptador que confirma la
evaluación. No abre otra lectura pública auditable dentro de ella. La futura
revisión persistente del plazo debe vincular perfil y algoritmo exactos,
calificación, selección, materiales y cabezas observadas, resultado, traza y
estado de seguimiento; almacenar sólo la fecha calculada sería insuficiente.

### Cambios durables

Las inserciones de revisiones emiten eventos bajo el mismo bloqueo de auditoría.
La secuencia se asigna después de adquirirlo; se admiten huecos por rollback,
pero no confirmaciones tardías detrás de un cursor consumido. El mismo registro
de eventos incluye cambios de perfiles, con operación y expediente exactos.
Los eventos son insumos para la reevaluación, no una autorización para inferir
condiciones jurídicas desde texto ni reemplazar evidencia histórica.

## Consequences

- Los datos insuficientes producen explicaciones reproducibles y visibles.
- La historia conserva cantidades, precisión y política originalmente usadas.
- Los catálogos privados requieren autorización antes de filtrar o paginar.
- Las consultas ligeras evitan cargar corpus completos; la validación matemática
  completa sigue siendo obligatoria al usar una definición para calcular.
- Agregar algoritmos requiere compatibilidad histórica explícita y pruebas.
- El ciclo operativo permanece incompleto hasta incorporar plazos persistentes,
  trabajos recuperables, avisos y sus flujos HTTP y Qadra.
