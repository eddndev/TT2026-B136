# Alcance de audiencias y medidas cautelares declaradas

Estado: **dominio y servicios de aplicación verificados localmente**. El trabajo local
comprende esos componentes y sus contratos. Los adaptadores PostgreSQL conservan
convocatorias de imposición y revisión, sustitución de programación y cancelación, así como
decisiones con historia exacta y ausencia de cambios. Su alcance
se detalla abajo e incluye anclas iniciales ordinarias exactas. La persistencia
con anclas cautelares, G2/M2 y registros
administrativos, HTTP, Agenda, alertas e interfaz
conservan sus propias comprobaciones pendientes. Este contrato no
acredita la implementación completa ni la aceptación del flujo de producto. La decisión de arquitectura se conserva
en [ADR-0071](adr/0071-declared-precautionary-hearings-and-measures.md).

## Persistencia local de convocatorias cautelares

`PostgresPrecautionaryHearingStore` implementa los puertos existentes de comandos
y lecturas mediante PostgreSQL. Las migraciones `0033_` conservan una raíz y
revisiones inmutables con procedencia exacta, autor y rol históricos, reloj UTC
con nanosegundos, soporte admitido y referencia a su evento de auditoría. Cada
consulta reconstruye el prefijo completo, limitado a 256 revisiones, y compara
sus valores y compromisos; un digest almacenado no sustituye esa comprobación.
La autorización vigente precede a la consulta de operaciones. Un reintento exacto
conserva su captura original incluso después del cierre administrativo.

El arranque valida catálogo, permisos e inventario. El runtime sólo puede leer
y anexar columnas explícitas; no puede actualizar o borrar capturas ni alterar
el esquema. Las escrituras y la auditoría comparten transacción y bloqueo. Una
operación rechazada no deja raíces, revisiones ni eventos parciales. La evidencia
de auditoría impide reutilizar una identidad tras perder sus registros y evita
presentar una revisión anterior o una página vacía como estado actual válido.

La migración `0036_precautionary_hearing_review.sql` admite objetivos `review`
por identidad, revisión y digest exactos. El cargador recupera todos sus grupos
propietarios y ancestros en la misma transacción, incluidos hermanos no
seleccionados. Un prefijo puede conservar revisiones distintas de una medida;
no sustituye las selecciones antiguas por su cabeza actual. También admite
capturas terminales, sin reactivarlas ni atribuirles efectos jurídicos nuevos.

Cada revisión se reconstruye con su cierre exacto; la historia devuelve la unión
del prefijo seleccionado. Una lectura o replay de la primera captura no incorpora
decisiones posteriores ajenas. La cancelación conserva objetivos, fuentes y
contexto de programación anteriores. Los límites son independientes: hasta 256
capturas de audiencia y 8192 objetivos; hasta 256 grupos y 8192 miembros en la
historia de medidas. No se cargan grupos cuando no existen objetivos.

La existencia del adaptador no habilita rutas, Agenda, alertas o Qadra, ni acredita
restauración integral o despliegue. Véase [operación de base de datos](database-operations.md).

## Persistencia local de decisiones cautelares

`PostgresMeasureDecisionStore` admite grupos de 1 a 32 imposiciones iniciales y
decisiones explícitas NoMeasureChange. Estas últimas tienen decisión, propietario
de grupo y auditoría reales, sin filas de medidas. Las migraciones `0034_` guardan
operaciones, decisiones, raíces y revisiones iniciales en cuatro tablas inmutables.
Las migraciones `0035_` amplían las guardas para confirmar, modificar, revocar,
cesar y sustituir medidas desde predecesores exactos. Las raíces conservan su
propietario inicial; cada grupo retiene todos sus miembros y ancestros, incluso
los de hermanos no seleccionados. Una captura nueva exige la cabeza vigente;
lectura y replay reconstruyen la historia original. Las migraciones `0037_`
añaden anclas iniciales ordinarias exactas: se comprueban raíz, prefijo completo
hasta la revisión elegida, fuentes, recibos y auditoría original. Se conservan
revisiones antiguas y canceladas, sin exigir que sean cabeza vigente. El prefijo
seleccionado admite hasta 256 revisiones. Las anclas cautelares, G2/M2 y
correcciones administrativas siguen pendientes.

La preparación admite el soporte exacto fuera del bloqueo; la confirmación exige
ambos digests y vuelve a comprobar principal, acceso, contexto y fuentes. Todas
las filas del grupo y un evento de auditoría se confirman juntos. Las lecturas
reconstruyen el grupo completo con las fuentes históricas y su origen `mg1`;
el replay autorizado conserva autoría y tiempo originales. Las listas paginan
decisiones inmutables, incluidas las que no cambian medidas, hasta 20 por página.

