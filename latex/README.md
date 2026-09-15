# Trabajo Terminal 2026-B136 — Fuente LaTeX

Migración a LaTeX del documento del TT *«Prototipo de un sistema web para la
gestión de expedientes digitales y actividad procesal de un despacho legal,
con servicios de integridad y no repudio.»* (ESCOM-IPN).

**Autores:** Eduardo Alonso Sánchez · Hatziry Vitales Herrera
**Directora:** Sandra Díaz Santiago

## Compilar

Requiere TeX Live (con LuaLaTeX, biber, makeglossaries). Compila con:

```bash
make            # latexmk -lualatex (corre biber + makeglossaries + N pasadas)
make watch      # recompila al guardar
make clean      # borra auxiliares
make purge      # borra auxiliares y el PDF
```

o manualmente: `latexmk -lualatex main.tex`. El motor **debe ser LuaLaTeX**
(se usa `fontspec`, fuentes Unicode y `\newunicodechar`).

## PDF en GitHub Releases

`main.pdf` es un resultado local ignorado por Git. Las fuentes LaTeX, las
figuras y la bibliografía siguen versionadas. El workflow
[`Documents`](../.github/workflows/documents.yml) instala TeX Live, biber,
glosarios, Times New Roman y DejaVu Sans Mono en Ubuntu 24.04.

- En PR y cambios de documentos en `main`, compila y conserva el artefacto
  `report-pdf` durante 14 días para revisión.
- Al publicar una release, compila la revisión de su etiqueta y adjunta
  `TT2026-B136-reporte.pdf`, `SHA256SUMS` y
  `SOURCE_COMMIT`. Las prereleases también ejecutan este flujo.
- Para repetir la publicación en una release existente, ejecutar `Documents`
  manualmente desde Actions e indicar su etiqueta en `release_tag`. La
  compilación usa esa etiqueta; los adjuntos del mismo nombre se reemplazan.
  No crea etiquetas ni releases automáticamente y requiere adjuntos editables.

La publicación usa el `GITHUB_TOKEN` del job de subida con permiso
`contents: write`; la compilación solo tiene acceso de lectura. Las fuentes
Times New Roman se extraen del archivo original `times32.exe`, descargado por
HTTPS y comprobado contra un SHA-256 fijo antes de extraer sus cuatro estilos.
La procedencia y el procedimiento se documentan en
[ADR-0017](../docs/adr/0017-report-font-installation.md). Las fuentes no se copian
al repositorio. Si la descarga, su verificación o LaTeX fallan, no se publica
el PDF.

Quitar los PDF del seguimiento evita incorporar nuevos binarios. Los commits
históricos que ya los contienen no se reescriben.

## Actualización académica por entrega

El reporte debe acompañar los avances funcionales. Cada entrega actualiza los
apartados afectados de implementación, pruebas y anexos, además del contrato
HTTP, las decisiones de arquitectura y el informe técnico. Las correcciones
necesarias de diseño se integran en sus apartados, como prosa académica continua,
sin notas editoriales sobre versiones anteriores. El resumen aprobado, los
objetivos y el estado del arte se conservan salvo solicitud expresa de revisión.
Las conclusiones permanecen pendientes hasta completar el proyecto. Las funciones
futuras y las mediciones históricas se identifican como tales.

El corte técnico del 12 de septiembre de 2026 incluye autorización documental
por expediente, documentos y auditoría transaccionales, migración y restauración.
Las cifras de 519 pruebas aprobadas y cobertura global del 90.7 % proceden de
[`docs/verification-report.md`](../docs/verification-report.md).
El corte del 14 de septiembre incorpora consultas documentales y la interfaz
Qadra conectada a servicios reales, con 539 pruebas Rust y cobertura de 91.0 %.
La entrega de versiones inmutables amplía el historial y las operaciones sobre
una instantánea explícita, con 577 pruebas Rust aprobadas y cobertura de 91.7 %.
La clasificación incorpora valores organizativos e historia auditada independientes
del contenido, con carga multipart atómica y filtros sobre las revisiones actuales.
Su corte registra 644 pruebas Rust aprobadas y cobertura global de 92.4 %.
El directorio incorpora fichas por expediente, archivo y reactivación con
revisiones inmutables; permanece separado de cuentas y membresías. Su corte
registra 704 pruebas Rust aprobadas y cobertura global de 92.9 %. La identidad
jurídica, FIREL, duplicidad verificada y expediente penal activo siguen abiertos.
La gestión procesal completa y la usabilidad siguen pendientes. Las mediciones de cada entrega se registran por
separado, sin actualizar cifras históricas por cambios de funcionalidad.
La revisión del reporte se registra en
[`docs/academic-report-verification.md`](../docs/academic-report-verification.md).

