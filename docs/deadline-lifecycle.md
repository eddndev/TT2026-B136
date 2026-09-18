# Perfiles, evaluaciones y seguimiento de plazos

Estado: implementación parcial del seguimiento completo. La resolución
autorizada de insumos, el catálogo de perfiles con API, el evaluador, los eventos
de cambio y el [registro persistente de plazos](deadline-records.md) están
implementados. El backend y la [API operativa](deadlines-api.md) están integrados
en `main`, con evaluación histórica, responsable, atención, corrección y retiro
auditable. La ampliación Qadra permite esas operaciones y la selección paginada
de responsables elegibles, con campañas locales completas aprobadas. El
[informe de verificación](verification-report.md) separa sus resultados de la
comprobación remota y de la integración de cada entrega.

Los trabajadores de activación y reevaluación, la agenda conjunta y las alertas
descritos más abajo siguen siendo objetivos de implementación. Tener eventos
inmutables no demuestra que se procesen, y guardar un vencimiento no demuestra
que se haya entregado un aviso. El cierre de perfiles jurídicos con fuentes
primarias y aceptación del supuesto también permanece pendiente; los ejemplos
sintéticos verifican mecanismos, no aplicabilidad jurídica. Véanse la
[matriz funcional](product-completion.md), los [insumos](deadline-inputs.md) y
las [fronteras de las reglas](deadline-rule-research.md).

## Resultado para el usuario

Desde una resolución, notificación o resultado de audiencia, el litigante
selecciona un perfil disponible, declara su aplicabilidad, completa los datos
requeridos y asigna un responsable. El sistema calcula y conserva una evaluación
reproducible. Los datos completos permiten activar el vencimiento sin introducir
su fecha manualmente. Los datos insuficientes conservan el cálculo parcial y
explican qué falta. La historia permite consultar tanto las evaluaciones como
la actuación registrada y los avisos.

La agenda debe reunir audiencias y vencimientos de los expedientes autorizados
en vistas diaria, semanal y mensual. Las alertas incluyen anticipaciones
configurables de 48 y 24 horas, audiencias próximas, cambios que afectan un
vencimiento inferior a 48 horas y vencimiento sin actuación registrada. El
seguimiento del tablero incluye ventanas de 48 horas y siete días.

## Perfiles configurables y versionados

El catálogo publica combinaciones finitas del motor de días, meses civiles y
horas; no ejecuta expresiones suministradas por el usuario. Cada definición
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

Las lecturas conservan historia y evidencia de expedientes cerrados. El cierre
organizativo bloquea las decisiones manuales sujetas a esa política, pero no
suspende por sí mismo términos ni borra alertas o reevaluaciones. Los trabajadores
tienen identidad técnica propia; no aparentan actuar como el publicador de una
fuente. La baja o revocación de una cuenta impide futuras entregas protegidas.

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
avanza su cursor en una transacción. El trabajador confirma nueva evaluación,
resultado del trabajo y sustitución de alertas pendientes juntos. Conserva
responsable y atención vigentes; no los reinicia al recalcular. Las correcciones
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
