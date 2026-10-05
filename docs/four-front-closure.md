# Cierre de los cuatro frentes procesales

## Frontera y estado reconciliado

Esta entrega termina los cuatro frentes de la tabla siguiente. No equivale al
cierre de todos los objetivos académicos ni autoriza funciones nuevas a partir
de decisiones técnicas. El código existente se conserva. Un pendiente solo se
agrega si impide una aceptación de esta lista o corrige un defecto reproducido;
se identifica el comportamiento afectado antes de implementar.

Estado contrastado el 5 de octubre de 2026 con main remoto `e9afef8`, el código
de la rama cautelar y la evidencia conservada en el informe de verificación.
Los respaldos de la rama cautelar no son integraciones ni despliegues.

| Frente | Integrado | Implementado sin integrar | Faltante concreto |
| --- | --- | --- | --- |
| Reglas de cálculo | Motor de horas/días/meses, perfiles versionados, calendario, explicación, reevaluación, Agenda y alertas. | No se acredita un corpus jurídico adicional terminado. | Calificar y aceptar los supuestos finitos indicados abajo; la aritmética sintética no prueba la regla aplicable. |
| Resultado y plazo derivado | Creación explícita conjunta, origen exacto, regla seleccionada, historial, Agenda, alertas y recuperación sin duplicados. | Ningún incremento necesario para rehacer este mecanismo. | Aplicar los perfiles jurídicos aceptados al recorrido existente. La falta de esos perfiles pertenece al primer frente. |
| Audiencias de recursos | Alegatos de apelación y revocación escrita, permisos, origen, historia, Agenda, alertas y Qadra. | Ningún incremento necesario dentro de su contrato. | Preservar su aceptación; corregir resúmenes antiguos que todavía dicen no integrado. |
| Cautelares y catálogo | Las seis familias ordinarias y de recursos ya disponibles. | Dominio, servicios y PostgreSQL cautelares; HTTP compuesto y consulta de Agenda PostgreSQL/HTTP con parser verificados. | Alertas, detalle de Agenda y demás Qadra, aceptación integrada con restauración, manuscrito y gates antes de una sola PR. |

Los [resultados/plazos derivados](hearing-derived-deadlines.md) y las
[audiencias de recursos](resource-hearings.md) conservan sus contratos. No se
repiten sus campañas completas ni se confunden con cobertura jurídica universal.

## Lista definitiva del cálculo para esta entrega

1. Investigación complementaria: duración judicial efectivamente declarada,
   inicio, meses, extremos de mes/inhábiles y cierre fundados. Los máximos de
   dos/seis meses no rellenan una duración concedida desconocida.
2. Prórroga de investigación: concesión expresa, duración, ancla y límite
   acumulado; no confundir solicitud con concesión.
3. Revocación oral: límite referido al fin de la audiencia identificada;
   no convertirlo en una suma de días ni crear otra cita.
4. Revocación escrita: supuesto de dos días, con notificación y efectos
   identificados, inicio, calendario y recepción/cierre aplicables.
5. Apelación de auto/providencia del juez de control: supuesto de tres días.
6. Apelación de sentencia definitiva del juez de control: supuesto de cinco días.
7. Apelación relativa al desistimiento de la acción penal: supuesto separado
   de tres días.
8. Apelación de sentencia definitiva del tribunal de enjuiciamiento: supuesto
   de diez días. No trasladar su inicio a cualquier apelación.

Esta lista fija familias y supuestos a verificar, no certifica una fórmula
jurídica. Cada modalidad de notificación admitida debe quedar nombrada en el
perfil y sustentada; una modalidad no calificada queda fuera de su aplicación.
No incorporar otros recursos, incidentes o términos por analogía.

El compromiso de investigación se encuentra en
[análisis y diseño](../latex/chapters/03-analisis-diseno.tex): HU-20, CU-08 y los
criterios RF-07 y RF-08. El alcance de recursos está explícito en
[su matriz](procedural-resources-scope.md) y en el objetivo de gestión de
[la introducción](../latex/chapters/01-introduccion.tex).

**Fuera de esta entrega:** calcular retención de 48 horas, vinculación de
72/144 horas y acusación de 15 días. En los documentos disponibles, sus duraciones
específicas aparecen en el [marco teórico](../latex/chapters/02-marco-teorico.tex),
no en una aceptación funcional específica de esos cálculos. RF-09 sí exige
alertar sobre un plazo de vinculación ya registrado que vence en 48 horas:
se conserva esa aceptación del mecanismo de alertas, sin derivar de ella otro
perfil jurídico. No se localizó un protocolo aprobado separado en el árbol
versionado; no se atribuyen compromisos a un documento que no se pudo inspeccionar.