El arranque comprueba catálogo, privilegios e inventario. Las conexiones abiertas
también rechazan grupos incompletos y auditoría huérfana; perder filas no libera
sus identidades. Se conservan los canones existentes. La aceptación de restauración,
la integración HTTP, Agenda, alertas e interfaz siguen pendientes; este adaptador
no acredita el cierre del flujo completo.

## Frontera de la implementación local

El primer checkpoint implementa localmente valores de convocatoria: identidad propia,
operación, revisión positiva, propósito `imposition` o `review`, hora exacta,
modalidad, lugar, nota opcional, participantes con identidad/revisión exactas,
únicos y ordenados, y soporte exacto
con declaración y localizador obligatorios. Una revisión selecciona de una a 32
medidas por identidad, revisión y digest; una imposición no acepta objetivos de
revisión. `PHEAR1` debe comprometer todos esos valores sin cambiar `HEAR1` ni
`RHEAR1`. La verificación focal de dominio está aprobada; las comprobaciones
remotas y la integración siguen pendientes.

La ampliación local de valores comprende `MeasureKind`, `MeasureTime` y
`MeasureValidity`, con el compromiso `MVAL1`. Diez pruebas focales y Clippy
aprobaron tras el fallo inicial de las pruebas. Esta evidencia es independiente
de las once pruebas anteriores de convocatoria; la integración sigue pendiente.

La capa de aplicación valida ahora un contexto histórico completo (`PCTX1`):
administración observada, etapa y administración exacta que originó esa etapa,
con valores, soportes y procedencia. `PHTXN1` vincula autor y rol capturados,
expediente, operación, convocatoria, acción, revisiones, digest previo, contexto,
valores y motivo. Son comprobaciones y compromisos de datos; no acreditan acceso
vigente, admisión documental nueva ni una operación guardada.

La resolución local de participantes verifica de cero a 32 capturas exactas,
manuales o tipadas, con sus valores y sujeto histórico vinculado. Rechaza fuentes
ajenas, faltantes, duplicadas o alteradas y deriva las etiquetas de esas mismas
revisiones. Permite reconstruir capturas archivadas sin autorizar su selección
para una nueva operación.

La admisión local del soporte directo verifica una sola versión y digest, y
utiliza el procesador documental acotado para integridad y formato PDF/DOCX.
La pertenencia al expediente debe comprobarse en el almacén autorizado.
El soporte de resolución seleccionado por `MDVAL1` utiliza la misma frontera
interna de integridad y formato, conservando su referencia y metadatos exactos.

Los valores locales de medida (`MEAS1`) conservan sujeto exacto, clase,
condiciones, vigencia y supervisión: participante exacto con declaración, o
desconocimiento con motivo. Los datos factuales de decisión (`MDVAL1`) conservan
autoridad declarada, tiempo, justificación y soporte con localizador. Disponen
de identidad de decisión separada. El resultado `MEFX1` normaliza cambios
declarados o una declaración de ausencia de cambios. Rechaza identidades
repetidas en todos los lugares del grupo, incluidos ambos lados de sustitución,
y acota a 32 la unión de medidas anteriores y nuevas. Estos valores todavía no
resuelven predecesores ni confirman efectos o grupos atómicos.
La resolución local de fuentes comprueba el sujeto exacto y el supervisor
manual o tipado, cuando se declara. Deriva sus etiquetas, conserva revisiones
archivadas para historia y rechaza contradicciones de una misma ficha/revisión.
No certifica identidad civil, designación oficial ni permiso actual.

Los recibos locales `PHPR1`/`PHCR1` conservan los datos completos y procedencia
de la convocatoria de imposición. Validan digest, tiempo y reemplazo/cancelación
contra su predecesor exacto; rechazan copias contradictorias de una misma fuente
inmutable. Esta comprobación no prueba origen persistido, historial completo,
autorización actual ni admisión nueva. La convocatoria de revisión resuelve
capturas reales de medidas y todos sus grupos de origen. Reemplazo, cancelación
e historial validan la unión de referencias anteriores y nuevas; sus tiempos no
pueden preceder las fuentes exactas. Los accesos sin evidencia siguen rechazando
la revisión.

El validador local de historial exige una cadena desde la captura inicial,
comprueba todos los campos del origen y rechaza operaciones repetidas o fuentes
contradictorias aunque reaparezcan después de una revisión intermedia. La
existencia durable, el inventario y la cabeza vigente requieren al almacén; una
cadena suministrada válida no demuestra que no existan revisiones posteriores.

