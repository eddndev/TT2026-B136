# Cierre de los cuatro frentes procesales

## Frontera y estado reconciliado

La tabla siguiente delimita los cuatro frentes, ya cerrados dentro del alcance
acordado, incluida la aceptación final del corpus jurídico de ocho supuestos. La continuación
del resto de pendientes técnicos autorizados se delimita por separado en
[el inventario de cierre técnico](technical-closure.md); no autoriza funciones
nuevas a partir de decisiones técnicas. El código existente se conserva. Un pendiente solo se
agrega si impide una aceptación de esta lista o corrige un defecto reproducido;
se identifica el comportamiento afectado antes de implementar.

La base confirmada en producción es `671d356c`, versión `v0.1.6`, con el corpus
integrado por PR100 y las correcciones de preparación de pruebas de PR101/102.
Conserva el flujo cautelar de PR90 y las entregas posteriores; auditoría y
respaldo Redis están resueltos. Las fichas, `AtAnchor` y el cierre civil semanal
completaron aceptación, integración, paquete y activación. La evidencia funcional se
conserva en el [informe de verificación](verification-report.md); los gates se
contrastan con el HEAD exacto. Los respaldos no son integraciones ni despliegues.

| Frente | Integrado | Implementado sin integrar | Faltante concreto |
| --- | --- | --- | --- |
| Reglas de cálculo | Motor de horas/días/meses, perfiles versionados, calendario, explicación, reevaluación, Agenda y alertas; `AtAnchor` y cierre civil semanal. Ocho supuestos aceptados con HTTP/Qadra y restauración, integrados y activos en `v0.1.6`. | Ninguno dentro de las ocho fichas. | Ninguno: fuentes, ámbito, condiciones, esperados, manuscrito y gates comprobados. Las condiciones ajenas bloquean el vencimiento. |
| Resultado y plazo derivado | Creación explícita conjunta, origen exacto, regla seleccionada, historial, Agenda, alertas y recuperación sin duplicados. | Ningún incremento necesario para rehacer este mecanismo. | Ninguno: los perfiles del corpus ya se aceptaron sobre este recorrido. |
| Audiencias de recursos | Alegatos de apelación y revocación escrita, permisos, origen, historia, Agenda, alertas y Qadra. | Ningún incremento necesario dentro de su contrato. | Ninguno dentro de su contrato; conservar la aceptación existente. |
| Cautelares y catálogo | Flujo completo de PR90: dominio, servicios, PostgreSQL, HTTP, Agenda, alertas y Qadra; convocatorias, decisiones y rectificaciones con recuperación, navegador real, reinicios/restauración y manuscrito separado aceptados. Conservado en la base productiva actual. | Ningún incremento funcional pendiente dentro del catálogo cerrado. | Ninguno en este frente. No incluye la aceptación del corpus jurídico del primer frente. |

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

Las fichas de [investigación y prórroga](legal-corpus-investigation.md),
[seis recursos](legal-corpus-resources.md) y
[calendario y canales](legal-corpus-calendar.md) fijan fuentes primarias,
versiones, localizadores, condiciones y resultados independientes para las ocho
filas. El caso controlado es penal local de CDMX; ya están delimitadas las
notificaciones personales y la ventanilla ordinaria UGJ. No se exige un
expediente publicado idéntico ni revisión externa obligatoria de los ocho casos.

Los meses dependen de una orden ficticia expresa de fecha a fecha y cierre a
las 18:00 -06:00, separado del horario de ventanilla. La prórroga exige concesión
y acumulado expresamente calificados, sin inferencia de historia. La revocación
oral toma el fin declarado de la audiencia del 09-10-2026 mediante `AtAnchor`.
Las cinco fichas por días incluyen los efectos declarados del 15-10-2026 y el
cierre semanal documentado. El extremo escrito del 29-10 cruza los inhábiles
publicados del 30-10 y 02-11. Estas condiciones cierran la selección documental;
su aceptación funcional, integración y activación están registradas por separado
en el [informe de verificación](verification-report.md).

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
- Recorrido con la fuente normativa real y registros de prueba ficticios
  identificados en el flujo existente, fecha calculada visible y alertas del
  término operativo, sin duplicación y con historia preservada.

Las contradicciones entre redacción académica y fundamento se reportan como
una decisión concreta; no se resuelven cambiando objetivos ni ampliando el motor.
Las fuentes y los esperados anteriores ya satisfacen la parte documental de
estos criterios. La aceptación real comprobó ocho positivos, un límite inhábil
y diez bloqueos, junto con HTTP, Qadra, reinicio y restauración; su aprobación
no procede solamente de contar con fixtures. Una condición desconocida, rechazada
o ajena a las fichas bloquea el vencimiento operativo y no exige ampliar el plan.

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
revisión. Tampoco se rediseña Qadra ni se infieren las decisiones humanas
pendientes de cuenta/correo Owner, inactividad o política documental Client.

## Secuencia y condición de término

HTTP, Agenda/alertas y Qadra cautelares completaron sus recorridos; la aceptación
real incluye restauración y conciliación de API, operación y manuscrito. PR90
entrega el flujo unido, conservado en la base productiva actual. PR100 cerró el
corpus sobre el mecanismo de plazos existente; `v0.1.6` completó la activación
con `AtAnchor` y cierre semanal. No queda otro incremento procesal de esta lista.
La conciliación documental y aceptación productiva restante están en
[su registro propio](production-acceptance.md).

La integración exige todos los gates para el HEAD exacto, squash y confirmación
posterior de main. No se repiten aceptaciones válidas por una actualización del
inventario. El PDF aceptado se preserva; un nuevo manuscrito se construye y revisa
como resultado separado antes de sustituirlo.

El cierre se mide por las filas anteriores aceptadas y por main confirmado,
no por tamaño del código, commits, número de pruebas o consumo. Una aceptación
aprobada no se reabre por una mejora opcional. Los datos jurídicos de los ocho
supuestos están delimitados; las condiciones fuera de ellos se presentan como
un bloqueo concreto, sin una investigación indefinida. La evaluación con
participantes reales mantiene su propia necesidad de personas y resultados
reales; las pruebas automáticas del corpus no la sustituyen.
