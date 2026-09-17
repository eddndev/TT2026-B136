# Aritmetica temporal para reglas de plazos

Estado: componente de aritmetica pura implementado. La verificacion de su implementacion se
registra en [el informe de verificacion](verification-report.md). Este componente no publica perfiles del
CNPP ni crea plazos operativos. El alcance pendiente conserva activacion desde
actos, calificacion y fuentes exactas, persistencia, reevaluacion, alertas y Qadra.

La [extracción de tiempos](deadline-triggers.md) ahora coordina esta aritmética
con un campo o una calificación sobre material exacto. El dominio comprueba
coherencia de las referencias y conserva la procedencia; el servicio todavía
debe verificar autorización, recibos, fuentes y aplicabilidad antes de persistir
una evaluación operativa.

## Entradas explicitas

`evaluate_deadline_arithmetic` recibe una `ArithmeticRule`, el tiempo declarado
original y, cuando la regla lo requiere, los valores de un calendario exacto.
No recibe una fecha de vencimiento introducida por el operador. Deriva una
candidata con la unidad, inclusion y politica indicadas por la regla.

- `Days`: cantidad positiva, inclusion de la fecha ancla o inicio al dia
  siguiente, base natural o dias computables y politica del ultimo dia.
- `CivilMonths`: cantidad positiva de meses civiles y politica del ultimo dia.
  Desplaza directamente el mes y conserva el numero de dia, si existe.
- `ElapsedHours`: cantidad positiva de horas transcurridas desde un instante
  declarado con segundo y desfase. No aplica ajustes de calendario.

No existe `Default` para seleccionar silenciosamente unidad, inclusion o
politica. Este valor aritmetico no acredita aplicabilidad, procedencia juridica
ni seleccion del campo desencadenante. Un perfil verificado debe derivar la
regla y elegir expresamente el campo correspondiente del hecho exacto. Practica,
recepcion, efecto declarado, emision, programacion y final de audiencia no son
intercambiables. Texto de acuerdos o resumenes no selecciona reglas.

## Semantica

### Dias

El inicio es la fecha del ancla o su siguiente fecha civil, segun la regla.
Para dias naturales la candidata es inicio mas cantidad menos uno. No se
consulta el calendario para excluir fechas intermedias. Para dias computables
se reutiliza [el conteo civil](deadline-day-counting.md), sin alterar sus
clasificaciones, fuentes, presupuesto ni candidatos.

### Meses

La operacion calcula el ano y mes destino de una sola vez. Mantiene el dia
original; no suma meses de forma iterativa ni los convierte en treinta dias.
Si no existe ese numero de dia en el mes destino, devuelve
`MissingHomologousDay` con ano, mes y dia solicitado. No ajusta a fin de mes ni
avanza al siguiente: esas politicas requieren un contrato y respaldo propios.

La aritmetica no decide que el primer dia se incluya, que el plazo sea el maximo
legal o que una fecha judicial de cierre se pueda reemplazar por este resultado.
La calificacion debe conservar cantidad realmente ordenada, inicio y fuente;
la seleccion y efecto juridicos siguen pendientes del perfil correspondiente.

### Horas

Exige precision `Second` y desfase explicito. Una fecha o minuto no se completa
con ceros. Un tiempo desconocido y un segundo sin desfase generan bloqueos
diferentes. Con los datos suficientes calcula el instante inicial mas cantidad
por 3600 segundos y expresa la candidata en UTC.

La salida calculada no es una nueva declaracion del hecho. El resultado conserva
integro el ancla, incluido su desfase original. No convierte el desfase fijo en
una zona territorial ni predice la hora local del vencimiento. El instante UTC
queda limitado a los anos 1 a 9999. La cuenta es de tiempo transcurrido y no
mueve horas hasta dias habiles, aunque se suministre un calendario.

### Ultimo dia