La captura local de una imposición inicial o una decisión explícita sin cambios
produce revisión, decisión, medidas y grupo completos. `MDTXN1` vincula la
instrucción; `MDPR1`, `MDCR1`, `MMCR1` y `MDGR1` conservan sus compromisos sin
ciclos. La reconstrucción exige todos los miembros, origen y fuentes exactas,
y tiempos de captura compatibles con su procedencia. La ampliación con historial
resuelve grupos de origen completos, incluidas las dependencias de medidas
hermanas, y admite confirmación, modificación, revocación, cese y sustitución
conjunta. Conserva origen y términos de los efectos que no los modifican, rechaza
revisiones repetidas y contradicciones, y exige un cierre exacto de dependencias
sin grupos ajenos. El límite técnico es de 256 grupos y 8192 filas de medidas,
incluido el candidato; superarlo rechaza evidencia incompleta sin truncarla.
Las anclas distinguen la revisión inicial ordinaria de la cautelar y conservan
capturas completas. Admiten referencias históricas programadas o canceladas sin
afirmar celebración. Las dependencias incluyen tanto medidas afectadas como
medidas citadas por el ancla; ambas selecciones pueden ser diferentes. `MHIA1`
conserva el detalle ordinario, y la cautelar conserva su `PHCR1` original. No
acredita persistencia, cabeza vigente, acceso ni efectos jurídicos.

El servicio local de convocatorias y sus consultas aprobaron la verificación focal.
Comprueba permisos del actor actual, admisión del soporte seleccionado, confirmación
de instrucción y revisión completa, y recuperación de la operación original.
Los puertos exigen al almacén volver a comprobar acceso, cabeza y material exactos
antes de guardar con auditoría. Las consultas conservan el prefijo histórico y
origen completos; admiten personal autorizado y rechazan Client. Todavía no hay
adaptador persistente ni ruta de producto para esta familia.

El servicio de decisiones también pasó la verificación focal: admite el soporte
exacto, exige ambos digests y conserva el grupo completo y sus ancestros al
recuperar o confirmar. Cubre todos los efectos declarados y las anclas admitidas;
la decisión y cada medida se validan juntas. No sustituye la revalidación durable
ni la transacción que debe implementar el adaptador.

Las consultas locales de decisiones validan el grupo original completo y sus
ancestros con permisos actuales. La lista conserva decisiones inmutables aunque
existan revisiones posteriores de sus medidas; pagina hasta 20 decisiones y
rechaza identidades o fuentes compartidas contradictorias.

La rectificación tiene ahora valores de dominio y formato `MCVAL1` verificados.
Permite corregir condiciones, declaración de vigencia, inicio, término ya
presente y texto de supervisión. Preserva sujeto exacto, clase, presencia de
terminación y selección/tipo de supervisor. Rechaza cambios vacíos; permite
componentes y precisión explícitos sin inventar información. Los valores por sí
solos no crean una revisión ni un efecto judicial; toda decisión histórica queda
inmutable.

La primera captura administrativa local rectifica una medida judicial exacta y
exige su grupo completo con todos sus ancestros. Agrega una sola revisión con
recibo propio, sin inventar otra decisión o grupo judicial. Conserva origen,
última acción, fuentes completas y referencia exacta a la última medida/grupo
judiciales. El soporte pertenece a esa última declaración real: después de una
modificación no se sustituye por el de la imposición inicial. La corrección de
una declaración terminal conserva su acción terminal.

`MATXN1` vincula autor, expediente, operación, objetivo, contexto esperado, motivo
y acción; `Correct` usa la etiqueta 0 y valores `MCVAL1`, mientras
`MarkEnteredInError` usa la etiqueta 1 sin carga de valores. `MAPR1` conserva
instrucción, contexto, soporte y resultado completos;
`MARCR1` compromete la fila administrativa, procedencia y digest de revisión;
`MAGR1` reúne revisión y fila con sus digests y tiempo de captura sin ciclos.
Se preservan los canones judiciales anteriores. La reconstrucción exige una fila
exacta, fuentes inmutables compatibles y reloj UTC no anterior a su procedencia.
Los límites de 256 propietarios y 8192 filas incluyen la nueva captura.

El resolvedor mixto local acepta referencias exactas a registros judiciales o
administrativos y permite rectificaciones repetidas. Valida el inventario plano
completo, con operaciones y revisiones de medida únicas entre ambas familias;
rechaza dependencias faltantes, ajenas, sobrantes, cíclicas o contradictorias.
Descubre dependencias sin recursión entre propietarios judiciales G1/G2 y
administrativos; reconstruye una vez cada propietario completo después de sus
padres. Incluye las relaciones de efectos, objetivos administrativos y anclas
Review. El inventario de fuentes se comparte entre todos los propietarios.

La proyección comprobada conserva el tiempo/contexto/valores del registro exacto
seleccionado y, por separado, la última medida/grupo judiciales reales. Una
corrección repetida avanza desde su predecesora administrativa sin cambiar origen,
acción judicial ni soporte. Los límites combinados incluyen al candidato; los
selectores públicos admiten hasta 32 identidades. Las entradas anteriores usan
la misma validación sin copiar anticipadamente el historial ni cambiar sus bytes.

