# Entrega: documentos por expediente y auditoría transaccional

Implementación y ensayos locales del 12 de septiembre de 2026. Este documento
conserva el alcance y los criterios de la entrega; la evidencia reproducida está
en [el informe de verificación](verification-report.md), las decisiones en
[ADR-0016](adr/0016-case-document-transactions.md) y la operación en
[la guía de base de datos](database-operations.md).

## Objetivo

Autenticar y autorizar cada operación documental contra la pertenencia vigente,
conservar evidencia criptográfica existente y confirmar cambios documentales y
de asignaciones junto con su auditoría bajo una frontera transaccional explícita.
Incluir migración verificable y recuperable desde los registros locales, pruebas
reales, contrato, demo y PR de una rama `feat/`, con CI aprobado y squash.

## Orden de trabajo

1. Inventariar registros y todos los escritores de auditoría. Definir en un ADR
   las asociaciones, el punto de autorización, compatibilidad y migración.
   Los documentos antiguos requieren un mapa explícito documento-expediente.
2. Escribir pruebas de permiso por rol y pertenencia para carga, sellado,
   verificación y exportación, incluidas rutas heredadas y revocación concurrente.
3. Implementar la transacción PostgreSQL y la continuidad de auditoría. Incluir
   mutaciones de expedientes y asignaciones; coordinar también los escritores de
   identidad para evitar cadenas o estados de confirmación contradictorios.
4. Separar preparación criptográfica y TSA del commit; definir reintentos sin
   regenerar evidencia histórica ni mantener una transacción durante llamadas
   externas. Establecer el orden entre revocación y confirmación de una mutación.
5. Ensayar migración sobre copia, reconciliar resultados y restaurar un respaldo.
   Ejecutar demo integrada y actualizar contrato, ADR e informe antes de la PR.

## Criterios de cierre

- Los cuatro roles y dos expedientes demuestran consultas y mutaciones
  autorizadas, rechazo cruzado y revocación usando la misma sesión. Cliente
  conserva denegadas las operaciones documentales hasta una ampliación explícita
  de la matriz de permisos posterior a verificar todas las rutas.
- Fallos antes de guardar, durante auditoría y al confirmar no dejan una mutación
  sin evento ni un evento de éxito sin mutación. La cadena conserva orden bajo
  concurrencia y el rol operativo no puede modificar ni eliminar eventos.
- UUID, versión, contexto autenticado y contenido cifrado se conservan. Firma,
  sello, certificados, CRL y material capturado siguen verificándose con OpenSSL;
  sustituciones entre documentos/expedientes y reintentos no alteran evidencia.
- La migración incluye inspección sin escrituras, mapa de asociaciones, rechazo
  de huérfanos/corruptos y ejecución reanudable o idempotente. Conteos y hashes
  cuadran; se conservan originales y cadena histórica y se demuestra restauración.
- Dos procesos no producen doble sellado ni pérdida de cambios. Una prueba fija
  el comportamiento de revocación concurrente respecto al commit y documenta el
  límite de respuestas que ya estaban en curso.
- TDD, PostgreSQL/Redis desechables, formato, build, suite, Clippy, cobertura y
  demo HTTP pasan; la PR describe las garantías y límites reales.

## Secuencia propuesta

El cronograma académico versionado en
[`latex/chapters/03-analisis-diseno.tex`](../latex/chapters/03-analisis-diseno.tex)
ordena diez semanas y reserva el cierre para pruebas y usabilidad. Su sección de
capacidad contempla 48 horas-persona semanales y una sola línea de trabajo de
24 horas en pareja. Las fechas siguientes son una propuesta operativa nueva;
no se presentan como fechas calendario ya aprobadas en ese documento.

| Ventana propuesta de 2026 | Resultado |
| --- | --- |
| 14-16 de septiembre | Inventario, ADR y pruebas que fijan autorización y migración. |
| 17-18 de septiembre | Primer corte de autorización por expediente y diseño transaccional revisado. |
| 21-23 de septiembre | Persistencia transaccional, continuidad de auditoría, fallos y reintentos. |
| 24-25 de septiembre | Migración sobre copia, restauración, concurrencia y demo. |
| 28-30 de septiembre | Reserva para defectos, verificación final y PR. |

El primer corte concentra autorización e inventario. Agrupar además transacción,
migración, restauración y CI en cinco días dejaría poco margen para descubrir
incompatibilidades de datos. Reestimar el trabajo posterior con evidencia de
este avance y conservar la ventana final de validación.

Después siguen consultas y versiones documentales, participantes procesales,
audiencias/plazos/alertas y la interfaz mínima. Esos bloques quedan fuera de
este objetivo, igual que un PSC externo y el despliegue público. Las asignaciones
de usuarios actuales no equivalen a participantes procesales.

El punto de partida y las limitaciones operativas están en
[backend-review.md](backend-review.md), [el contrato HTTP](http-api.md) y
[ADR-0014](adr/0014-case-membership-and-isolation.md).
