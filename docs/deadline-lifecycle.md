# Perfiles, evaluaciones y seguimiento de plazos

Estado: seguimiento persistente, reevaluación, Agenda y alertas integrados en
`main` y conservados en la base productiva `3794181`, versión `v0.1.5`.
La resolución autorizada de insumos, el catálogo de perfiles con API, el
evaluador, los eventos de cambio y el
[registro persistente de plazos](deadline-records.md) conservan evaluación
histórica, responsable, atención, corrección y retiro auditable. La
[API operativa](deadlines-api.md) y Qadra permiten esas operaciones y la
selección paginada de responsables elegibles.

El [despachador](deadline-dispatch.md), el
[consumidor local](deadline-worker.md) y su composición en `serve` están
integrados por PR34, con expansión de eventos, confirmación técnica durable,
políticas explícitas e historia humana y técnica. La
[API de seguimiento](deadline-tracking-api.md) y Qadra V2 separan la captura
histórica de su proyección operativa comprobada. La aceptación incluyó
reevaluación, parada TERM/INT, reinicios y restauración; el
[informe de verificación](verification-report.md) conserva los resultados y
gates de cada entrega.

La agenda conjunta está integrada por PR35 y las alertas durables, preferencias,
bandeja Qadra y consumidor de correo por PR36, con aceptación propia. La
creación explícita conjunta de resultado y plazo derivado también está
integrada; no equivale a inferir términos automáticamente desde texto.
Guardar un vencimiento o mostrarlo en Agenda no demuestra entrega de avisos.

El corpus finito de ocho supuestos ya tiene fuentes, condiciones y resultados
independientes documentados. Su incremento `AtAnchor` y cierre civil semanal
está implementado localmente y aceptado por HTTP y navegador, todavía sin
integrar ni desplegar. La regresión API completa aprobó en 606.453 s; CI e integración
por PR permanecen pendientes. Véanse la
[lista de los ocho supuestos](four-front-closure.md), la
[matriz funcional](product-completion.md), los [insumos](deadline-inputs.md) y
las [fronteras de las reglas](deadline-rule-research.md).

## Resultado para el usuario

Desde una resolución, notificación o resultado de audiencia, el litigante
selecciona un perfil disponible, declara su aplicabilidad, completa los datos
requeridos y asigna un responsable. El sistema calcula y conserva una evaluación
reproducible. Una evaluación aceptada con instante y dependencias vigentes
permite consultar el vencimiento operativo sin introducir su fecha manualmente.
Los datos insuficientes conservan el cálculo parcial y explican qué falta. La
historia permite consultar evaluaciones, atención declarada y los avisos con
su origen capturado. La creación del plazo es explícita; no se deducen
aplicabilidad ni términos nuevos de un texto o del estado de una audiencia.

La agenda reúne audiencias y vencimientos de los expedientes autorizados en
vistas diaria, semanal y mensual, además de un rango personalizado. Las
[alertas integradas](alerts-api.md) incluyen anticipaciones configurables, con
valores iniciales de 48 y 24 horas, audiencias próximas, cambio de fecha próxima,
revisión requerida y vencimiento sin atención declarada. El
[tablero integrado](dashboard-api.md) incluye ventanas de 48 horas y siete días
sobre vencimientos operativos comprobados. Cada mecanismo conserva su evidencia
y contrato; una alerta leída no acredita atención del plazo.

## Perfiles configurables y versionados

El catálogo publica combinaciones finitas del motor de días, meses civiles y
horas; el incremento local añade el instante del ancla mediante `AtAnchor`.
No ejecuta expresiones suministradas por el usuario. Cada definición
conserva nombre, descripción, ámbito global declarado o expediente restringido,
referencias públicas estructuradas, campo temporal requerido, regla de cantidad,
política de finalización, condiciones de aplicabilidad y ejemplos esperados.
Se reutilizan los valores de ámbito y referencia del catálogo de calendarios;
su uso no concede acceso a expedientes ni certifica el contenido de una URL.

Owner publica, reemplaza y retira perfiles mediante revisiones y recibos. El
catálogo global excluye el contenido de perfiles restringidos. Una determinación
particular debe quedar en la calificación o en el perfil del expediente. El
litigante asignado declara las condiciones aplicables, sus fuentes y su motivo;
el servidor vincula la declaración con la revisión exacta del perfil y del acto.
Una publicación hace disponible una configuración del despacho: su integridad y
reproducción matemática no acreditan por sí solas una interpretación jurídica.

La cantidad fija y la duración ordenada son variantes distintas. La segunda
requiere una cantidad positiva propia, con máximo opcional independiente. La
cantidad ausente no se rellena con el máximo y el exceso no se recorta. Las
políticas de inclusión, días computables y tratamiento final se conservan sin
conversiones de unidad. El algoritmo mensual disponible desplaza directamente
al homólogo y bloquea su ausencia. Una política distinta exige implementación
y evidencia propias; una fecha judicial aislada no permite deducir la fórmula.