La declaración pura `MarkEnteredInError` agrega una revisión desde un registro
exacto con validez de captura Valid. Conserva valores, fuentes, proyección,
identidad, raíz, origen judicial, última medida/grupo judiciales, acción y soporte;
la validez de captura pasa a EnteredInError. Registra autor, contexto, motivo y
tiempo propios sin revocar, cesar, sustituir ni anular una decisión judicial.
También conserva las acciones judiciales terminales y no crea un reemplazo.

La preparación genérica `prepare_measure_administrative_record_with_history`
admite Correct y MarkEnteredInError. Las dos entradas anteriores de preparación
de corrección siguen limitadas a Correct y rechazan Mark. Ninguna acción admite
como predecesor un registro EnteredInError: no hay reactivación mediante otra
marca o corrección. Se mantienen los límites combinados, las fuentes exactas y
las comprobaciones de contexto y reloj desde el registro seleccionado.

Los formatos `MATXN1`, `MAPR1`, `MARCR1` y `MAGR1` conservan sus estructuras y
los bytes anteriores de Correct. El resolvedor mixto, los recibos y los orígenes
reconstruyen una marca válida como evidencia histórica aunque su registro esté
declarado EnteredInError; no alteran capturas anteriores ni sus referencias.

La preparación pura de convocatorias Review admite ahora registros judiciales
y administrativos exactos mediante
`prepare_precautionary_hearing_with_record_history`. Sus verificadores de recibo,
transición, origen e historial tienen entradas `_with_record_history`; conservan
los valores, estructuras y bytes anteriores de la convocatoria.

La entrada adicional `prepare_precautionary_hearing_with_decision_history`
acepta también M2 y correcciones posteriores a M2 con sus propietarios G2
reales. Su material conserva contexto observado, fuentes exactas, predecesor
opcional e historial prestado `MeasureDecisionRecordHistoryEvidence`. Los
verificadores de recibo, transición, origen e historial tienen versiones
`_with_decision_history`; las firmas públicas anteriores permanecen disponibles.
No se agregan instrucciones ni formatos: PHEAR1/PHTXN1/PHPR1/PHCR1 conservan sus
bytes, y entradas históricas equivalentes producen la misma captura.

Todo objetivo exacto debe tener validez de captura Valid, incluso en prueba
histórica o cancelación. Se rechaza una convocatoria que seleccione una revisión
ya declarada EnteredInError aunque se recalculen sus hashes. Una convocatoria
anterior que seleccionó una revisión Valid sigue siendo válida y cancelable tras
una marca posterior, con su cierre exacto original y sin consultar la cabeza
vigente. Las acciones judiciales terminales siguen siendo seleccionables; no se
confunden con la validez de captura.

El contexto y tiempo efectivos de una corrección seleccionada rigen las
comprobaciones de avance y procedencia de la convocatoria. No se sustituyen por
los de la última medida judicial retenida. Reemplazos e historiales validan la
unión de referencias exactas anteriores y nuevas, incluidas distintas revisiones
de una misma identidad, y comparan todas las fuentes compartidas de G, registros
administrativos y convocatorias. Cada convocatoria conserva su límite de 32
objetivos y de 32 participantes/fuentes y proyecciones.

La prueba con decisiones V2 usa un solo cierre compartido G1/G2/administrativo,
incluidas hermanas, relaciones de sustitución y anclas. Una C que conserva M2
aporta su contexto, tiempo y validez efectivos; M2 sigue siendo evidencia judicial
separada. Se comprueba nuevamente el predecesor completo antes de reemplazar o
cancelar. El historial puede conservar referencias M1/C/M2/C de la misma identidad
y una unión de objetivos vacía exige evidencia de propietarios vacía.

Las nuevas entradas de prueba mixta acotan a 256 las capturas de convocatoria y
a 8192 la unión de objetivos exactos, además del límite independiente de 256
propietarios G/administrativos y 8192 filas de medida. Rechazan material excesivo
antes de hashear o copiar; no cambian los límites de las entradas históricas
anteriores. Los servicios autorizados de convocatorias todavía no incorporan
este material mixto. Las entradas con historial de decisiones V2 mantienen los
mismos límites para las tres familias y las formas anidadas de sus anclas.

Las decisiones judiciales puras admiten ahora predecesores administrativos
mediante `prepare_measure_decision_with_record_history`, material V2 y un
inventario plano que agrega grupos G2 completos al historial G1/administrativo.
El resolvedor común conserva la familia real M1/M2 o C; nunca fabrica una medida
judicial para representar una corrección. Cada resultado M2 conserva raíz de
registro y origen judicial como datos separados.

Confirmar, revocar, cesar y sustituir como salida preservan términos corregidos
y fuentes completas. Modificar conserva sujeto exacto, clase, raíz y origen,
con nuevos términos y fuentes verificadas. Imponer y sustituir como entrada
crean identidades R1 con origen judicial nuevo. Las relaciones muchos-a-muchos
conservan sus conjuntos completos; NoMeasureChange produce decisión y grupo
sin medida. Los efectos rechazan predecesores EnteredInError y acciones
judiciales terminales; esto no cambia la selección de terminales para Review.

