# Verificación de la actualización académica

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
