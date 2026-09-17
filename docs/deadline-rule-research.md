# Fuentes y fronteras de los perfiles de cómputo

Estado: investigación normativa acotada para diseñar perfiles verificables.
Consulta de fuentes oficiales: 16 de septiembre de 2026. No constituye un
contrato completo del evaluador ni declara plazos operativos implementados.

El [alcance de recursos](procedural-resources-scope.md) conserva el cómputo
automático, las audiencias, los recursos, la reevaluación y las alertas. Esta
investigación no autoriza sustituir el criterio aprobado de cómputo mensual por
fechas manuales ni reducir el alcance por falta de un oráculo. El término
"oráculo" designa aquí un conjunto de casos cuyos resultados esperados tienen
fundamento identificado, distinto de probar que una operación matemática funciona.

## Fuentes consultadas

| Fuente primaria | Lectura y alcance comprobados |
| --- | --- |
| [Registro de reformas del CNPP](https://www.diputados.gob.mx/LeyesBiblio/ref/cnpp.htm) | La última reforma publicada que muestra el registro es DOF 28-11-2025. |
| [CNPP consolidado](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf) | Edición de 168 páginas con esa reforma. Se leyeron los artículos 82, 94, 211, 321, 322, 465 y 466 para este análisis. |
| [Amparo en revisión 630/2022, engrose](https://www2.scjn.gob.mx/juridica/engroses/1/2022/2/2_305479_6649_firmado.pdf) | Primera Sala, 20-09-2023. Antecedente del párrafo 3 y razonamiento de los párrafos 37 a 51. El título de extracción automática no coincide con la portada: el cuerpo identifica AR 630/2022. |
| [Amparo directo en revisión 6372/2024, engrose](https://www2.scjn.gob.mx/juridica/engroses/1/2024/10/2_339591_7324_firmado.pdf) | Primera Sala, 13-08-2025. Lectura de los párrafos 71 a 83 sobre suspensión del juicio. La portada y los encabezados identifican 6372/2024; el primer párrafo contiene una referencia discordante a 6374/2024. |

### Base legal resumida

Los artículos 321 y 322 distinguen duración judicial, máximos de dos/seis
meses y prórroga limitada. El 211 sitúa el comienzo de la fase complementaria
en la imputación. Para revocación escrita fuera de audiencia, el 466 prevé dos
días siguientes a notificación. El 82 atribuye efectos al día siguiente a la
personal; el 94 regula inicio diario, inhábiles, excepciones y ajuste final.
Fuente: [CNPP](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).

## Investigación complementaria: oráculo mensual sin cerrar

El AR 630/2022 aplica la CT 230/2021 y reproduce la jurisprudencia
1a./J. 146/2022 sobre la imposibilidad de prorrogar más allá del máximo. Ese
razonamiento no define una fórmula para todos los extremos mensuales.

Su párrafo 3 relata una vinculación el 13-04-2021 y un plazo judicial de tres
meses cuya conclusión se fijó el 12-07-2021. Es un antecedente del caso, no un
criterio general que permita restar un día o demostrar inclusión inicial.
[Fuente y localización](https://www2.scjn.gob.mx/juridica/engroses/1/2022/2/2_305479_6649_firmado.pdf).

No se localizó una decisión que resuelva conjuntamente los siguientes extremos.
La ausencia de hallazgo en esta búsqueda no demuestra inexistencia de criterios.

| Extremo pendiente | Insumo o regla que debe cerrarse antes de activar el perfil |
| --- | --- |
| Inicio e inclusión | Acto y fecha que gobiernan el plazo concreto, determinación judicial exacta y regla de inclusión; no equiparar automáticamente inicio de fase e inicio aritmético. |
| Día homólogo inexistente | Fundamento aplicable para elegir último día del mes, traslado u otra solución; no seleccionar una política por conveniencia de programación. |
| Año bisiesto | Casos normativos esperados para febrero, además de la corrección gregoriana del algoritmo. |
| Inhábiles | Tratamiento aplicable al transcurso mensual y al eventual desplazamiento final, con ámbito y calendario identificados. |
| Prórroga | Unidad y duración efectivamente concedidas, ancla, máximo acumulado y fuente exacta; no asumir equivalencia entre suma única y sumas sucesivas. |

La fecha expresamente fijada por un órgano puede conservarse como hecho declarado.
Eso no prueba que el sistema haya calculado automáticamente el término ni valida
retrospectivamente una fórmula elegida para reproducir esa fecha.

## Revocación escrita: candidata bajo una inferencia estrecha

Inferencia propuesta de lectura conjunta: resolución fuera de audiencia,
notificación personal directa al destinatario en instalaciones judiciales y
supuesto de revocación identificado. Práctica en lunes y martes/miércoles
computables: martes sería primer día y miércoles candidata civil, sin sumar
otro día después de efectos. No es un criterio jurisprudencial localizado ni
un corte horario acreditado. [Base examinada](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).

No se encontró un precedente específico que cierre, para ese supuesto, todas
las cuestiones de inicio y recepción final. La propuesta requiere revisión
explícita de aplicabilidad; no se presenta como interpretación universal.
No se trasladaron criterios de apelación, amparo, materia civil o fiscal.

| Insumo pendiente de validación | Consecuencia técnica propuesta si falta |
| --- | --- |
| Resolución exacta y calificación del supuesto | Bloquear la selección automática de perfil; `Order`, `Judgment` y el resumen libre no completan esa calificación. |
| Destinatario, receptor, representación, subtipo y constancia | No inferir eficacia desde asistencia, perfil profesional o un archivo cargado. |
| Práctica y efectos identificados, sin controversia determinante | Conservar desconocimiento o discrepancia; no elegir silenciosamente una fecha. |
| Calendario exacto, cobertura y ámbito aplicable | No inventar feriados ni aplicar un calendario federal a cualquier expediente. |
| Canal de presentación, corte y zona con fundamento | Permitir, si procede, candidata civil; no fabricar `due_at`, medianoche ni alertas operativas. |
| Decisiones particulares que modifiquen el tratamiento | Conservarlas como dependencias explícitas, sin sustituir automáticamente el cálculo histórico. |

## Hallazgo sobre suspensión del juicio

El ADR 6372/2024, párrafos 71 a 83, identifica la tensión entre el texto del
artículo 351 y el 94 e interpreta el plazo de suspensión del juicio como diez
días hábiles. Es razonamiento de la sentencia, no únicamente un alegato de parte.
[Engrose oficial](https://www2.scjn.gob.mx/juridica/engroses/1/2024/10/2_339591_7324_firmado.pdf).

Este hallazgo impide tratar la extracción literal de una norma como perfil
operativo suficiente. Antes de incorporar esa interpretación deben verificarse
su fuerza, ámbito, aplicación temporal y relación con otros criterios. La
presente revisión no certifica su aplicabilidad a todos los expedientes ni lo
extiende a meses de investigación o a revocación.

## Fuentes no accesibles y límites de evidencia

- La apertura directa de la [tesis 2025607](https://sjfsemanal.scjn.gob.mx/detalle/tesis/2025607)
  falló. La constatación del criterio utilizada arriba procede de su reproducción
  en el engrose AR 630/2022, que sí fue leído; no de una ficha íntegra recuperada.
- La [Mesa Cinco del CJF](https://www.cjf.gob.mx/reformas/sigesca/data/documentos/congreso/jun2016/Mesa5.pdf)
  apareció indexada, pero no pudo abrirse íntegramente. Sus extractos no se usan
  para establecer la fórmula mensual. Un material de conversatorio tampoco se
  equipara por su alojamiento oficial a una decisión jurisdiccional.
- Las búsquedas no aportaron un oráculo de fin de mes/bisiesto ni un fundamento
  específico de corte horario para el supuesto diario propuesto. No se rellenan
  esos vacíos con reglas de otras materias.

## Frontera del algoritmo

El [conteo civil](deadline-day-counting.md) y la
[precisión temporal declarada](procedural-time.md) permiten construir operaciones
matemáticas reproducibles. Las políticas de inclusión, homólogo inexistente,
suma mensual y ajuste final pueden probarse con casos sintéticos y trazas.
Su disponibilidad técnica no establece cuál política corresponde jurídicamente.

Mantener separados el corpus matemático y los perfiles normativos. Cada perfil
necesita versión, fuentes, supuesto, datos requeridos, resultados esperados y
límites de aplicabilidad. Una fecha candidata no equivale a vencimiento operativo;
un perfil sin fundamento suficiente debe informar el bloqueo, sin ocultarlo con
una fecha manual ni declarar satisfecho el alcance aprobado.