`AtAnchor` no admite cantidad ni políticas civiles. Se limita a una regla fija
con fin calificado de resultado de audiencia y finalización por instante
aritmético. Exige segundo y desfase declarados; ni la hora programada ni el
estado concluido sustituyen ese fin exacto. No agrega días ni horas.

Cada definición tiene de una a dieciséis referencias, condiciones y ejemplos,
con identificadores distintos dentro de su colección. UUID cero es un valor,
no ausencia. Cada condición y ejemplo señala referencias existentes y distintas.
El orden declarado se conserva. La publicación reproduce todos los ejemplos y
exige al menos una candidata matemática; un conjunto compuesto exclusivamente
por bloqueos no demuestra que el perfil pueda calcular. Los ejemplos conservan
su calendario completo incluso cuando la operación no lo consume.

El corpus valida la aritmética y conserva su procedencia. La cobertura temporal
del corte se comprueba sobre la evaluación del expediente; un ejemplo aritmético
puede producir una candidata fuera de esa cobertura sin invalidar su suma.
Las pruebas sintéticas del modelo se identifican como tales y no sustituyen
los casos de aceptación con fundamento del perfil publicado.

## Fecha candidata e instante operativo

Un perfil horario requiere segundo y desfase del ancla y produce un instante
UTC. Los perfiles civiles pueden conservar únicamente una candidata o aplicar
un corte declarado con canal, segundo exacto, desfase propio, cobertura civil y
referencia. Ese desfase no se hereda del acto inicial. Un corte fijo sólo vale
dentro de su cobertura declarada, inclusiva y acotada a 1096 fechas.

El incremento local de corte semanal conserva canal, desfase, cobertura y
referencia, con siete horas opcionales de lunes a domingo. Selecciona la hora
del día de la candidata sin desplazar esa fecha. Un día sin hora o fuera de
cobertura conserva el bloqueo y no produce un vencimiento operativo. En
`AtAnchor`, resultado y traza preservan el desfase original del fin declarado;
el vencimiento operativo usa UTC, como los demás plazos.

Una candidata sin corte suficiente se conserva como fecha civil. No se fabrican
medianoche, segundos de una precisión de minuto ni desplazamientos de zona.
Sin un instante final suficientemente determinado no se programan las alertas
relativas de 24 o 48 horas. La interfaz debe distinguir este bloqueo de un
cálculo matemático imposible y mostrar los datos que permitirían resolverlo.

## Evaluación y seguimiento persistentes

Un plazo tiene identidad estable y expediente inmutable. Cada revisión conserva
perfil y algoritmo exactos, calificación, selección temporal, cantidad ordenada,
fuentes y calendarios exactos, cabezas observadas, resultado, traza, bloqueos,
responsable, atención, causa, autor, captura y recibo. Las dependencias tienen
referencias tipificadas al mismo expediente; los calendarios globales conservan
su identidad independiente. Se separan el estado del cálculo, la atención y el
vencimiento derivado del reloj. Registrar atención no acredita una presentación
válida ni borra el hecho de que se haya registrado después de vencer.

La preparación no reserva una operación. El commit revalida identidad, rol,
pertenencia, revisión esperada, fuentes, perfil y resultado bajo la transacción
auditable común. Un resolvedor interno recibe esa transacción: no llama al puerto
público de insumos desde otro commit. Revisión, dependencias, recibo y auditoría
se confirman juntos. Una cabeza nueva produce conflicto o una nueva evaluación;
una modificación de contenido histórico produce inconsistencia, nunca sustitución.

La lectura actual y el listado comparan las cabezas verificadas con las
observaciones capturadas dentro de una transacción auditada. Conservan el
cálculo histórico y distinguen su fecha de un vencimiento operativo vigente.
No deducen frescura del estado de la cola ni de un resultado del consumidor.
Una consulta exacta de revisión conserva evidencia histórica y no comprueba
las cabezas actuales; las reglas están en el [contrato de seguimiento](deadline-tracking-api.md).

Las lecturas conservan historia y evidencia de expedientes cerrados. El cierre
organizativo bloquea las decisiones manuales sujetas a esa política, pero no
suspende por sí mismo términos ni borra alertas o reevaluaciones. Los trabajadores
tienen identidad técnica propia; no aparentan actuar como el publicador de una
fuente. La baja o revocación de una cuenta impide futuras entregas protegidas.

## Agenda autorizada de actividades

La [API de agenda](agenda-api.md) reúne ambas familias bajo una lectura auditada
que revalida actor, membresías y administración. Sólo incluye plazos V2 activos,
aceptados y con un instante operativo cuya vigencia se comprueba en esa lectura.
La fecha histórica sirve para localizar candidatos, pero no basta para mostrar
un vencimiento. Los plazos pendientes, cambiados, bloqueados, legados o retirados
siguen consultándose dentro del expediente. Atención declarada y cierre
administrativo conservan las lecturas autorizadas y no eliminan por sí solos
una actividad vigente.

