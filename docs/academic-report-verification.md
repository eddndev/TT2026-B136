# Verificación de la actualización académica

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