Antes de integrar cambios, compilar desde las fuentes y revisar el PDF: tablas,
rutas y listados legibles, referencias resueltas, paginación y afirmaciones
coherentes con la evidencia. Conservar los cambios locales de otros autores y
no añadir el PDF generado al índice de Git. La presentación es un entregable
separado; actualizar el reporte no implica que aquella haya sido revisada.

## Estructura

```
main.tex                 Archivo maestro (orden del documento)
config/
  preamble.tex           Paquetes, fuentes, estilos, biblatex, glossaries
  glossary.tex           84 entradas de glosario (\newglossaryentry)
frontmatter/
  portada.tex            Portada institucional IPN/ESCOM
  resumen.tex            Resumen + palabras clave
chapters/
  01-introduccion.tex    Cap. 1
  02-marco-teorico.tex   Cap. 2
  03-analisis-diseno.tex Cap. 3 (incl. 3.7 Arquitectura, 3.11 Criptografía)
  04-implementacion.tex  Cap. 4 Implementación del prototipo
  05-pruebas.tex         Cap. 5 Pruebas y resultados
  06-conclusiones.tex    Cap. 6 Conclusiones
  anexos.tex             Anexo A
  anexo-b-cli.tex        Anexo B Manual de la CLI
  anexo-c-verificacion.tex Anexo C Verificación independiente con OpenSSL
  anexo-d-pki.tex        Anexo D Scripts de la CA interna
  anexo-e-matriz.tex     Anexo E Matriz de pruebas y evidencia
references.bib           92 referencias (biblatex, estilo IEEE)
figures/                 Imágenes (image*.png) y diagrama TikZ (despliegue.tex)
```

## Convenciones (mantenibilidad)

- **Numeración automática**: las secciones/tablas/figuras NO llevan número
  manual; LaTeX las numera. Para referirte a ellas usa `\cref{etiqueta}`.
- **Etiquetas**: `sec:*` (secciones), `tab:*` (tablas), `fig:*` (figuras),
  `cap:*` (capítulos). Define una al crear el elemento.
- **Citas**: `\cite{clave}`; las claves están en `references.bib`. El estilo
  IEEE numera por orden de aparición.
- **Tablas**: usa `booktabs` (`\toprule/\midrule/\bottomrule`), sin reglas
  verticales. Tablas anchas → `tabularx`/`longtable`.
- **Glosario**: agrega términos en `config/glossary.tex`. Se imprimen todos
  vía `\glsaddall` en `main.tex`.

## Pendientes señalados en la migración

- **Ref. [75] (Vernon, *Implementing Domain-Driven Design*)**: estaba en la
  lista de referencias del documento original pero **nunca se cita en el
  cuerpo**. Se conserva con `\nocite{vernon2013}` en `main.tex`. Temáticamente
  corresponde a §3.7.5 (Bounded Contexts): decidir si citarla ahí o eliminarla.
- El conteo de tablas en la lista (85) supera en 4 al de la fuente Word (81):
  algunos bloques se modelaron como tablas independientes. No hay pérdida de
  contenido; revisar si se desea consolidar alguna.

## Procedencia

Migrado desde `TT_unpacked.docx` (versión del 8-may, la más completa) con
pandoc como base y limpieza a LaTeX idiomático. Se corrigió el defecto de
jerarquía donde «3.7 Diseño de Arquitectura» figuraba como capítulo (ahora es
la sección 3.7 del capítulo 3) y se recuperó el diagrama de la metodología XP
(`image4.png`) que el documento original incrustaba como objeto agrupado.