Para aceptar cada fila del corpus deben existir:

- Fuente primaria, ámbito, supuesto y versión; datos de entrada y resultado
  esperado establecidos independientemente del algoritmo.
- Caso calculable con inicio, efecto de notificación cuando corresponda,
  calendario/inhábiles, cierre y explicación reproducible.
- Casos límite aplicables: fin de mes/bisiesto para meses; inhábil/excepción para
  días; dato o precisión ausente y notificación/ámbito no cubiertos.
- Motivo visible y ningún vencimiento operativo inventado cuando falta un dato.
  Una regla jurídica todavía sin fundamento no se declara terminada por devolver
  siempre un bloqueo.
- Recorrido con fuente real en el flujo integrado, fecha calculada visible y
  alertas del término operativo, sin duplicación y con historia preservada.

Las contradicciones entre redacción académica y fundamento se reportan como
una decisión concreta; no se resuelven cambiando objetivos ni ampliando el motor.

## Catálogo definitivo de audiencias

Inicial; intermedia; juicio oral; individualización y reparación; medidas
cautelares; alegatos de apelación; revocación escrita. Son siete familias.
Continuación es una relación con la sesión anterior, no una octava familia.
La familia cautelar tiene los dos propósitos ya construidos: imposición y revisión.

## Aceptaciones finitas del flujo cautelar

| Recorrido observable | Criterio para marcarlo terminado |
| --- | --- |
| Programar y gestionar convocatoria | Desde Qadra, programar imposición/revisión, reprogramar y cancelar; consultar la revisión actual y la historia con sus participantes y soporte. |
| Declarar decisión y medidas | Registrar decisión expresa y sus medidas, conservar origen, sujeto, condiciones, inicio y vigencia declarados; consultar la última declaración registrada. Si se vincula a una audiencia inicial se reutiliza su cita. |
| Registrar cambios y rectificaciones | Exponer las acciones ya construidas: imponer, confirmar, modificar, revocar, cesar, sustituir y declarar sin cambios; corregir captura, marcar error y reemplazar identidad sin reescribir la decisión histórica. |
| Agenda y alertas | Una fila por convocatoria cautelar, apertura de su detalle autorizado y alertas con origen propio. No añadir filas por cada medida ni duplicar la audiencia inicial. |
| Acceso y recuperación | Owner y Litigante asignado gestionan; Paralegal asignado consulta; Client y expedientes ajenos quedan denegados. Cierre impide mutaciones. Respuesta perdida/reintento conserva el mismo registro; reingreso reautoriza y no reenvía automáticamente. |
| Reinicio y restauración | Servicios reales conservan identidades, historia, recibos, orígenes y auditoría; no aparecen duplicados ni escrituras parciales. Recorrido real en escritorio y móvil con evidencia revisable. |

Las catorce clasificaciones de medidas y acciones ya implementadas están
congeladas en [el contrato cautelar](precautionary-hearings-scope.md). Se conectan
a la interfaz; no se abre otro catálogo, formato histórico o subsistema salvo
defecto demostrado que impida estos recorridos.

Quedan fuera: recomendar medidas, decidir procedencia, supervisar cumplimiento,
extraer obligaciones del texto, concluir medidas por el reloj o cierre del
expediente, crear plazos cautelares automáticamente y sumar 48 horas por elegir
revisión. Tampoco se rediseña Qadra ni se habilitan decisiones pendientes de
Owner, correo, inactividad o activación en VPS3.

## Secuencia y condición de término

Primero terminar el HTTP cautelar compuesto; después conectar Agenda/alertas y
Qadra sobre los servicios existentes; luego aceptar el recorrido real con
restauración y conciliar API, operación y manuscrito. El corpus jurídico se
cierra sobre el mecanismo de plazos ya integrado, sin reescribirlo.

Una demostración parcial de HTTP es avance funcional, no cierre de la entrega.
La rama cautelar permanece unida hasta completar su recorrido. Se ejecuta la
regresión completa una vez al cierre, con todos los gates para el HEAD exacto,
squash y confirmación posterior de main. El PDF aceptado se preserva; cualquier
nuevo manuscrito se construye y revisa como resultado separado antes de sustituirlo.

El cierre se mide por las filas anteriores aceptadas y por main confirmado,
no por tamaño del código, commits, número de pruebas o consumo. Una aceptación
aprobada no se reabre por una mejora opcional. La falta de datos jurídicos se
presenta con el supuesto y decisión faltantes, sin una investigación indefinida.