La nueva decisión avanza revisión, contexto y tiempo desde el registro efectivo
seleccionado. Las entradas administrativas `_with_decision_history` permiten
corregir o marcar después de M2 y conservan esa medida/grupo judiciales reales
como última declaración, con su soporte. La historia unificada comprueba
operaciones únicas entre familias, decisiones únicas entre G1/G2, propietarios
exactos de revisión, hermanas completas y fuentes inmutables compatibles.

`MDPR2`, `MMCR2` y `MDGR2` vinculan predecesores con etiqueta de familia, raíz
separada, resultados y recibo completo. Se mantienen `MDTXN1`, `MDCR1` y todos
los formatos anteriores judiciales, administrativos y de convocatoria. La entrada
V2 siempre emite V2; las anteriores conservan V1. Sus límites combinados siguen
siendo 256 propietarios y 8192 filas, incluido el candidato, con hasta 32
identidades afectadas y rechazo de exceso antes de hashear o copiar. Una
convocatoria Review sobre G1/C puede anclar G2 con su cierre exacto.

El inspector puro `inspect_measure_administrative_dependencies` valida un bosque
suministrado de propietarios G1/G2/administrativos y prefijos de convocatorias,
y devuelve los usos directos de una referencia exacta. Distingue predecesores
en efectos judiciales, objetivos de comandos administrativos, selecciones de
cada captura Review histórica y decisiones que anclan una captura Review exacta.
Incluye cancelaciones, revisiones reemplazadas y decisiones NoMeasureChange con
ancla; no convierte raíces, última evidencia judicial, fuentes compartidas o
revisiones distintas en nuevos usos del objetivo.

Comprueba una vez cada propietario completo, incluidos grupos sin medidas y
raíces desconectadas. Toda dependencia requerida, origen, familia, hermana y
fuente debe ser consistente. Las consultas anteriores de cierre exacto siguen
rechazando propietarios sobrantes. Cada audiencia suministrada tiene un único
prefijo no vacío desde R1 con su origen, y toda ancla cautelar debe coincidir con
la captura completa de su revisión dentro de ese prefijo. Los prefijos comparten
el índice comprobado y el inventario de fuentes del bosque; errores ajenos a la
ascendencia del objetivo también invalidan la inspección completa.

El objetivo inspeccionado puede ser terminal o EnteredInError sin que eso autorice
corregirlo. Los objetivos Review conservan la exigencia de validez exacta Valid.
El informe ordena y elimina solo usos idénticos después de validar; distingue
la selección por una convocatoria de su uso como ancla y no enumera descendientes
transitivos. Antes de hashear o copiar se acotan 256 propietarios y 8192 filas,
256 prefijos y 256 capturas totales, y 8192 ocurrencias de objetivos Review,
además de las formas individuales de hasta 32 elementos. Hay como máximo 768
usos directos, sin truncamiento ni nuevos digests o formatos.

Un informe vacío solo declara que no hay esos usos en el bosque suministrado.
No prueba que el inventario durable esté completo, que el objetivo sea la cabeza
vigente ni que exista permiso de modificación. El resultado comprobado no
expone una bandera de elegibilidad y el almacén debe resolver esas obligaciones
durante la admisión atómica con autorización y auditoría.

El servicio autorizado `MeasureAdministrativeService` prepara y confirma Correct
y MarkEnteredInError para Owner y Litigator; Paralegal y Client no pueden escribir.
`MeasureAdministrativeReady` conserva contexto observado, un documento cifrado,
cabeza exacta observada e inventario completo suministrado de propietarios y
prefijos. La referencia del comando debe coincidir con `target_head`; se valida
una vez todo `dependency_inventory` y se rechaza cualquier uso directo conocido,
incluidos Review históricos y decisiones sin filas que anclan Review. El registro
seleccionado debe conservar validez Valid. Una acción judicial terminal sigue
siendo rectificable administrativamente sin cambiar ni revivir su declaración.

El mismo índice comprobado aporta el registro efectivo y su cierre original de
ancestros. La revisión contrasta contexto activo, comando y fuentes inmutables;
conserva las fuentes históricas, incluso archivadas. Extrae el cierre exacto del
objetivo sin volver a hashear el bosque ni guardar raíces ajenas o la ascendencia
adicional del prefijo como ancestros del recibo. El candidato cuenta en los
límites de 256 propietarios y 8192 filas del cierre devuelto. El inventario
observado mantiene, por separado, los límites completos del inspector.

Ambas acciones admiten fuera del bloqueo de auditoría la versión cifrada exacta
del soporte de la última declaración judicial real. El procesador comprueba
integridad y formato; la captura admitida debe coincidir íntegramente con el
soporte retenido. No se sustituyen sujetos o supervisores por sus cabezas actuales.
Solo el servicio construye el valor preparado privado. La confirmación exige
los digests de instrucción y revisión completa, reautentica al actor actual
completo y exige observaciones UTC compatibles y monótonas. La captura nueva no
puede preceder las fuentes comprobadas ni la observación previa al commit.

