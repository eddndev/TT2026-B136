# Verificación de la actualización académica

Fecha: 12 de septiembre de 2026. Base de software: `57f9076`, que integra
la autorización documental por expediente y la auditoría transaccional.
La evidencia funcional procede del [informe de verificación](verification-report.md).
Esta actualización modifica documentación y composición del reporte; no cambia
el software ni presenta una nueva ejecución local de la suite Rust.

## Contenido contrastado

- Resumen y alcance: backend evaluado, objetivos originales y funciones pendientes.
- Diseño e implementación: sesiones opacas, asignaciones, permisos por recurso,
  PostgreSQL, auditoría, límites de concurrencia y separación de administración.
- Pruebas y matriz: 520 funciones inventariadas, 519 aprobadas, una ignorada;
  cobertura del 90.7 %, con PostgreSQL y Redis reales en el corte reproducido.
- Recuperación: ensayos de dos servidores, migración de cuatro documentos y
  restauración que conserva 46 eventos históricos y la evidencia capturada.
- Conclusiones: resultados, contribuciones, limitaciones y trabajo posterior.
- Anexos: comandos administrativos e importación, diferencia entre exportación
  y verificación, y política temporal de certificados y TSA.

Las cifras de rendimiento y transcripciones anteriores conservan su fecha.
La asignación de usuarios no se presenta como gestión procesal completa, y la
TSA local no se equipara a un prestador independiente. Los originales de la
bibliografía y figuras se conservan. La presentación no forma parte de esta
actualización.

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

La compilación final terminó con código cero y produjo **236 páginas**, formato
carta, en **5 939 272 bytes**. Se comprobó que el manifiesto de fuentes y figuras
de la copia compilada coincide con la copia de trabajo. No hay referencias o
citas indefinidas, etiquetas duplicadas, glifos ausentes ni marcadores pendientes
en las conclusiones.

Se revisó la composición general en miniaturas y, tras los ajustes finales,
23 páginas de detalle: resumen, tabla redimensionada, diseño, rutas HTTP,
transacciones, migración, resultados, cobertura, conclusiones, comandos y matriz.
Se corrigieron identificadores que desbordaban, la altura del encabezado,
una tabla demasiado alta y dos finales de capítulo con apenas unas líneas.
La extracción de posiciones no encontró palabras fuera del área segura de la
página. No se observaron recortes ni superposiciones en las páginas revisadas.

El log conserva un aviso de desborde horizontal de **0.12 pt** en el índice de
tablas y las sustituciones de versalitas de Times New Roman ya existentes;
no impiden la compilación ni generan un defecto visual apreciable.

## Conservación y publicación

Se conservó una copia de las fuentes previas antes de editar. Los 36 archivos
preexistentes de presentación y entregables comparados por hash permanecen
intactos. El PDF anterior se preservó en la carpeta local de entregables antes
de actualizar `latex/main.pdf` y producir la copia de entrega.

El PDF generado sigue ignorado por Git. La integración conserva únicamente
fuentes y documentación; el workflow [Documents](../.github/workflows/documents.yml)
compila esas fuentes para revisión y adjunta el reporte al publicar una release.
[AGENTS.md](../AGENTS.md) y [CONTRIBUTING.md](../CONTRIBUTING.md) establecen que
la actualización académica y su revisión forman parte del cierre funcional.