Qadra convierte el periodo civil y su desfase explícito en un intervalo UTC,
conserva las fechas originales de las audiencias y muestra listas o calendarios.
Acumula páginas y conserva la revisión mayor de cada par familia/identificador.
La continuación avanza incluso cuando una página no contiene actividades;
mientras exista, la consulta permanece parcial. Cada página observa el estado
actual, sin reservar una instantánea entre peticiones. Cambiar filtros invalida
respuestas anteriores y actualizar reinicia la consulta.

Abrir una actividad revalida el expediente antes de consultar la revisión exacta
seleccionada. Esta implementación y sus límites están en
[ADR-0038](adr/0038-authorized-combined-agenda.md); su aceptación e integración
están confirmadas.
La agenda no recalcula fechas, activa términos ni entrega alertas.

## Cambios y reevaluación sin pérdida

Cada revisión de resolución, notificación, resultado o calendario agrega un
evento en la misma transacción mediante un trigger de inserción. El evento fija
familia, raíz, revisión, expediente cuando corresponda y operación causante.
Referencias tipificadas y una comprobación de operación impiden asociarlo a otra
fuente. Los eventos son inmutables. Su secuencia se asigna después de adquirir
el bloqueo común; un rollback puede dejar huecos, pero una transacción tardía no
puede confirmar un evento detrás de un cursor ya consumido.

No se exige inventar eventos anteriores a la instalación. El alta de un plazo
observa las cabezas vigentes bajo el mismo bloqueo y registra sus dependencias;
todo cambio posterior emite un evento durable. El inventario valida los eventos
existentes, los triggers y la secuencia; la restauración conserva esos datos.

El despachador pagina dependencias, inserta trabajos únicos por evento/plazo y
avanza su cursor en una transacción. El consumidor local confirma revisión,
resultado del trabajo y auditoría juntos, o registra la causa de no producir una
revisión. El subsistema integrado de alertas conserva los avisos históricos y
resuelve los superados según su [contrato](alerts-api.md). El consumidor
mantiene responsable y atención vigentes; no los reinicia
al recalcular. Las correcciones
de fuente no autorizan inferir nuevamente condiciones jurídicas desde texto.
La revisión de aplicabilidad pendiente debe quedar visible, con el cálculo
anterior conservado. Los cambios de calendario aplicable se evalúan con su nueva
revisión y dejan trazabilidad del resultado anterior y del nuevo.

La entrega de correo ocurre fuera del bloqueo. Sus intentos conservan estado,
identidad del proveedor y resultados inciertos; la ausencia de duplicados exige
idempotencia o conciliación. Leer una alerta interna no equivale a atender el
plazo. Las consultas transversales, agregados e informes filtran por autorización
en PostgreSQL antes de paginar o calcular totales.

## Comprobación del flujo completo

La aceptación incluye días naturales y computables, meses y horas, precisión
insuficiente, cantidad ordenada frente a máximo, homólogo ausente, calendarios
incompletos, corte fuera de cobertura y cambios de dependencias. Debe demostrar
concurrencia, rollback, reanudación tras caída, restauración y entrega sin
duplicados; también aislamiento entre expedientes, revocación y cierre.

Los recorridos de navegador deben cubrir captura desde el acto, cálculo,
confirmación, conflicto, respuesta incierta, historia, actuación, cambio de
calendario, agenda conjunta de al menos cinco expedientes y avisos internos.
Se conservan tipografía, colores, componentes y comportamiento móvil de Qadra.
El catálogo y el cómputo parcial no bastan para declarar completos esos flujos.

## Corpus acotado y aceptación local

El corpus contiene investigación complementaria, su prórroga, revocación oral,
revocación escrita y las cuatro apelaciones de tres, cinco, tres y diez días
del [catálogo definitivo](four-front-closure.md). Las fichas de
[investigación](legal-corpus-investigation.md),
[recursos](legal-corpus-resources.md) y
[calendario y canales](legal-corpus-calendar.md) fijan fuentes primarias,
versiones, ámbito penal local de CDMX, entradas ficticias identificadas y
resultados independientes. No amplían el catálogo ni requieren un expediente
publicado idéntico o revisión externa obligatoria.

Las dos determinaciones mensuales declaran duración efectiva, inicio y método
de fecha a fecha. El cierre a las 18:00 -06:00 pertenece a la orden ficticia
expresa; no se atribuye al CNPP ni al horario de ventanilla. La prórroga exige
concesión y acumulado expresamente calificados. Las cinco fichas por días usan
efectos de notificación personal y cierre semanal de ventanilla UGJ. La ficha
oral conserva el fin exacto de la audiencia del 09-10-2026. Una condición
desconocida, rechazada o ajena a estas fichas bloquea el vencimiento operativo.

La aceptación HTTP con servicios reales aprobó captura, reinicio y restauración
de los ocho supuestos, el extremo escrito que cruza inhábiles y diez negativos.
El recorrido Qadra con backend real aprobó en 1440 y 390 píxeles sin fallos.
Esta evidencia corresponde al incremento local de `AtAnchor`, cierre semanal
y perfiles del corpus. La regresión API completa aprobó en 606.453 s, incluido
el historial de reevaluación, reinicio y restauración. Quedan pendientes los
gates de CI, integración por PR y despliegue de ese incremento.