La recuperación devuelve captura, origen y cierre original, sin el nuevo
propietario dentro de sus ancestros. Conserva autor, rol, correo y hora históricos,
no readmite el soporte ni exige ausencia eterna de usos posteriores. Se valida
todo recibo devuelto, incluso una operación concurrente idéntica. El almacén debe
autorizar el acceso actual antes de buscar la operación, también en expediente
cerrado, y la revocación sigue impidiendo su devolución.

El servicio distingue cabeza obsoleta, dependientes conocidos, operación en
conflicto, confirmaciones distintas e historia incompleta o inconsistente.
El puerto exige revalidar bajo el bloqueo de auditoría principal, acceso,
contexto activo, cabeza Valid, ausencia durable completa de dependientes y
fuentes/soporte exactos; debe impedir carreras y guardar recibo, fila, origen,
operación, cabeza y auditoría atómicamente. Esa obligación no queda demostrada
por un inventario suministrado sin usos conocidos.

El servicio administrativo y sus puertos no acreditan todavía persistencia ni admisión
transaccional real. Siguen pendientes PostgreSQL, rutas HTTP y aceptación de
restauración, la ampliación de los servicios de convocatorias y decisiones para
material mixto y el reemplazo administrativo opcional con identidad nueva y
enlace atómico. Agenda, alertas e interfaz siguen pendientes. Los formatos
MATXN1/MAPR1/MARCR1/MAGR1 no cambian.

Los resultados focales se registran en [el informe](verification-report.md);
no acreditan por sí solos el flujo completo.

## Fuente primaria y límites de interpretación

