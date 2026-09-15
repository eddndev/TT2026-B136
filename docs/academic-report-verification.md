# Verificación de la actualización académica

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
