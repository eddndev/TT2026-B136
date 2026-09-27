# 0056. Instantánea autorizada del tablero operativo

## Estado

Aceptado para implementación.

## Contexto

La pantalla de Inicio ofrece navegación, pero sus tarjetas no consultan
indicadores operativos. Sumar páginas de consultas independientes permitiría
mezclar asignaciones, estados y revisiones diferentes; también podría presentar
como vigentes fechas de plazos que requieren revisión.

## Decisión

Añadir un puerto de lectura de tablero y una implementación PostgreSQL que
calcula los indicadores dentro de la transacción auditada existente. Owner
consulta el despacho; Litigator únicamente expedientes asignados. La identidad
se revalida tanto en persistencia como al completar el servicio. Paralegal y
Client conservan su navegación sin acceso a este agregado.

Reutilizar la reconstrucción verificada de expedientes y la proyección de
vigencia de plazos. Contar cada documento una sola vez por su última versión y
clasificación. Las etiquetas de tipo `contrato` y `contract`, sin distinguir
mayúsculas ASCII, identifican contratos declarados; no inferir por extensión ni
nombre. Limitar y comprobar las fuentes antes de devolver resultados completos.

Presentar carga de trabajo por asignaciones activas de litigantes activos, sin
exponer miembros ni expedientes fuera del ámbito autorizado. Conservar un
instante común y explicitar que siete días incluye la ventana de 48 horas.
Consultar mediante la acción de apertura o actualización, sin sondeo periódico.
El contrato y las fronteras temporales se detallan en `docs/dashboard-api.md`.

## Consecuencias

El tablero añade una consulta consistente y auditada, sin crear otra copia de
los datos del dominio ni otro proceso en segundo plano. La reconstrucción de
plazos tiene un costo acotado; un límite o error produce un fallo visible en vez
de totales incompletos. Este tablero no implementa generación de informes,
consulta de actividad, activación automática ni clasificación jurídica de
plazos fatales. La firma individual continúa siendo un flujo distinto del sello
interno contabilizado.