`Preserve` devuelve la candidata civil sin consultar calendario. `NextCountable`
clasifica desde esa candidata mediante el conteo de un dia computable. Avanza
sobre fechas excluidas, conserva sus fuentes y se detiene ante una fecha sin
resolver o fuera de cobertura. Nunca salta incertidumbre. El calendario de
ajuste es exactamente el proporcionado; no se selecciona otro por conveniencia.

El ajuste conserva la candidata aritmetica original en la traza. Si bloquea, esa
fecha es un antecedente y no el resultado ajustado. No se impone esta politica a
un instante horario, pues cambiar su fecha exigiria decidir tambien el corte.

## Resultado y traza

`DeadlineArithmetic` conserva regla, ancla, resultado y pasos. Sus resultados
son una candidata civil, una candidata de instante UTC o un bloqueo tipado.
Ninguno declara por si solo un plazo activo, vencido, firme o inadmisible.

Los pasos conservan operandos, candidata intermedia cuando existe, recorrido de
conteo o ajuste y ano/mes/dia destino aun si falta el homologo. Una entrada
insuficiente no recibe operaciones ficticias. Los bloqueos distinguen ancla
desconocida, precision insuficiente, desfase o calendario ausentes, falta de
homologo, calendario sin resolver, falta de cobertura y agotamiento del rango.
No se consulta el reloj actual; repetir los mismos valores reproduce el resultado.

Naturales, meses y horas hacen trabajo constante, incluso para `u32::MAX`. Cada
recorrido de calendario conserva el limite de 1097 pasos. La regla solo puede
invocar un conteo y un ajuste: a lo sumo dos recorridos, sin reservar memoria
segun la cantidad. El dominio no verifica hashes ni lee snapshots de aplicacion;
la extracción temporal conserva la referencia y las huellas recibidas de la
fuente. La integración futura debe verificar esos insumos y vincular también
identidad, revisión y huellas del calendario aplicable.

## Corpus matematico, separado de criterios juridicos

| Operacion sintetica | Resultado esperado |
| --- | --- |
| 2026-01-30, dos naturales, ancla incluida | 2026-01-31 |
| 2026-01-30, dos naturales, desde el dia siguiente | 2026-02-01 |
| 2026-01-31 mas un mes | Homologo inexistente en febrero |
| 2026-01-31 mas dos meses | 2026-03-31 |
| 2028-01-29 mas un mes | 2028-02-29 |
| 2028-02-29 mas doce meses | Homologo inexistente en febrero de 2029 |
| 2026-12-31 23:30:00 -06:00 mas dos horas | 2027-01-01 07:30:00 UTC |

Los fixtures de calendario son sinteticos y no acreditan dias oficiales. Las
pruebas comparan tambien excepciones, fuentes, bloqueos, precision, cantidades
maximas y limites del ano. No derivan el resultado esperado llamando a la misma
funcion que se prueba. Fecha civil con desfase sigue siendo fecha, sin corte
horario para alertas de 48 o 24 horas.

## Integracion que falta

Hace falta una declaracion de calificacion estructurada y versionada ligada a
resolucion, notificacion o resultado/acuerdo exactos. Debe separar maxima legal
de duracion ordenada, destinatario de receptor, tiempo generico de sesion de
terminacion y estado administrativo de efecto judicial. Un perfil normativo
versionado necesita corpus de disparador, inclusion, unidad, excepciones y corte.

Despues, el servicio debera verificar acceso, fuentes y calendario; persistir
la evaluacion con su historia y auditoria; coordinar reevaluaciones durables y
alertas; exponer los flujos en HTTP y Qadra. Correccion o retiro de una fuente no
sustituye una evaluacion historica ni equivale a cancelacion judicial. La
[matriz de producto](product-completion.md) conserva estos pendientes.

La investigacion y los limites actuales de aplicabilidad se conservan en
[fuentes y fronteras de los perfiles](deadline-rule-research.md).
