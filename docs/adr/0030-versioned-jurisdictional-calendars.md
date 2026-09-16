# ADR-0030: Calendarios jurisdiccionales con revisiones exactas

## Status

Accepted. Backend, HTTP and Qadra implemented and verified locally.

## Context

La agenda de audiencias organiza citas con instante y desfase. El calculo de
plazos requiere ademas clasificacion de fechas aplicable al supuesto, cobertura,
fuentes y cambios trazables. Una lista de descansos laborales o el texto libre
de autoridad del expediente no determina ese calendario.

El [CNPP, articulo 94](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf)
distingue dias, horas y excepciones por acto. El
[listado OAJ](https://www.oaj.gob.mx/transparencia/paginas/diasinhabiles.htm)
reune fundamentos y ambitos distintos. Son motivos para conservar alcance y
procedencia explicitos, no para importar un calendario unico por defecto.

## Decision

Incorporar un catalogo global administrado por Owner, consultable por el personal,
con fechas civiles, ambito inmutable desde la primera revision y clasificacion
semanal expresa. Cada revision conserva cobertura finita, excepciones sin
solapamiento y referencias publicas declaradas propias. No reutilizar documentos
privados de expedientes ni inferir aplicabilidad mediante nombres.

Publicacion, reemplazo y retiro producen revisiones inmutables y recibos propios.
Canon de valores JCAL1 y canon de envio JCTX1 permiten conciliar operaciones
exactas. Auditoria y estado comparten transaccion, con cuenta y rol revalidados
despues del bloqueo. El retiro conserva el pasado y es terminal.

Clasificar una fecha fuera de cobertura produce estado explicito. Una regla sin
resolucion produce unresolved; no hay lunes-viernes predeterminado ni dia habil
por falta de fuente. La revision exacta determina la consulta incluso despues
de un reemplazo o retiro. No se convierte fecha en hora o zona del navegador.

El contrato completo esta en [judicial-calendars-api.md](../judicial-calendars-api.md).
Los codigos de entidad se fijan en 01..32 del catalogo INEGI citado alli; organo y
territorio permanecen descripciones. El catalogo geografico no acredita competencia.

## Consequences

- Este catalogo aporta configuracion y consulta; no completa calculo, reevaluacion,
  notificaciones ni el calendario unificado exigidos por el alcance academico.
- Las referencias son metadatos, sin copia archivada ni verificacion del contenido
  remoto. Preservar evidencia normativa necesitara almacenamiento y permisos propios.
- La evaluacion futura fijara regla, hechos, calendario exacto y semantica temporal.
  No se activan plazos a partir de texto narrativo de audiencias.
- Al integrar plazos, la publicacion creara intenciones durables de reevaluacion
  en su misma transaccion; las confirmaciones de plazos revalidaran la cabeza para
  evitar carreras. No se anticipan colas vacias ni recuentos de trabajo ficticios.
- Coberturas incompletas y supuestos no representables permanecen visibles y
  necesitan datos o una ampliacion del contrato antes de producir vencimientos.
