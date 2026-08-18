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
references.bib           88 referencias (biblatex, estilo IEEE)
figures/                 Imágenes (image*.png)
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
