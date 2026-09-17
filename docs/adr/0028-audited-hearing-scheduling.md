# 0028. Programación de audiencias con historia y recibos de operación

## Status

Accepted.

## Context

Las etapas registradas y el directorio histórico permiten vincular audiencias a
un expediente y a revisiones concretas de sus participantes. Una cita futura no
prueba celebración, asistencia, resultado ni inicio de un término. Una escritura
puede confirmarse aunque el navegador pierda su respuesta.

El catálogo de programación distingue inicial, intermedia, debate e
individualización. La correspondencia ordinaria con etapas y el antecedente de
condena para individualización se basan en los artículos 211, 307, 334, 349, 401,
408 y 409 del [CNPP de la Cámara de Diputados](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf),
consultado el 16 de septiembre de 2026, última reforma indicada 28 de noviembre
de 2025. Su traducción a restricciones del prototipo es una decisión técnica;
no determina la procedencia judicial de una convocatoria concreta.

## Decision

Programación, reemplazo y cancelación organizativa producen revisiones
inmutables. Tipo fijo por raíz; citas adicionales usan raíces nuevas. Se conserva
fecha y hora comunicada con desfase explícito, sin inferir resultados del reloj.
La selección de participantes refiere fichas y revisiones exactas, separadas de
las cuentas autorizadas de Qadra. Reemplazar puede conservar referencias ya
vinculadas, incluso después de editar o archivar la ficha.

Alta y reemplazo comprueban expediente activo, perfil completo, etapa compatible
y revisiones de contexto esperadas. Individualización exige declaración y
soporte exacto validado mediante la admisión documental existente. Cancelación
conserva los valores y contexto anteriores; no vuelve a exigir la etapa ni la
actividad actual de participantes. El cierre administrativo impide mutaciones.

Las consultas, mutaciones y eventos siguen la autorización por expediente y el
bloqueo transaccional común. Se repiten las condiciones mutables después del
bloqueo. Las evidencias documentales se validan fuera de él y se comparan otra
vez al confirmar. Un fallo de auditoría revierte la escritura completa. El
instante de captura de la revisión procede del reloj inyectado tras el bloqueo.

La preparación pública es transitoria y no reserva identificadores. Devuelve el
comando normalizado y digest de operación antes del envío. Cada revisión conserva
un recibo propio, ligado a UUID de operación único, actor, expediente, audiencia,
acción, revisión/contexto esperados, valores y motivo. La conciliación comprueba
ese recibo exacto; igualdad de valores o ausencia provisional no justifican
reintentar. No se duplica el canon en JavaScript ni se persisten tickets temporales.

La agenda selecciona cabeceras actuales y aplica permisos antes de filtros y
paginación. No expone notas, conexiones ni participantes en listados. Intervalos
y páginas acotados permiten consultas transversales sin recorrer expedientes en
el navegador. El contrato preciso está en [hearings-api.md](../hearings-api.md).

## Consequences

- Programar y conservar historial no completa resultados, asistentes reales,
  acuerdos, activación de plazos ni un calendario judicial.
- La integridad de una revisión requiere validar también sus referencias
  históricas y su origen de contexto, incluido el registro inicial de etapa que
  no posee un digest separado de valores procesales.
- El flujo de preparación requiere una solicitud adicional; permite conservar un
  borrador y distinguir conflictos de una respuesta perdida sin repetir escrituras.
- La agenda es una colección mutable. Cambios entre páginas requieren refresco;
  no se promete una fotografía global ni reservas temporales de resultados.