La consulta del 4 de octubre de 2026 al
[CNPP de la Cámara de Diputados](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf)
y al [índice oficial de reformas](https://www.diputados.gob.mx/LeyesBiblio/ref/cnpp.htm)
identificó como última reforma reportada la del 28-11-2025. El SHA-256 del PDF
consultado fue
`7ceb0682c948d7d945e7e68b8d8699ceb795fc9a491aa62325db91ccf2309904`.
La fecha y huella identifican esa consulta; no certifican la ausencia de otra
disposición o resolución aplicable.

| Artículos del CNPP | Base resumida | Consecuencia técnica propuesta |
| --- | --- | --- |
| 153, 154, 157 | Imposición judicial con supuestos y decisión en audiencia. | Separar solicitud, convocatoria y decisión; capturar fundamento y soporte. |
| 155, 156, 157 | Catorce medidas, proporcionalidad y restricciones de combinación. | Clasificación cerrada; sin recomendación o validación jurídica automática. |
| 159 | Justificación, lineamientos y vigencia en la resolución. | Conservar declaraciones y evidencia, incluidas sus carencias. |
| 161-163 | Revisión a petición y audiencia; el término de 48 horas tiene un supuesto. | La solicitud no modifica la medida; no generar `+48h` por seleccionar revisión. |
| 164 | Evaluación y supervisión por autoridades determinadas. | Registrar la fuente sin certificar cumplimiento o atribuciones. |
| 307 | La audiencia inicial comprende solicitudes cautelares. | Enlazar la audiencia existente sin duplicar la cita. |
| 347, 401, 405 | Medidas al abrir juicio y levantamiento en absolución. | Conservar historia y decisión expresa, sin terminar por cambio de etapa. |
| 137 | Medidas u órdenes de protección con régimen propio. | No incluirlas como sinónimos del catálogo cautelar. |

Fuente de la tabla: [CNPP, artículos indicados](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).
La última columna expresa decisiones de diseño, no reglas de procedencia. El
prototipo no calcula competencia, proporcionalidad, cumplimiento ni efectos de
impugnaciones. Tampoco convierte un máximo legal en una duración concedida.

## Catálogo finito de audiencias del producto

| Familia o relación | Contrato y frontera |
| --- | --- |
| Inicial | [Familia ordinaria](hearings-api.md); conserva programación y [resultados declarados](hearing-results-api.md). La materia cautelar puede enlazarse a esa misma cita. |
| Intermedia | Familia ordinaria existente; admisión o exclusión probatoria es contenido de actos, no otra etiqueta de cita. |
| Juicio oral | Familia ordinaria existente; sus sesiones/resultados no acreditan firma judicial por usar la firma interna del prototipo. |
| Individualización y reparación | Familia ordinaria existente; conserva el contexto condenatorio declarado y su soporte. |
| Medidas cautelares | Familia propia propuesta: convocatoria de imposición o revisión, decisiones y medidas con historia. |
| Alegatos de apelación | [Familia propia de recurso](resource-hearings.md), con su origen independiente. |
| Audiencia de revocación escrita | Familia propia de recurso; la revocación oral no crea una cita separada. |
| Continuación | Relación exacta con sesión/resultado anterior, no una octava especie de audiencia. |

La matriz acota el compromiso del prototipo, sin pretender enumerar todas las
audiencias del CNPP. No reemplaza otros criterios del
[alcance del producto](product-completion.md) ni declara terminado el documento
académico. Las descripciones de
[audiencias](../latex/chapters/03-audiencias.tex) y su
[anexo](../latex/chapters/anexo-e-audiencias.tex) conservan su alcance versionado.

## Tres registros con historia propia

1. **Convocatoria cautelar:** propósito, cita exacta, modalidad/lugar,
   participantes y soporte. Reprogramación y cancelación conservan historia.
   La revisión refiere medidas exactas sin afirmar que fueron modificadas.
2. **Decisión declarada:** identidad de resolución/autoridad, tiempo con su
   precisión, justificación y soporte exacto. Puede enlazar una audiencia inicial
   o convocatoria cautelar mediante referencia discriminada; no inventa una
   audiencia ausente ni usa un resultado concluido como prueba de imposición.
3. **Medida:** identidad estable, sujeto con ficha/revisión exacta, clase,
   condiciones, inicio y vigencia declarados, decisión de origen e historia.
   Persiste aunque cambie la cita, etapa o estado administrativo del expediente.

La convocatoria propia captura etapa y administración observadas, junto con el
contexto declarado. Puede registrar un señalamiento comunicado en cualquiera de
las tres etapas sin convertirlas en una regla de competencia. Toda referencia
contradictoria, ajena o cuyo contexto revisado cambió debe rechazarse.

### Clasificación cerrada de medidas

Los códigos técnicos corresponden, en orden, a las catorce fracciones del
[artículo 155 del CNPP](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).
Son clasificaciones declaradas, no una elección automática de medida aplicable.

| Código | Clasificación declarada |
| --- | --- |
| `periodic_appearance` | Presentación periódica |
| `financial_guarantee` | Garantía económica |
| `asset_seizure` | Embargo de bienes |
| `account_freeze` | Inmovilización de cuentas y valores |
| `travel_restriction` | Restricción de salida del ámbito fijado |
| `custody_or_institution` | Cuidado, vigilancia o internamiento indicado |
| `place_restriction` | Restricción de reuniones o lugares |
| `contact_restriction` | Restricción de convivencia, acercamiento o comunicación |
| `home_separation` | Separación del domicilio |
| `public_office_suspension` | Suspensión en cargo público |
| `professional_suspension` | Suspensión de actividad profesional o laboral |
| `electronic_monitoring` | Localizador electrónico |
| `home_confinement` | Resguardo domiciliario |
| `pretrial_detention` | Prisión preventiva |

Conservar el texto de las condiciones sin inferir montos, zonas, periodicidad o
plazos. Si la fuente no permite clasificar, conservar documento/borrador; no
publicar una medida ficticia bajo `other`. Una persona supervisora se captura
con su revisión exacta o una declaración explícita de desconocimiento; un título
en el directorio no prueba nombramiento oficial.

### Tiempo y estado visible

Separar fecha declarada de decisión, inicio declarado, fin declarado y captura
real del servidor. Una fecha civil y un instante con desfase no son
intercambiables. `MeasureTime` conserva precisión, componentes y desfase opcional.
`Unknown` exige motivo; `Date`, `Minute` y `Second` prohíben el motivo de valor
desconocido. No inventar medianoche, UTC, duración o la hora de captura.

`MeasureValidity` compara inicio y término solo con estas reglas:

| Precisiones declaradas | Comparación permitida |
| --- | --- |
| `Date` / `Date` | Orden civil cuando los desfases opcionales son iguales, incluidos ambos ausentes; sin convertir la fecha en un instante. |
| `Minute` / `Minute` o `Second` / `Second` | Orden en UTC únicamente con ambos desfases explícitos; conserva componentes y desfases originales. |
| Precisión mezclada, hora sin desfase, fechas civiles con desfases opcionales distintos o un valor desconocido | Conserva las declaraciones y deja el orden sin resolver. |

Rechazar un término anterior al inicio si la pareja es comparable. Conservar una
pareja no comparable no acredita que su orden cronológico sea correcto.

La vigencia conserva texto expreso. El término ausente difiere de un término
declarado `Unknown` con motivo; ninguno significa perpetuidad ni conclusión.
`MVAL1` vincula precisión, componentes, desfase opcional y motivo del inicio,
la declaración de vigencia y la presencia y valor completo del término opcional.
Es un compromiso de valores, sin recibo, decisión o medida persistida.
La interfaz debe decir **última declaración registrada**; el reloj, cierre del
expediente y cambio de etapa no modifican el estado jurídico. Obligaciones
periódicas y términos requieren un contrato expreso adicional; no se extraen
automáticamente del texto de condiciones.

### Decisiones, grupos y rectificación

- `measure_changes` requiere cambios; `no_measure_change` exige cero cambios y
  resultado observado con razón y soporte. Desconocer el resultado sigue siendo
  incompleto. La no celebración pertenece a la cita.
- Una decisión puede afectar varios sujetos y hasta 32 identidades de medida,
  contando anteriores y nuevas. Es un límite técnico; no dividir silenciosamente
  una decisión mayor. Ordenar por UUID y rechazar objetivos repetidos.
- `impose`, `confirm`, `modify`, `revoke` y `cease` conservan decisiones y
  predecesores exactos. Confirmar agrega evidencia sin cambiar origen, sujeto,
  clase, condiciones, inicio o vigencia; modificar no reescribe el pasado.
- `substitute` enlaza uno o más predecesores y uno o más sucesores del mismo
  sujeto. Identidades disjuntas y únicas, sin ciclos ni parejas uno a uno
  inventadas; enlaces y revisiones se confirman juntos.
- `correct_record` rectifica texto/precisión con razón, soporte original y
  predecesor exacto. No cambia identidad, sujeto, clase, origen, efecto judicial
  ni decisión histórica. Solo actualiza la proyección desde su cabeza exacta.
- Identidad capturada por error requiere `entered_in_error`, conservando
  historia. Una entrada correcta tiene identidad nueva y enlace administrativo
  atómico; no se presenta como revocación o sustitución judicial. Las decisiones
  posteriores requieren conciliación expresa, sin cascadas inferidas.

## Autorización, evidencia y recuperación pendientes

Owner y Litigator con asignación vigente preparan/confirman; Paralegal asignado
consulta; Client permanece denegado. Mutar requiere administración activa.
Historia y replay requieren autorización actual. Reautenticar al principal
completo y conservar por separado ID, correo y rol históricos del autor.

La preparación resuelve documentos, participantes, sujetos y medidas por su
revisión/digest exactos; las cabezas vigentes se observan aparte. Admite soportes
fuera del bloqueo de auditoría y no crea objetos. Confirmar vuelve a comprobar
identidad, membresía, administración, contexto, soportes y revisiones esperadas
dentro de la transacción.

Una operación UUID compromete una instrucción inmutable. Replay idéntico y
autorizado devuelve el grupo original; otra instrucción entra en conflicto.
Encontrar registros aislados por ID no prueba origen conjunto. Decisión, todas
las revisiones, enlaces, origen y un evento de auditoría del grupo se confirman
todo o nada, incluso ante concurrencia o fallo. El evento enlaza la decisión y
cada revisión; la cita nueva es una intención separada.

Agenda necesita `precautionary_hearing`, orden y cursor estables con familia,
identidad y revisión, además de detalle autorizado propio. Una decisión sobre
inicial reutiliza su fila; no agrega otra cita ni filas de medidas. Las alertas
conservan el origen de la cita y no infieren incumplimientos o términos.

La interfaz debe conservar borradores solo para la misma identidad al reingresar,
retirar aprobación y reautorizar. Respuestas inciertas exigen conciliación exacta
antes de un reintento expreso; sin reenvío automático. Cambio de usuario, logout
o expediente invalida respuestas tardías y no recupera borradores ajenos.

Restauración exige migraciones nuevas, inventario estricto y decodificadores
acotados por versión. Conserva convocatorias, decisiones, revisiones, enlaces,
correcciones, recibos, autores, orígenes y auditoría; rechaza huérfanos, ambigüedad,
versiones desconocidas y discrepancias. No sustituye fuentes por cabezas actuales
ni recalcula efectos. Los canones históricos mantienen sus bytes y significado.

## Evidencia necesaria para cerrar el flujo

1. TDD de valores/canones y preservación de vectores históricos; casos positivos
   y negativos de alcance, precisión, propósitos y orden estable.
2. Aplicación y PostgreSQL: autorización vigente, aislamiento entre expedientes,
   cambios de contexto, objetivos obsoletos, rollback de grupos y replay concurrente.
3. HTTP antes/después de reinicio y restauración: mismos orígenes, lecturas y
   recibos; sin duplicados, auditoría separada o sustitución parcial.
4. Qadra en escritorio/móvil: programar, declarar, revisar, rectificar, consultar
   historia y recuperar respuesta perdida/nueva sesión; inicial sin cita duplicada.
5. Documentación de API, operación, alcance e implementación académica consistente
   con resultados reproducidos; [informe de verificación](verification-report.md)
   con evidencia fresca separada de la histórica y comprobaciones de cierre.

Estos criterios no declaran implementado un corpus completo de términos, firma
judicial, supervisión oficial o adjudicación jurídica. El
[flujo de plazos derivados](hearing-derived-deadlines.md) conserva su contrato
ordinario; admitir una fuente cautelar requerirá su propio contrato exacto.
