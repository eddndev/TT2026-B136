# Verificación de la actualización académica

## Seguimiento humano V2 compuesto y Qadra: 19 de septiembre de 2026

Los apartados propios describen decisiones humanas V2, consultas de vigencia,
reevaluación compuesta en `serve`, supervisión y aceptación real HTTP/Qadra.
Se conserva el alcance de los perfiles sintéticos, la activación, las alertas
y el trabajo jurídico restante. Las correcciones de cierre sólo modificaron
el ajuste de rutas y nombres de archivos en el anexo de plazos; no cambiaron
resumen, objetivos, estado del arte, conclusiones ni presentación.

El [workflow Documents](https://github.com/eddndev/TT2026-B136/actions/runs/35431568521)
compiló el árbol de `f743c5e`: el artefacto identifica el merge de revisión
`37e5e5d`, cuyos padres son `214202a` y `f743c5e`; su árbol coincide exactamente
con la cabeza de la rama. El PDF tiene **315 páginas, formato carta y 5874639
bytes**. SHA-256:
`db171360b216c9ca23498f85230a4a90878b85729a06a31d184308a729395e6f`.

La primera revisión detectó una lista de archivos fuera del margen en la página
310. Se separaron sus elementos y se permitió el ajuste de una ruta larga.
La comprobación final renderizó **22 páginas físicas: 3-5, 185, 190-196,
224-228 y 307-312**. Diecinueve imágenes son idénticas byte por byte a las ya
inspeccionadas; se inspeccionaron de nuevo las páginas 310-312. No presentan
recortes ni solapamientos. Siete páginas de implementación recibieron revisión
independiente. Esta selección no equivale a inspeccionar las 315 páginas.

La última pasada de LuaLaTeX no contiene referencias o citas indefinidas,
etiquetas duplicadas ni glifos ausentes. Conserva las sustituciones de versalitas
de Times New Roman y el desborde histórico de **0.11754 pt**; el desborde nuevo
quedó corregido. El artefacto comprobado se conserva como
`output/pdf/TT2026-B136-reporte.pdf`; el PDF previo de 312 páginas se preservó
por separado. No se añaden binarios a Git.

La aceptación de software pertenece al [informe técnico](verification-report.md).
Este resultado no acredita usabilidad con personas, perfiles jurídicos
calificados, aprobación del manuscrito completo ni preparación final de defensa.


## Consumidor durable de reevaluación: 18 de septiembre de 2026

Se actualizaron seis apartados propios de implementación, pruebas y trazabilidad
de perfiles y plazos. Describen la captura humana V2, el despacho persistente,
el consumidor local, los resultados e intentos auditados, la clasificación
tipada de fallos y la recuperación. Separan esos componentes de su composición
pendiente en `serve`, del servicio humano y HTTP/Qadra V2, de la activación,
la agenda conjunta y las alertas. Los resultados funcionales pertenecen al
[informe técnico](verification-report.md); la campaña integral de 2630 pruebas
incluye los 27 casos PostgreSQL del consumidor, sin volver a sumar sus campañas
focales.

Se conservaron por hash **77 de 83 archivos versionados** del reporte y la
presentación; sólo cambiaron los seis apartados descritos. Resumen, objetivos,
estado del arte, conclusiones pendientes y presentación permanecen iguales.
Los entregables de otros checkouts no se sustituyeron.

`make -C latex` con LuaLaTeX, biber y glosarios terminó con código cero en
**82.001 s**, después de finalizar la verificación de software. Las fuentes
conservaron sus huellas durante la compilación. El PDF resultante contiene
**312 páginas, formato carta y 5789359 bytes**; se conserva localmente como
`output/pdf/TT2026-B136-reporte.pdf`, sin añadirlo a Git.

Se renderizaron e inspeccionaron **22 páginas físicas: 3-5, 187-194, 219-224
y 304-308**, incluidos índices y transiciones adyacentes. Trece páginas
recibieron revisión independiente. Texto, rutas, tablas, encabezados y jerarquía
son legibles, sin recortes ni solapamientos. El log final no contiene referencias
o citas indefinidas, etiquetas duplicadas ni glifos ausentes. Conserva las
sustituciones preexistentes de versalitas de Times New Roman y el desborde
histórico de **0.11754 pt**.

SHA-256: `ab19660015679902fa7ba34b4f259af4919a203038112912b5face14e7d03af8`.

Esta comprobación corresponde al PDF local y a las páginas indicadas; no afirma
una nueva compilación remota ni una inspección visual de todas sus páginas.
Tampoco acredita aceptación operativa en Qadra, usabilidad con personas,
calificación jurídica de perfiles ni cierre del manuscrito completo.

## Interfaz y selector de plazos: 18 de septiembre de 2026

Se actualizaron los tres apartados propios de implementación, pruebas y matriz
para describir la operación de plazos en Qadra y el selector autorizado de
responsables. Incluyen revisión exacta, datos desconocidos, confirmación de
bloqueos, conflictos, conciliación sin reenvío, cuatro roles y cierre. Los
recorridos controlados y los de servicios reales se distinguen; sus resultados
pertenecen al [informe técnico](verification-report.md).

Se conservaron por hash **80 de 83 archivos versionados** del reporte y la
presentación; sólo cambiaron esos tres apartados. Resumen, objetivos, estado del
arte y conclusiones permanecen iguales. La presentación y las ediciones ajenas
no se incorporaron a este cambio. El PDF anterior de 305 páginas se preservó.

`make` con LuaLaTeX, biber y glosarios aprobó en **59.808 s**. El PDF resultante
tiene **306 páginas y 5769158 bytes**. Se inspeccionaron las páginas físicas
**190-192, 218-220 y 300-302**, incluidas transiciones adyacentes; cinco recibieron
revisión independiente. Texto, rutas, tabla y jerarquía son legibles, sin recortes
ni solapamientos. No hay referencias/citas indefinidas ni glifos ausentes. El log
conserva las sustituciones preexistentes de versalitas de Times New Roman y la
caja excedida histórica de **0.11754 pt**. Las fuentes comprobadas son las mismas
que produjeron el PDF.

SHA-256: `e87e93fa377cc88cb4cbe60311975ba0698a449b9e711c2b3f7d6a2f6556ef4b`.

Esta revisión no acredita usabilidad con personas, corpus jurídico aprobado,
reevaluación durable, alertas ni cierre del manuscrito completo. La aceptación
integral de software y la integración remota conservan evidencia propia.

## Plazos persistentes y consulta HTTP: 17 de septiembre de 2026

Se añadieron tres apartados propios de [implementación](../latex/chapters/04-plazos-persistentes.tex),
[pruebas](../latex/chapters/05-plazos-persistentes.tex) y
[trazabilidad](../latex/chapters/anexo-e-plazos-persistentes.tex). Describen la
entrada DEVI1, el resultado histórico DRES1, PostgreSQL 0017, responsables,
atención declarada, recibos y auditoría, sin recalcular la historia sobre las
fuentes actuales. La elegibilidad vigente del responsable se comprueba en alta
y corrección; atención y retiro conservan su captura histórica. Se corrigieron
las referencias de estado de los apartados previos para enlazar ese flujo.

El texto diferencia las pruebas focales de la campaña HTTP con servicios reales,
que recorrió días, meses y horas y conservó **14 respuestas exactas de plazos**
después de restaurar. Las cifras y comandos de software pertenecen al
[informe técnico](verification-report.md); este corte académico no introduce
un conteo global nuevo. La interfaz de plazos en Qadra, los estados derivados
del reloj, la reevaluación y las alertas siguen pendientes.

De **70 fuentes académicas previas**, **64 permanecen idénticas** por hash.
Se modificaron seis archivos existentes —tres inclusiones y tres ajustes
puntuales de estado— y se añadieron tres; el corte contiene **73 fuentes**.
Las nueve fuentes propias coinciden con las usadas en la compilación final.
El README de LaTeX no cambió. Se preservaron resumen, objetivos, teoría,
estado del arte, conclusiones y presentación. No se incorporaron las doce
ediciones ajenas descritas en los cortes anteriores.

`make` con LuaLaTeX, biber y glosarios terminó con código cero. El PDF final
contiene **305 páginas y 5762720 bytes**. Se inspeccionaron visualmente las
páginas físicas **189-192, 217-219 y 299-302**, incluidos los párrafos finales
sobre autorización, API y restauración. Se corrigieron seis desbordamientos
en los nombres de archivos del anexo mediante directorios, prefijos y sufijos
separados y alineación local; la tabla y el tamaño global de letra se conservaron.
También se corrigió la concordancia de referencias a secciones. Las páginas
afectadas se volvieron a renderizar e inspeccionar: texto, tablas, jerarquía y
transiciones quedaron legibles, sin recortes ni solapamientos nuevos.

El log final no contiene referencias o citas indefinidas ni glifos ausentes.
Sólo permanece la caja excedida preexistente de **0.11754 pt**, ajena a los
apartados propios. La huella del PDF se comprobó después de la revisión visual.

SHA-256: `2f144bbaa41c02eccc3ee687c76c0e9ba0f85fc8aeb043366b246b1d93c1d5cd`.

Esta revisión comprende los apartados modificados y sus transiciones; no es
una nueva inspección de todas las páginas históricas. Tampoco acredita
usabilidad, aceptación jurídica de perfiles, entrega de alertas ni terminación
del manuscrito. Las conclusiones y la actualización integral pendiente de
calendarios, hechos y sesiones conservan sus límites de alcance y autoría.

## Catálogo de perfiles y evaluación explícita: 16 de septiembre de 2026

Se añadieron tres apartados propios de implementación, pruebas y trazabilidad
para perfiles versionados, evaluación en memoria y eventos durables. Se incorporó
una inclusión en cada capítulo contenedor y se corrigieron tres frases de estado
en el apartado de insumos. La descripción distingue el catálogo persistente y
su API de los plazos operativos, responsables, atención, trabajadores, alertas y
Qadra todavía pendientes. Los resultados de software pertenecen al
[informe técnico](verification-report.md); no se reutilizan las cifras históricas.

De **67 fuentes académicas previas**, **63 permanecen idénticas** por hash.
Cambiaron cuatro archivos existentes y se añadieron tres: el corte contiene
**70 fuentes**. Resumen, objetivos, estado del arte, conclusiones y presentación
se preservaron. No se incorporaron las doce ediciones ajenas cuya procedencia
se describe en los cortes anteriores.

`make` con LuaLaTeX, biber y glosarios aprobó. El PDF final tiene **300 páginas y
5742752 bytes**. Se inspeccionaron las páginas físicas **187-191, 214-216 y
295-296**, con revisión independiente de cinco de ellas. Se corrigieron seis
desbordamientos del anexo separando directorios y nombres por elemento, y la
concordancia de una referencia al apéndice. La página de esa corrección se volvió
a renderizar e inspeccionar. Texto, tabla, jerarquía y transiciones quedaron
legibles, sin recortes ni solapamientos nuevos.

El log final no contiene referencias o citas indefinidas ni glifos ausentes.
Conserva la caja excedida preexistente de **0.11754 pt**, ajena a los apartados
propios. Las fuentes quedaron idénticas al manifiesto de la última compilación;
el PDF anterior de 294 páginas se conservó por separado.

SHA-256: `9b5a4ce25710d48b5ba72397c80859bb36f0cd169471a45dd9dc3fa5ff59d086`.

Esta revisión no acredita usabilidad, aceptación jurídica de los perfiles,
entrega de alertas ni terminación del manuscrito completo. Las conclusiones y
la actualización integral pendiente de calendarios, hechos y sesiones conservan
sus límites de alcance y autoría.

## Insumos temporales autorizados: 16 de septiembre de 2026

Se ampliaron los tres apartados propios de implementación, pruebas y matriz
para describir la resolución autorizada en aplicación/PostgreSQL. Conservan la
selección histórica y la cabeza observada, recibos, administración sin revisiones,
lecturas cerradas, autorización y segunda autenticación. La matriz comprende
**50 pruebas nuevas** aprobadas en la suite de **1878 aprobadas, cero fallidas
y una externa ignorada**; sus resultados y comandos pertenecen al
[informe técnico](verification-report.md).

De **67 fuentes académicas previas**, **63 permanecen idénticas** por hash.
Solo cambiaron los tres archivos de desencadenantes y el README. Se preservaron
resumen, objetivos, estado del arte y conclusiones, así como las doce ediciones
ajenas pendientes de integración descritas abajo.

`make` con LuaLaTeX, biber y glosarios terminó correctamente. El PDF tiene
**294 páginas y 5719479 bytes**. Se inspeccionaron las páginas físicas
**186-188, 210-211 y 289-290**: jerarquía, texto, tablas y transiciones legibles,
sin recortes ni superposiciones. No hay referencias/citas indefinidas ni glifos
ausentes. El log mantiene la caja excedida preexistente de **0.11754 pt** en
un título ajeno a los apartados modificados.

SHA-256: `8d98cab6ff1905ac298e6eec60fe26438004a5e9412c5515a055974364a56108`.

Esta comprobación no sustituye la actualización integral pendiente de calendarios,
hechos y sesiones, ni completa perfiles normativos, evaluaciones persistentes,
alertas, usabilidad o conclusiones. La preparación verificada permanece en memoria.

## Fuentes temporales y aritmetica: 16 de septiembre de 2026

Se incorporaron tres apartados propios de implementacion, pruebas y trazabilidad
para la extraccion temporal exacta y su coordinacion con la aritmetica. Describen
campos independientes, ausencia frente a desconocimiento, expediente, revision,
padre y acuerdo exactos, precision, calificacion declarativa y limites de
confianza. La matriz registra **45 pruebas focales aprobadas**; la suite global
de **1828 aprobadas, cero fallidas y una externa ignorada** tiene su propia
evidencia en [el informe tecnico](verification-report.md).

Los cambios consisten en tres archivos nuevos y una inclusion en cada archivo
contenedor de implementacion, pruebas y matriz, mas la actualizacion del README.
De los **64 archivos academicos previos** controlados por hash, **60 permanecen
identicos**. Los cuatro modificados son esos tres contenedores y el README.
Resumen, objetivos, estado del arte y conclusiones pendientes permanecen iguales.
No se incorporaron las doce ediciones ajenas mencionadas en el corte siguiente.

`make` compilo con LuaLaTeX, biber y makeglossaries. El PDF final tiene **292
paginas y 5713480 bytes**, sin referencias o citas indefinidas ni glifos ausentes.
Se inspeccionaron las paginas fisicas **185-187, 208-209 y 287-288**. Se corrigio
la ubicacion del apartado de dominio para conservar la jerarquia HTTP, la
jerarquia independiente del anexo, una concordancia de referencia y el ancho
del encabezado Casos. La ultima recompilacion y las paginas afectadas volvieron
a comprobarse visualmente: texto y tablas legibles, sin recortes o solapamientos.

El log conserva una caja excedida de **0.11754 pt** en el titulo existente del
riesgo RLC-03, ajeno a los apartados nuevos. No hay una medicion de usabilidad
ni una inspeccion visual nueva de todas las paginas historicas del reporte.
SHA-256 del PDF final:
`fe6be26f54d6b2e3ffbb2a87a1a599d3f20051100125c133b3e1f3d726f6779e`.

La actualizacion integral de calendarios, hechos y sesiones conserva los
pendientes de procedencia identificados abajo. Estos apartados de dominio no
cierran perfiles normativos, plazos persistentes, alertas, conclusiones o defensa.

## Procedencia de las fuentes y actualización pendiente

El corte de sesiones y resultados descrito abajo corresponde a un snapshot local
de 65 fuentes y figuras. Doce fuentes del manuscrito siguen pendientes de
integración; el PDF comprobado conserva su procedencia de árbol de trabajo y no
se atribuye a un commit que no contenga esas fuentes. Los resultados funcionales
del software tienen su propia evidencia e historia versionada.

La entrega de calendarios jurisdiccionales incorpora contrato, decisiones,
operación y resultados reproducidos en [el informe técnico](verification-report.md).
Su actualización del manuscrito, compilación e inspección de PDF siguen pendientes.
Las comprobaciones de PDF de sesiones y programación conservadas en este documento
no verifican los cambios posteriores de calendarios. El resumen, los objetivos,
el marco teórico y las conclusiones pendientes permanecen protegidos.


La captura de resoluciones y notificaciones agrega dominio, servicio,
persistencia auditada, API y cliente Qadra. Sus contratos y campañas de software
se describen en [la API de hechos](procedural-facts-api.md) y en el informe
técnico. La actualización correspondiente del manuscrito, su compilación y
la inspección del PDF siguen pendientes junto con las de calendarios. Los
resultados documentales históricos de este archivo no verifican esa interfaz
ni su implementación posterior.


La aritmetica temporal de dias, meses civiles y horas y la investigacion de
perfiles se documentan en [el contrato](deadline-arithmetic.md) y en
[las fuentes normativas](deadline-rule-research.md). Los apartados de operandos y
fuentes exactas incorporan ahora los componentes de dominio al manuscrito, con
la comprobacion documental descrita arriba. La investigacion de aplicabilidad
juridica y la integracion operativa mantienen su trabajo pendiente. Las pruebas
matematicas no acreditan un perfil juridico ni reemplazan los criterios aprobados
sobre plazos, calendario y alertas.

## Sesiones y resultados declarados: 16 de septiembre de 2026

Se incorporaron cuatro apartados de diseño, implementación, pruebas y
trazabilidad para sesiones declaradas: raíz por acto, ancla exacta de programación,
continuidad histórica, comparecencias, acuerdos, procedencia y soporte exacto.
La interfaz Qadra y sus estados de conflicto e incertidumbre se describen junto
con la API y la transacción auditada. La programación anterior conserva su alcance
propio. El registro declarado no acredita notificación ni efectos judiciales.

### Evidencia incorporada y límites

El manuscrito describe **1301 pruebas Rust aprobadas, cero fallos y una prueba
externa de TSA ignorada**, en ejecución ordinaria e instrumentada con servicios
desechables. Mantiene **28 704 / 31 010 líneas, 92.56 %**, y los numeradores por
crate. Formato, compilación, Clippy 1.98.1, Rust 1.88, política de dependencias,
binario release, CLI y recorrido HTTP/restauración quedaron contrastados con
los manifiestos. La revisión focal posterior del test de arranque concurrente
se informa por separado, sin sumarla al total ni repetir la cifra de cobertura.

Las cifras web finales son **151 unitarias, 204 escenarios simulados y 14
recorridos reales**. Se distinguen 223.553 segundos del comando simulado y
258.642 del script real, frente a los 3.7 y 2.5 minutos informados por Playwright.
No se equiparan estas pruebas con una evaluación de usabilidad con personas.

La media actual de Argon2id, **409.6 ms en cinco corridas**, permanece por debajo
de la banda objetivo; su subcriterio se indica pendiente de recalibración.
La medición histórica de 529.4 ms conserva su fecha y contexto. No se cambiaron
parámetros criptográficos para alterar el resultado. Plazos automáticos,
calendarios, alertas, recursos y aceptación integral permanecen pendientes.

### Conservación y comprobación visual

La edición conservó **592 archivos protegidos** y las cinco tablas aprobadas
CU-08, RF-07, RF-08, RF-09 y RF-10 byte por byte. Resumen, introducción con
objetivos, marco teórico, conclusiones pendientes, presentación y entregables
anteriores permanecieron iguales. Las diferencias necesarias se explican en
prosa técnica y trazabilidad sin reescribir aquellos criterios.

La compilación final usó el Makefile versionado sobre un snapshot aislado de
**65 fuentes y figuras**, después de completar las campañas funcionales.
Terminó con salida cero en 77.588 segundos. El resultado tiene **295 páginas**,
**5 729 707 bytes**, formato carta y SHA-256
`6bb7bad31e75032469fb1c6010baa3bb3e3f3f813b310c4e61ff13718cfb5003`.
El manifiesto de las 65 fuentes tiene SHA-256
`4fb488d02274462392d36f0257791e7aed32ca6232cf15107a2dd7ded206414f`.

Se renderizaron y revisaron **61 páginas** afectadas o de control: 17 de forma
individual y 44 mediante hojas de contacto. La comprobación de coordenadas
mantuvo **15 299 palabras** dentro de los límites definidos; no se observaron
recortes, superposiciones ni defectos nuevos. Otra revisión individual comprobó
las páginas PDF 211, 212, 213 y 224, incluidas cobertura, cifras web y el estado
pendiente de Argon2id. La inspección no pretende abarcar visualmente las 295
páginas completas. No hay referencias/citas indefinidas ni glifos ausentes.
Persisten los avisos históricos de versalitas y 0.11754 pt en el índice de tablas,
sin defecto visual apreciable.

El PDF se publicó como un entregable nuevo, conservando los anteriores y los
preliminares de esta entrega. Los manifiestos separan el snapshot de trabajo,
las fuentes efectivas, el PDF, sus comprobaciones y la procedencia de software.
La compilación y la inspección documental no son una nueva ejecución de Rust;
la evidencia funcional está en [el informe de verificación](verification-report.md).
La integración remota y su compilación CI se comprobarán por separado.


## Programación de audiencias: 16 de septiembre de 2026

Se añadieron apartados de diseño, implementación, pruebas y trazabilidad para
programar, reemplazar y cancelar audiencias con historia inmutable. Describen
fecha y desfase originales, contexto administrativo y procesal, referencias
exactas a participantes y soporte, canon HEAR1, recibo HTXN1, autorización por
expediente y conciliación de respuestas inciertas. Se ajustaron las menciones
que todavía presentaban toda programación como pendiente.

El alcance permanece parcial: no se atribuyen celebración, resultados,
asistencia, acuerdos, plazos ni alertas a una cita registrada. La matriz explicita
las menciones adicionales de medidas cautelares y continuación del marco
teórico. La agenda autorizada no equivale al calendario completo con plazos.
CU-08, RF-07, RF-08, RF-09 y RF-10 conservan sus tablas aprobadas byte por byte;
la limitación se explica en la prosa técnica. El resumen, la introducción con
sus objetivos, el marco teórico y las conclusiones pendientes no se modificaron.

### Evidencia de software descrita

La ejecución global ordinaria y la instrumentada aprobaron **1183 pruebas Rust,
cero fallos y una externa de TSA ignorada**, en 360.492 y 370.401 segundos.
La cobertura fue **25 201 / 27 226 líneas, 92.56 %**, con los tres umbrales
del 90 % aprobados para dominio, aplicación e infraestructura. El capítulo
conserva los numeradores por crate y distingue las pruebas con dobles de las
transaccionales sobre PostgreSQL real. Se incorporaron revalidación tras
preparación, concurrencia con un solo sucesor, rollback y referencias exactas.
Formato, compilación, Clippy 1.98, Rust 1.88 y cargo-deny 0.20.2 terminaron
satisfactoriamente. El binario release midió 12 176 936 bytes. La CLI terminó
en 9.097 segundos y midió Argon2id en 540.7 ms sobre cinco corridas, concurrentes
con otras comprobaciones; no se presenta como benchmark aislado.

La demostración HTTP con PostgreSQL/Redis aislados terminó con código cero
en 166.180 segundos y conservó el recorrido de audiencias y su restauración.
Se compararon doce respuestas completas; el inventario conservó tres raíces y
cinco revisiones de audiencias junto con expedientes, documentos, participantes
y evidencia anterior. Qadra aprobó 119 pruebas unitarias, 178 escenarios
simulados, formato y compilación. La repetición final del navegador aprobó los
once recorridos con servicios reales: 222.42 segundos de comando completo y
2.1 minutos de escenarios Playwright. Los tres casos nuevos comprueban recibos,
conflictos e historia, autorización de agenda y soporte exacto de individualización.

Las cifras del cierre de identidades permanecen históricas. La evidencia
funcional consta en [el informe de verificación](verification-report.md);
esta revisión describe exclusivamente la fidelidad y composición del reporte.

### Conservación y preparación documental

Antes de editar se verificaron las 75 huellas del baseline del cierre anterior,
sin diferencias. Se amplió el registro a 149 archivos: fuentes protegidas,
presentación completa, avance de Windows, PDF anteriores y materiales de la
compilación CI anterior. La comprobación posterior a la edición conserva todas
esas huellas; incluye `latex/main.pdf` y ambos PDF de 281 páginas del cierre de
identidades, local y CI. No se reemplazó ningún entregable.

Las fuentes de audiencias se separan en cuatro archivos incluidos desde sus
capítulos, y el README describe su organización. El PDF preliminar se compiló
con el Makefile versionado en una copia aislada de 61 fuentes y figuras, con
caché de fuentes propia y logs duraderos. Esa copia conserva el corte anterior
a la conclusión de las verificaciones integradas.

La compilación preliminar terminó con código cero y produjo **286 páginas**,
**5 694 304 bytes**, formato carta, con SHA-256
`6410c53da20d4fe11b42391100626d563129f236588bcb1cd8dd15a8b371ef09`.
Se renderizaron y revisaron **34 páginas** de índices, caso de uso, apartados
modificados, transiciones y conclusiones pendientes. Una referencia gramatical
al apéndice se corrigió y se volvió a comprobar. Las 10 043 palabras revisadas
por coordenadas permanecen dentro del área segura; no se observaron recortes
ni superposiciones. No hay referencias o citas indefinidas ni glifos ausentes.
Persisten únicamente los avisos históricos de versalitas de Times New Roman y
0.11754 pt de desborde en el índice de tablas, sin defecto visual apreciable.
Las 61 fuentes del snapshot coincidieron con el árbol académico en esa
compilación. El PDF preliminar y su snapshot se conservaron sin reemplazo.
Después se añadió la evidencia HTTP/restauración, la interacción Qadra y las
métricas globales a las fuentes de la entrega final descrita a continuación.

### Compilación final, inspección y procedencia

Una segunda copia aislada de las 61 fuentes se compiló con el Makefile
versionado, LuaLaTeX, biber y glosarios. El nuevo PDF final contiene **289
páginas carta y 5 702 678 bytes**. Se publicó como
`output/pdf/TT2026-B136-audiencias-qadra-2026-09-16.pdf`, acompañado por
`.sources.json`, `.provenance.json` y `.sha256`. Su SHA-256 es
`762c91a46ea2c05dd82b1760686e121912817e85a462a079f32b562f2341e1dd`.

El snapshot registra el árbol de trabajo sobre
`869d3fbce8f0d20db1ec34f6175e3189fb85b45a`, vigente al copiar las fuentes,
y conserva las 61 huellas individuales. La publicación se verificó sobre
`3a9abcd4e354028178b7bf749ac7109f7730e5fb`; las fuentes académicas aún
estaban sin commit. Ninguno de esos identificadores se presenta como la
procedencia íntegra del PDF. Después de guardar las fuentes se cotejaron sus
61 huellas contra el commit de entrega `e13522cfaf4b2151e8fb470f0d0b9911d4f400cd`,
sin diferencias, y se asoció ese identificador en el sidecar de procedencia.
La compilación es local y no se atribuye a una nueva ejecución remota.

Se renderizaron y revisaron **50 páginas físicas**: 1-10, 61-62, 81-83,
116-117, 130-131, 134-137, 176-181, 204-208, 221-223, 261-266 y 283-289.
Incluyen índices, CU-08 y requisitos aprobados, arquitectura, diseño e
implementación de audiencias, interfaz, resultados, transición a conclusiones,
leyenda D.6 y matriz de alcance. La revisión independiente de Qadra precisó
que consultar de nuevo base y contexto es obligatorio ante conflictos de
revisión o contexto; los cambios de participantes o soportes exigen revisar
las referencias afectadas. Una corrección de composición evitó una palabra
huérfana del anexo, sin modificar el texto ni sus mediciones históricas; se
volvieron a inspeccionar las cinco páginas afectadas.

El control de coordenadas revisó **12 892 palabras** sin salidas del área
segura. No se observaron recortes, superposiciones, glifos ausentes ni
referencias o citas indefinidas. Todas las fuentes están incrustadas; persisten
únicamente las sustituciones históricas de versalitas de Times New Roman y
el desborde de 0.11754 pt del índice de tablas, sin defecto visible. La
compilación inicial y las dos correcciones terminaron con código cero.

Las 61 fuentes del PDF coinciden con la copia compilada y el árbol académico.
La comprobación final conserva **279 huellas**: las 149 protegidas y otros
130 archivos del preliminar, incluidos su PDF, fuentes, logs y renderizados.
Los artefactos anteriores, las conclusiones pendientes y la presentación
permanecen íntegros. Los resultados nuevos sustentan la programación y su
historia, sin declarar completos CU-08, RF-07 u OE-2 ni convertir la agenda
en evidencia de celebración, resultados, plazos o notificaciones.



## Identidades y perfiles tipificados: 15 de septiembre de 2026

Se actualizaron diseño, implementación, pruebas, manual de CLI y anexos para
separar la cuenta autora, la identidad representada local al expediente y la
ficha de rol. El reporte describe once perfiles, historia manual y tipificada,
referencias exactas a identidad y documentos, candidatos con revisión explícita,
canon SUBJ1/PART2 y declaración PCRED1 firmada fuera del servidor. Se explican
la publicación administrativa de confianza, las CRL, las comprobaciones después
del bloqueo transaccional y la conservación de evidencia histórica.

La CA interna se presenta como un perfil de demostración, sin atribuirle FIREL,
identidad civil acreditada, habilitación profesional o reconocimiento judicial.
La tabla y el diagrama originales de CU-06, así como el criterio aprobado de
FIREL, permanecen íntegros; el cumplimiento jurídico pendiente se explica en la
prosa técnica. Las identidades tampoco conceden acceso a cuentas. Se conserva
la separación entre las revisiones actuales y las referencias históricas que
sustentan una ficha o su firma.

### Evidencia de software descrita

El primer corte instrumentado conserva **1066 pruebas Rust aprobadas, cero
fallos y una externa ignorada**, con **21 968 / 23 773 líneas, 92.4 % global**,
y el corte de interfaz de **89 pruebas unitarias, 147 escenarios simulados y
ocho recorridos reales**. El párrafo de repetición distingue la campaña final:
**1066 / 0 / 1** tanto ordinaria como instrumentada, **21 967 / 23 773 líneas**
y **11 211 / 12 150** en infraestructura; los demás numeradores no cambiaron.
Dominio, aplicación e infraestructura superaron sus umbrales del 90 %.

La repetición web aprobó **90 pruebas unitarias, 147 escenarios simulados y
ocho recorridos reales**, estos últimos en 2.0 minutos de Playwright. La
prueba unitaria adicional contrasta mensajes de errores de la API. El binario
release conservó **11 673 656 bytes** y la CLI midió Argon2id en **615.7 ms**
de promedio de cinco corridas. Compilación, formato, Clippy, MSRV, política de
dependencias, demostración HTTP y restauración terminaron satisfactoriamente.
Estos resultados proceden del [informe técnico](verification-report.md);
la revisión documental no sustituye esas ejecuciones ni una evaluación con
usuarios. Las mediciones históricas conservan su fecha y denominador.

### Compilación, inspección y procedencia

Se ejecutó el Makefile versionado con LuaLaTeX, biber y glosarios en una copia
nueva de **60 fuentes y archivos de soporte**, sin reemplazar `latex/main.pdf`.
Después de incorporar las cifras de cierre, la compilación definitiva terminó
con código cero: **281 páginas carta y 5 673 845 bytes**. El artefacto local es
`output/pdf/TT2026-B136-identidades-qadra-2026-09-15.pdf`, acompañado por
`.sources.json`, `.sha256` y `.provenance.json`. Su SHA-256 es
`83c33c484c238bd7ae1980d1d46e9c3c796825d87dfda604cd2349d8316eb740`.

El manifiesto registra los hashes exactos y coincide con la copia compilada y
el árbol utilizado. La procedencia identifica una instantánea de trabajo sobre
`f5b8fe6b7dc37fade363377354bd6540e5bfed45`, con fuentes modificadas y nuevas
que aún no tenían commit al compilar. No atribuye íntegramente este PDF a ese
commit base. Una revisión posterior puede asociar el commit de entrega si
verifica que los 60 hashes se conservan. El PDF y sus archivos auxiliares
permanecen fuera de Git; esta evidencia no afirma una nueva compilación remota.

Se inspeccionaron **55 páginas físicas**: 1-10, 57-58, 116-117, 130-135, 157,
173-177, 198-201, 232-233, 238-239, 245-246, 248, 250-260 y 275-281. Cubren
índices, CU-06, arquitectura, datos, perfiles, contratos, resultados, CLI,
verificación independiente y anexos. Las tablas de perfiles y cobertura, la
trazabilidad y el listado D.6 se ampliaron para revisar legibilidad. La leyenda
D.6 permanece con sus 24 líneas de código. Tras la corrección final se
revisaron nuevamente las páginas 198 y 199; otras **49** conservaron sus
renderizados idénticos píxel a píxel, y se añadieron las cuatro del anexo C.

No se observaron recortes, superposiciones o leyendas separadas del contenido
nuevo. El control de coordenadas comprobó **13 413 palabras** sin salir del
área segura. No quedaron citas o referencias sin resolver, etiquetas duplicadas
ni glifos ausentes. Solo persisten el desborde histórico de **0.11754 pt** en
el índice de tablas y las sustituciones de versalitas de Times New Roman.
Las fuentes usadas están incrustadas; Times New Roman y DejaVu Sans Mono
conservan su selección.

Los **78 archivos protegidos anteriores** mantienen sus hashes, incluidos
**13 PDF**, los **34 archivos de presentación**, las fuentes académicas
aprobadas y los doce recursos originales de Qadra. Resumen, introducción con
objetivos y revisión de plataformas, marco teórico y conclusiones pendientes
no se editaron. La propuesta de ajuste de OE-2 conserva su condición pendiente
fuera del manuscrito. El anexo C ya distinguía la CRL capturada al sellar y su
vigencia temporal; se comprobó su coherencia sin modificarlo. Comparar ZIP
tras restaurar bajo el mismo código no promete identidad del contenedor entre
versiones de software, cuyo instructivo puede cambiar.

## Etapas y admisión de soportes: 15 de septiembre de 2026

Se actualizaron diseño, implementación, pruebas y anexo para describir adopción,
los dos avances ordinarios, precisión temporal, soportes exactos y revisión
procesal independiente. Las tablas de permisos y contratos incluyen consulta,
historia y comandos; el cierre administrativo bloquea los registros procesales.
CU-07 distingue selección de versión y carga, y conserva una carga confirmada
cuando se rechaza su uso como soporte. Su diagrama vectorial editable corrige
la referencia RF-05, conserva las relaciones del caso de uso y su requisito
de habilitar módulos; el PNG original permanece intacto.
El anexo identifica las pruebas de dominio, aplicación, PostgreSQL, parsers,
HTTP y Qadra. Los archivos nuevos se incluyen desde sus capítulos correspondientes.

La admisión PDF/DOCX se describe como perfil restringido en un proceso acotado,
con biblioteca qpdf 12.4.1 verificada, presupuestos compartidos y salida sin
handlers `atexit`. No se presenta como validación general de carga, conformidad
completa del formato, evaluación jurídica ni sandbox general. Recursos,
audiencias, plazos e identidad tipificada conservan sus pendientes.

El nuevo corte reproduce **933 pruebas Rust aprobadas, cero fallos y una externa
ignorada**, incluidas las mismas pruebas bajo instrumentación. La cobertura es
**15 817 / 17 064 líneas, 92.7 % global**, con los tres umbrales obligatorios
aprobados. Qadra registra 56 pruebas unitarias, 129 escenarios simulados y seis
recorridos reales finales en aproximadamente 1.5 minutos. El capítulo distingue
estos resultados de los anteriores y de la evaluación con usuarios pendiente.
La calibración CLI fresca de Argon2id, 371.7 ms en cinco corridas, se identifica
fuera de banda; la medición histórica de 529.4 ms conserva su fecha y entorno.
La evidencia técnica detallada está en [el informe de verificación](verification-report.md).

### Compilación y revisión

Se ejecutó el Makefile versionado con LuaLaTeX, biber y glosarios sobre una copia
temporal de fuentes, con caché de fuentes separada. La primera compilación
permitió localizar desbordamientos de rutas en los párrafos nuevos. Se corrigió
su composición y una revisión independiente comprobó CU-07 e índices. Después
de conciliar el rechazo de soportes y corregir el diagrama, la compilación definitiva terminó
con código cero: **272 páginas carta, 5 638 644 bytes**.

Artefacto local: `output/pdf/TT2026-B136-etapas-qadra-2026-09-15.pdf`, acompañado
de `.sources.json` y `.sha256`. SHA-256:
`3d09d21cc6dcd44f25afb419c698255927de21d529414c2e7c8b41e192100d1f`.
Las **55 fuentes y archivos de soporte** del manifiesto coinciden con los archivos
del repositorio utilizados para compilar. El PDF permanece fuera de Git; el
workflow Documents conserva la reproducción desde fuentes.

Se revisaron las páginas físicas **161–166, 177–180, 191–195, 207–209 y 266–272**
mediante renderizados PNG, con ampliación de las tablas nuevas. Contratos,
porcentajes, rutas, párrafos, referencias y límites de página son legibles.
La tabla de cobertura conserva sus numeradores y denominadores. El anexo tiene
sección propia y el capítulo de conclusiones conserva sus marcadores pendientes.

La revisión independiente comprobó **1–10, 59–60 y 141** en el PDF definitivo,
incluidos portada, índices, tabla y diagrama CU-07 y matriz de permisos. Son
**38 páginas distintas** inspeccionadas entre ambas revisiones. Los 25
renderizados de implementación, pruebas y anexo permanecen idénticos píxel a
píxel después de corregir CU-07; el diagrama final también fue ampliado.

El log final no contiene referencias/citas sin resolver, caracteres ausentes
ni nuevos desbordamientos. Permanece el aviso histórico de **0.11754 pt** en
la lista de tablas y las sustituciones ya existentes de versalitas de Times
New Roman. Las fuentes usadas están incrustadas; Times New Roman y DejaVu Sans
Mono conservan su selección. No se modifica el diseño general del documento.

El manifiesto de preservación confirmó **53 archivos anteriores idénticos**,
incluidos los PDF existentes, el paquete de presentación, marcas y fuentes
académicas protegidas. Resumen, objetivos, estado del arte/marco y conclusiones
no se editaron. La propuesta de corrección del criterio OE-2 queda separada en
[alcance de recursos](procedural-resources-scope.md); no está aplicada al reporte.


## Coordinación de fichas: 15 de septiembre de 2026

Se actualizaron implementación, pruebas y README para describir la coordinación
entre una mutación confirmada y las consultas que actualizan su ficha. Sellado,
anexado, clasificación y edición de participantes mantienen ocupados los controles
correspondientes hasta terminar sus lectores dependientes. La carga inicial espera
el listado antes de montar clasificación e historial; editar participantes conserva
el historial abierto. Un cambio de estado confirmado retira el detalle y espera
el listado. Un fallo transitorio de consulta conserva el estado confirmado; una
denegación retira los datos y las respuestas tardías no los restablecen.

El alcance se limita a acciones y consultas de cada ficha; no modifica el límite
compartido del backend ni garantiza capacidad entre vistas o usuarios. Las
regresiones controladas no atribuyen una respuesta HTTP concreta a los fallos
remotos anteriores cuyo estado no se capturó. Las mediciones del 14 de septiembre,
incluidos 88 escenarios simulados, 770 pruebas Rust y cobertura global de 93.4 %,
conservan su condición histórica.

El corte final aprobó **48 pruebas unitarias, 109 escenarios simulados en 36.5
segundos y cuatro recorridos reales en 57.6 segundos**. Los recorridos cubren
administración penal, participantes, clasificación y versiones. Formato y
compilación terminaron satisfactoriamente; esta última tomó 1.45 segundos sin
advertencias. La auditoría técnica comprobó 156 archivos fuente/configuración
ASCII y menores de 400 líneas, con máximo de 382; los 337 hashes de Rust y
migraciones permanecieron idénticos y sus suites no se repitieron. Los doce
recursos originales de Qadra también conservaron sus bytes. La evidencia y sus
límites se detallan en el [informe técnico](verification-report.md); no constituyen
una evaluación de usabilidad o capacidad de producción.

### Compilación y preservación

Se ejecutó el Makefile de LuaLaTeX en una copia nueva con caché de fuentes separada.
Terminó con código cero: **266 páginas**, formato carta y **5 675 576 bytes**.
El artefacto nuevo es `output/pdf/TT2026-B136-perfil-penal-qadra-2026-09-15-coordinacion.pdf`, con manifiesto de fuentes y SHA-256,
sin sobrescribir la salida del 14 ni añadir el PDF a Git. Su hash es
`376c0a13689aa0c425c88d46f7f5b18a48507e03f582a02e5e0d7b00e8082eaa`. Los **51 archivos de fuentes y soporte** coinciden entre árbol,
copia compilada y manifiesto.

La comparación con el PDF anterior identificó las páginas modificadas y los
cambios de paginación. Se renderizaron y revisaron **106 páginas**, incluidos
índices y páginas adyacentes; la revisión se repartió entre dos lectores y se
inspeccionaron también las páginas de implementación, pruebas y una tabla de
verificación a escala individual. No se observaron recortes, superposiciones o
rutas ilegibles. El control de coordenadas pasó y no quedaron referencias o citas
indefinidas, etiquetas duplicadas ni glifos ausentes. Solo permanecen el desborde
histórico de 0.11754 pt del índice de tablas y las sustituciones de versalitas.

Los **once PDF anteriores** y los **34 archivos de presentación** conservan sus
hashes: 43 rutas únicas de artefactos protegidos. Resumen aprobado, objetivos,
revisión de plataformas, marco teórico y conclusiones pendientes permanecen
idénticos. Esta verificación corresponde al nuevo artefacto local y no afirma
una nueva compilación remota.

## Perfil y administración penal: 14 de septiembre de 2026

Se actualizaron alcance, diseño, modelo de persistencia y permisos, contratos
HTTP, implementación, recuperación, interfaz Qadra, pruebas, cobertura y anexo
de trazabilidad. La descripción sigue [ADR-0022](adr/0022-audited-penal-case-administration.md)
y el [contrato HTTP de administración](case-administration-api.md): raíz estable,
perfil penal manual y revisiones administrativas independientes de documentos
y participantes. La API básica conserva su respuesta de cuatro campos y las
lecturas auditadas muestran título y referencia actuales sin exponer el perfil
a Client. El alta penal confirma R1 activa y registro inicial de Investigación;
la básica conserva un perfil pendiente. Completar un perfil legado o básico
no fabrica una etapa, autor o fecha anteriores.

Se documentaron normalización de información multilineal, delitos ordenados,
canon CADM1 de hasta 12 717 bytes, digest de valores, autor UUID/correo capturado,
revisión esperada, unicidad literal de identificadores actuales y control de
estado dentro de la transacción. Las funciones SQL y los guards de arranque
conservan su frontera de confianza: no autentican cualquier DDL administrativo
ni detectan un rollback íntegro coherente. El cierre organiza el trabajo del
despacho y bloquea mutaciones; no concluye un proceso judicial. Transiciones,
adopción de etapa previa, identidad jurídica, FIREL, duplicidad verificada,
audiencias, plazos y usabilidad siguen requiriendo trabajo propio.

El corte reproduce **770 pruebas Rust aprobadas, cero fallos y una externa
ignorada**. Las 66 pruebas nuevas se distribuyen en 24 de dominio/aplicación,
12 HTTP y 30 de infraestructura. La ejecución instrumentada obtuvo **12 613 de
13 506 líneas cubiertas, 93.4 % global**, con los tres umbrales obligatorios
aprobados. Formato, compilación, Clippy, política de dependencias, CLI y campaña
HTTP terminaron satisfactoriamente. El entorno fue Rust/Cargo 1.94.0, PostgreSQL
18.6, Valkey 8.1.9, OpenSSL 3.5.7, Node.js 22.22.2 y npm 10.9.7. Los resultados
anteriores, incluidos 704 pruebas y 92.9 % del directorio, conservan sus cifras
históricas y no se suman a este corte.

La restauración conserva **cinco expedientes, once revisiones administrativas
y dos registros iniciales**, además de cinco raíces documentales, seis versiones,
tres revisiones de clasificación y dos participantes con seis revisiones.
Coinciden filas completas, detalle e historia, autoría, fechas y ZIP de las dos
versiones. La reconciliación del recibo posterior a las revisiones no altera
las filas ni la cadena. Los cuatro documentos y 63 eventos identifican el origen
importado y su prefijo, no el total final de auditoría; los 51 eventos previos
mantienen su fecha histórica. La reconstrucción legacy es un fixture documental
sobre baselines explícitas con hechos de creación conocidos, no una prueba de
conversión íntegra del historial administrativo actual. La preservación íntegra
se verifica con el respaldo completo posterior.

Qadra aprobó **45 pruebas unitarias, 88 escenarios simulados en 35.0 segundos
y cuatro recorridos reales en 1.3 minutos**, además de formato y compilación.
El recorrido penal contrasta R1 y R0, perfiles pendientes sin etapa inventada,
conflicto de edición con borrador conservado, cierre concurrente, bloqueo de
mutaciones, evidencia histórica, reapertura, reingreso y revocación. Paralegal
consulta la historia y Client no solicita el contrato sensible. El ZIP previo
permanece idéntico y no se registran errores JavaScript. El área de texto final
fue comprobada en móvil y con servicios reales. Estos resultados proceden del
[informe técnico](verification-report.md), se cuentan por separado de Rust y
no sustituyen una evaluación de usabilidad.

### Compilación y revisión documental

Se ejecutó el Makefile versionado con LuaLaTeX, biber y glosarios en una copia
temporal nueva, con caché de fuentes separada. La compilación final terminó con
código cero: **265 páginas**, formato carta y **5 672 258 bytes**. El nuevo
artefacto es `output/pdf/TT2026-B136-perfil-penal-qadra-2026-09-14.pdf`, acompañado
de manifiesto de fuentes y SHA-256, sin añadirlo a Git. Su hash es
`870a21bec1ef7aeb932efbd91736b8a1a5e03b80cd32b0d0ab5cd821eff8b147`.
Los **51 archivos de fuentes y soporte** del manifiesto coinciden con la copia
compilada y con el árbol actual.

Se renderizaron y revisaron **75 páginas** afectadas y adyacentes: índices,
alcance, modelo, permisos, implementación, contratos, migración/restauración,
interfaz, pruebas, cobertura y trazabilidad. Se corrigió una línea desbordada
con dos rutas frontend mediante alineación izquierda local. Las tablas y rutas
finales son legibles, sin recortes ni superposiciones; se inspeccionaron también
por separado las páginas de contrato, interfaz, pruebas, cobertura y anexo.
No quedaron citas o referencias indefinidas, etiquetas duplicadas ni glifos
ausentes. El control de coordenadas no detectó palabras fuera del área segura.
Solo permanecen las sustituciones históricas de versalitas de Times New Roman
y el desborde de 0.11754 pt del índice de tablas, sin defecto visual apreciable.

Resumen aprobado, objetivos, revisión de plataformas, marco teórico y
conclusiones pendientes conservan sus hashes. Los **diez PDF anteriores**,
incluido `latex/main.pdf`, y los **34 archivos de presentación** permanecen
idénticos: 42 rutas únicas de artefactos protegidos. La presentación no se
modificó. Esta evidencia corresponde a la compilación local y no afirma una
nueva ejecución remota ni cumplimiento jurídico integral.

## Directorio de participantes: 14 de septiembre de 2026

Se actualizaron alcance, diseño, modelo y permisos, implementación HTTP,
restauración, interfaz, matriz de pruebas, anexo de trazabilidad, bibliografía
y README. La descripción sigue [ADR-0021](adr/0021-audited-case-participants.md):
fichas por expediente independientes de cuentas y asignaciones, valores manuales,
canon PART1, revisión esperada, autor UUID y correo capturados, y cambio exclusivo
de estado dentro de la transacción. Las consultas seleccionan la revisión actual
antes de filtrar y usan cursores exclusivos. Los resultados se devuelven después
de confirmar auditoría; los comandos retornan su propia instantánea confirmada.

El directorio conserva su condición de cumplimiento parcial del catálogo:
identidad e identificadores por tipo, duplicidad de persona verificada y rol,
órgano/FIREL y expediente penal activo permanecen abiertos. Se agregaron referencias
primarias al CNPP consolidado y a los acuerdos OAJ de 2026 fuera del marco teórico
protegido. La explicación distingue sujetos, partes, etiquetas manuales, cuentas
y firma. Identifica la abrogación del Acuerdo 12/2020 y la derogación parcial del
acuerdo conjunto 1/2013 dentro de la competencia del OAJ, sin presentar esos
antecedentes como disposiciones actuales ni cambiar los criterios aprobados.

El nuevo corte registra 704 pruebas Rust aprobadas, cero fallos y una externa
ignorada; son 60 pruebas adicionales: 21 de dominio/aplicación, 12 HTTP y 27 de
infraestructura. La cobertura instrumentada es de 10 994 sobre 11 838 líneas
(92.9 %), con los tres umbrales obligatorios aprobados. Formato, compilación,
Clippy, política de dependencias, CLI y demostración API terminaron con código
cero. Se documentó también el cambio de `chacha20` 0.10.1 retirado a 0.10.2,
con la corrección SSE2 publicada por RustCrypto; no se atribuye un aviso RUSTSEC.
Las mediciones previas de consultas, versiones y clasificación mantienen sus
conteos y denominadores históricos.

La restauración conserva dos participantes y seis revisiones, cinco raíces
documentales y seis versiones de contenido, además de tres revisiones de
clasificación. Las filas completas, valores, UUID y correos históricos, fechas
y ZIP de evidencia coinciden antes y después. Los cuatro documentos y 51 eventos
identifican el origen importado y su prefijo, no el inventario final. Estos
resultados proceden del [informe técnico](verification-report.md) y se distinguen
de la compilación y revisión documental.

La interfaz aprobó 38 pruebas unitarias, 65 escenarios simulados en 24.6 segundos
y tres recorridos con servicios reales en 39.4 segundos, además de formato y
compilación. El directorio conserva borradores, confirma conflictos de edición
y archivo explícitamente, recupera la revisión siete tras reingreso y alcanza
la octava desde la cuenta del litigante. Paralegal consulta la historia, Cliente
no solicita el directorio y la revocación elimina datos visibles. El ZIP previo
permanece idéntico, sin errores JavaScript. Los tiempos pertenecen a las pruebas;
no se presentan como latencia de producto ni como evaluación de usabilidad.

### Compilación y revisión documental

Se ejecutó el Makefile versionado con LuaLaTeX, biber y glosarios en una copia
temporal nueva de las fuentes, con caché de fuentes separada. La compilación final
terminó con código cero: **256 páginas**, formato carta y **5 643 905 bytes**.
El artefacto local es `output/pdf/TT2026-B136-participantes-qadra-2026-09-14.pdf`,
acompañado de manifiesto de fuentes y SHA-256, sin incorporarlos a Git. Su hash es
`a1034aad4df60ceaf4e9b5bf86605e813242fe4833993899424fe0ff469404c3`.
Los 51 archivos del manifiesto compilado coinciden con las fuentes actuales.

Se renderizaron y revisaron **65 páginas** afectadas y adyacentes: índices,
alcance, diseño, modelo, permisos, implementación, contratos HTTP, límites
jurídicos, restauración, interfaz, pruebas, cobertura, bibliografía y trazabilidad.
Se corrigieron las líneas largas del nuevo anexo con alineación izquierda local;
las rutas y tablas resultantes son legibles, sin recortes ni superposiciones.
La revisión final no encontró citas o referencias indefinidas, etiquetas duplicadas
ni glifos ausentes; el control de coordenadas no detectó palabras fuera del área
segura. Solo permanecen las sustituciones históricas de versalitas de Times New
Roman y el desborde de 0.11754 pt del índice de tablas, sin defecto visual apreciable.

Resumen, objetivos, revisión de plataformas, marco teórico y conclusiones
pendientes conservan su contenido. Los nueve PDFs anteriores, incluido
`latex/main.pdf`, y los 34 archivos de presentación mantienen sus hashes;
son 41 rutas únicas al incluir los PDFs externos a presentación. La presentación
no se modificó. La evidencia documental corresponde a esta compilación local
y no afirma una nueva ejecución remota.

## Clasificación documental auditada: 14 de septiembre de 2026

Se actualizaron introducción, modelo de datos, permisos, implementación,
pruebas, anexo de trazabilidad y README. La prosa sigue
[ADR-0020](adr/0020-audited-document-classification.md): clasificación manual,
canon compartido, revisión cero, UUID y correo histórico del autor, carga
multipart atómica, conflictos y filtros sobre la clasificación actual.
Se distinguen las revisiones organizativas de las versiones cifradas y de
su evidencia. La explicación incluye consumo completo del cuerpo HTTP y
restauración con referencias SQL calificadas al esquema de instalación.

Los resultados históricos de consultas (539 pruebas y 91.0 %) y versiones
(577 y 91.7 %) se conservan. El nuevo corte registra 644 pruebas Rust aprobadas,
una externa ignorada y 9 854 de 10 670 líneas cubiertas (92.4 %); los tres
umbrales obligatorios aprobaron. Formato, compilación, Clippy, política de
dependencias, CLI y demostración HTTP terminaron satisfactoriamente. El inventario
restaurado contiene cinco raíces, seis versiones y tres revisiones organizativas;
conserva procedencia, filas completas y ZIP históricos. Los 51 eventos identifican
el prefijo importado y no se presentan como el total final.

La interfaz registra 34 pruebas unitarias, 47 escenarios con respuestas simuladas
y dos recorridos contra servicios reales. Clasificar, resolver un conflicto,
vaciar valores y reingresar preserva las cinco revisiones del recorrido nuevo
y los ZIP de ambas versiones. Estos resultados proceden del
[informe técnico de verificación](verification-report.md) y no se contabilizan
como comprobaciones documentales. Los casos de uso de carga y consulta del
producto permanecen parciales por sus políticas de formatos, entrega de contenido
sin sellado previo y alerta al Owner, además de la usabilidad pendiente.

### Compilación y revisión documental

Se ejecutó el Makefile versionado con LuaLaTeX, biber y glosarios en una copia
temporal nueva de las fuentes, con caché de fuentes separada. La compilación final
terminó con código cero y produjo **248 páginas**, formato carta, **5 616 629
bytes**. El artefacto local se conserva como
`output/pdf/TT2026-B136-clasificacion-qadra-2026-09-14.pdf`, acompañado de un
manifiesto de fuentes y un archivo SHA-256, sin añadirlos a Git. Su hash es
`701557b342c771cc3f24a293c8c83476bd5ac9c101cbf72d29d093e257805cfa`.
Los 51 archivos del manifiesto compilado coinciden con el árbol de trabajo.

Se renderizaron y revisaron **61 páginas** afectadas y adyacentes: índices,
alcance, modelo, permisos, implementación, contratos, restauración, interfaz,
pruebas, cobertura y trazabilidad. Se corrigió un desbordamiento del código de
error largo y se compactó la prosa de diseño para evitar cinco líneas aisladas
al cerrar el capítulo. Las tablas y rutas son legibles, sin recortes,
superposiciones, referencias indefinidas, etiquetas duplicadas ni glifos ausentes.
La comprobación de coordenadas no encontró palabras fuera del área segura.
Solo permanecen las sustituciones históricas de versalitas de Times New Roman
y el desborde de 0.11754 pt del índice de tablas, sin defecto visual apreciable.

Resumen, objetivos, revisión de plataformas, marco teórico y conclusiones
pendientes coinciden con su contenido anterior. Los ocho PDFs previos, incluido
`latex/main.pdf`, y los 34 archivos preexistentes de presentación mantienen sus
hashes; son 40 rutas únicas al incluir los PDFs externos a presentación.
No se modificó la presentación. Esta evidencia corresponde a una compilación
local y no afirma una nueva ejecución remota.

## Versiones documentales inmutables: 14 de septiembre de 2026

Se actualizaron introducción, diseño, implementación, pruebas, anexo de
trazabilidad y README del reporte. La descripción integra las versiones como
comportamiento disponible: identidad estable, DEK propia y AAD por UUID/versión,
selección explícita, anexado con cabeza esperada, filtros sobre la versión
máxima, historial descendente y primera versión realmente disponible. El ejemplo
de importación conserva la versión 7 y permite añadir la 8 sin inventar historia.
La tabla de permisos incorpora historial y anexado. Las acciones sin número
resuelven una única versión y conservan esa instantánea si aparece un sucesor.

Se describen la migración repetible, reconciliación y restauración, junto con
la selección histórica y los conflictos de carga en Qadra. El texto distingue
el binding de cifrado de la firma y el sello sobre contenido, y aclara que el
AAD no detecta la restauración íntegra de un estado antiguo válido. Clasificación,
gestión procesal completa y usabilidad permanecen pendientes.

Las cifras históricas de consultas se conservan: 539 pruebas y 91.0 % de
cobertura. El nuevo corte documenta 577 pruebas Rust aprobadas, una externa
ignorada y 8 908 de 9 719 líneas cubiertas (91.7 %), con los tres umbrales
obligatorios aprobados. La interfaz registra por separado 28 pruebas unitarias,
32 de navegador simulado y un escenario con servicios reales, además de
compilación y formato. El recorrido de versiones conserva el ZIP de la primera
versión y recupera ambas tras cerrar sesión y autenticar de nuevo. Los resultados
funcionales proceden del [informe de verificación](verification-report.md);
esta actualización académica no los cuenta como comprobaciones del documento.

### Compilación y revisión documental

Se ejecutó el Makefile versionado con LuaLaTeX, biber y glosarios en una copia
temporal nueva de las fuentes, con caché de fuentes separada. La compilación
final terminó con código cero y produjo **242 páginas**, formato carta,
**5 594 306 bytes**. El PDF local se conserva como
`output/pdf/TT2026-B136-versiones-qadra-2026-09-14.pdf`, acompañado de manifiesto
de fuentes y archivo SHA-256, sin añadirlos a Git. Su hash es
`b2dad626675612833fe4f25e9ab10b566e44d6afeea08aa265e88bde1b0263da`.
Los 51 archivos del manifiesto compilado coinciden con el árbol de trabajo.

Se renderizaron y revisaron **45 páginas** afectadas y adyacentes: índices,
alcance, modelo de datos, cifrado, permisos, implementación, contratos, interfaz,
pruebas, cobertura y trazabilidad. Se corrigieron desbordes de rutas y campos
largos y se compactó la prosa de diseño para evitar un cierre de capítulo casi
vacío. Las tablas y rutas son legibles, con referencias resueltas y sin recortes,
superposiciones, etiquetas duplicadas ni glifos ausentes. La comprobación de
coordenadas no encontró palabras fuera del área segura. Solo permanecen las
sustituciones históricas de versalitas de Times New Roman y el desborde de
0.11754 pt del índice de tablas, sin defecto visual apreciable.

Resumen aprobado, objetivos, revisión de plataformas, marco teórico y
conclusiones pendientes coinciden con su contenido anterior. Los siete PDFs
anteriores, incluido `latex/main.pdf`, y los 34 archivos preexistentes de
presentación mantienen sus hashes; son 39 rutas únicas al incluir los PDFs
ubicados fuera de presentación. No se modificó la presentación. Esta evidencia
corresponde a la compilación local y no afirma una nueva ejecución remota.

## Consultas documentales e interfaz Qadra: 14 de septiembre de 2026

Se actualizaron la descripción del alcance implementado, diseño, implementación,
pruebas y anexo de trazabilidad para las consultas autorizadas y la integración
Qadra. Los objetivos aprobados, resumen, marco teórico y conclusiones pendientes
se conservaron; se comprobaron contra su contenido anterior. Las referencias a
519 pruebas y 46 eventos mantienen su fecha histórica; el ensayo nuevo registra
539 pruebas, 51 eventos y 91.0 % de cobertura Rust. La matriz del catálogo
funcional completo y usabilidad se identifican como parciales, sin confundirlas
con la evidencia criptográfica ya disponible.

La compilación usó el Makefile versionado en una copia temporal de las fuentes,
con LuaLaTeX, biber y glosarios y una caché temporal de fuentes. Produjo **239
páginas**, **5 577 936 bytes**, en formato carta. Los 51 archivos del manifiesto
de fuentes coinciden con el árbol de trabajo. El PDF nuevo se conserva como
`output/pdf/TT2026-B136-consultas-qadra-2026-09-14.pdf`, junto con su manifiesto;
ninguno se añade a Git. Su SHA-256 es
`d68c08da5ef7ca9bb70aecdbde6935556f7e951895ad9e3d72424473b42ce5f4`.
Los cinco entregables anteriores comprobados por hash permanecen intactos,
incluido `latex/main.pdf`; la presentación no se modificó.

Se renderizaron y revisaron 37 páginas de los apartados afectados y sus páginas
adyacentes: alcance, diseño, contratos, consultas, interfaz, pruebas, cobertura,
matriz, conclusiones pendientes y anexo. Se corrigió un desborde nuevo de rutas
largas antes del render final. No hay citas o referencias indefinidas, glifos
ausentes, etiquetas duplicadas ni texto fuera del área segura. Se conservan las
sustituciones de versalitas de Times New Roman y el desborde histórico de
0.11754 pt en el índice de tablas, sin defecto visual apreciable.

Esta comprobación documenta el PDF local; no afirma una nueva compilación remota.
La evidencia funcional y los resultados del navegador se registran separadamente
en [el informe de verificación](verification-report.md). El escenario real de
navegador no constituye una evaluación de usabilidad con personal del despacho.

## Actualización anterior: 12 de septiembre de 2026

Fecha: 12 de septiembre de 2026. Base de software: `57f9076`, que integra
la autorización documental por expediente y la auditoría transaccional.
La evidencia funcional procede del [informe de verificación](verification-report.md).
Esta actualización modifica documentación y composición del reporte; no cambia
el software ni presenta una nueva ejecución local de la suite Rust.

## Contenido contrastado

- Introducción y diseño: alcance del backend, sesiones opacas, asignaciones,
  permisos por recurso, PostgreSQL y separación de administración, integrados
  en sus apartados mediante prosa académica continua.
- Implementación: frontera transaccional, límites de concurrencia,
  preservación de evidencia y decisiones técnicas de los adaptadores.
- Pruebas y matriz: 520 funciones inventariadas, 519 aprobadas, una ignorada;
  cobertura del 90.7 %, con PostgreSQL y Redis reales en la ejecución documentada.
- Recuperación: ensayos de dos servidores, migración de cuatro documentos y
  restauración que conserva 46 eventos históricos y la evidencia capturada.
- Anexos: comandos administrativos e importación, diferencia entre exportación
  y verificación, y política temporal de certificados y TSA.

El resumen coincide byte por byte con el original aprobado. Se conservaron
los objetivos, la revisión de plataformas y el contenido del marco teórico;
en este último solo se ajustó el tamaño tipográfico de una tabla.
Las conclusiones conservan el contenido pendiente previo y se completarán
al finalizar el proyecto. Las cifras de rendimiento y transcripciones
anteriores conservan su fecha. La asignación de usuarios no se presenta como
gestión procesal completa, y la TSA local no se equipara a un prestador
independiente. La presentación no forma parte de esta actualización.

## Comprobaciones documentales ejecutadas

Se compiló el Makefile versionado con LuaLaTeX, biber y makeglossaries en una
copia temporal de las fuentes. La caché de fuentes se situó en un directorio
escribible mediante `TEXMFCACHE` y `TEXMFVAR`; no se alteró la selección de fuentes.
El flujo equivalente desde el repositorio es:

```bash
make -B -C latex
pdfinfo latex/main.pdf
pdftotext -layout latex/main.pdf reporte.txt
pdftoppm -png latex/main.pdf pagina
git diff --check
```

La compilación final terminó con código cero y produjo **234 páginas**,
formato carta, en **5 559 649 bytes**. El manifiesto de las fuentes y figuras
de la copia compilada coincide con la copia de trabajo. No hay referencias o
citas indefinidas, etiquetas duplicadas ni glifos ausentes.

Se revisaron la composición general en miniaturas y páginas de detalle de
resumen, introducción, diseño, tablas, rutas HTTP, transacciones, migración,
resultados, cobertura, conclusiones pendientes, comandos y matriz.
Se corrigieron identificadores que desbordaban, la altura del encabezado,
una tabla demasiado alta y finales de capítulo con apenas unas líneas.
La tabla de bibliotecas se compactó y el diagrama de despliegue se representó
como fuente LaTeX, con sesiones opacas y la TSA local, conservando su imagen
original. Se comprobó la legibilidad del diagrama y de las tablas modificadas.
La extracción de posiciones no encontró palabras fuera del área segura de la
página. No se observaron recortes ni superposiciones en el contenido modificado.

El log conserva un aviso de desborde horizontal de **0.12 pt** en el índice de
tablas y las sustituciones de versalitas de Times New Roman ya existentes;
no impiden la compilación ni generan un defecto visual apreciable.

## Compilación remota

El instalador de fuentes de Ubuntu falló al descargar una familia adicional
mediante un espejo HTTP. Se sustituyó por la descarga HTTPS del archivo original
de Times New Roman, con SHA-256 fijo, tiempos y reintentos acotados. Se verificó
el archivo, se extrajeron sus cuatro estilos y se compararon sus hashes con las
fuentes utilizadas localmente. La sintaxis YAML y todos los bloques shell del
workflow se comprobaron antes de publicarlo. La decisión y procedencia están en
[ADR-0017](adr/0017-report-font-installation.md).

## Conservación y publicación

Se conservó una copia de las fuentes previas antes de editar. Los 36 archivos
preexistentes de presentación y entregables comparados por hash permanecen
intactos. El PDF anterior se preservó en la carpeta local de entregables antes
de actualizar `latex/main.pdf` y producir la copia de entrega.

El PDF generado sigue ignorado por Git. La integración conserva únicamente
fuentes y documentación; el workflow [Documents](../.github/workflows/documents.yml)
compila esas fuentes para revisión y adjunta el reporte al publicar una release.
[AGENTS.md](../AGENTS.md) y [CONTRIBUTING.md](../CONTRIBUTING.md) establecen que
la actualización académica y su revisión forman parte del cierre funcional,
preservando el resumen aprobado y las conclusiones pendientes.
