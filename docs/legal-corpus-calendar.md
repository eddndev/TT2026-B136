# Calendario y canales del corpus jurídico de CDMX

Ficha documental del 11 de octubre de 2026. Delimita entradas y resultados
esperados para el caso controlado de órgano penal local de CDMX, con cobertura
del **01-01-2026 al 30-11-2026**. Las fuentes y ejemplos por sí solos no acreditan
ejecución; la aceptación local se registra abajo. No se acredita integración o
revalidación diaria del calendario en producción. Su representación de datos
está en [legal_corpus_calendar.py](../scripts/legal_corpus_calendar.py).

## Fuentes y alcance

| Fuente primaria | Localizador y uso en la ficha |
| --- | --- |
| [CNPP consolidado, reforma 28-11-2025](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf#page=25) | Art. 94, pp. 24-25: cómputo ordinario, inhábiles y excepciones. |
| [Circular CJCDMX-21/2025](https://www.poderjudicialcdmx.gob.mx/wp-content/uploads/Circula_CJCDMX_21_2025.pdf) | Acuerdo 76-29/2025 de 20-08-2025; PDF pp. 1-4: días de 2026, guardias penales y vacaciones escalonadas. |
| [Circular CJCDMX-23/2025 y Acuerdo 80-29/2025](https://www.poderjudicialcdmx.gob.mx/wp-content/uploads/CIRCULAR_CJCDMX_23_2025_Acuerdo_80_29_2025.pdf#page=13) | Art. 4.I, p. 13: atención al público para promociones. Los apartados II-IV distinguen sistemas del MP y asuntos urgentes, fuera de este canal. |

Se adopta el supuesto ordinario del litigante, con notificación personal y
recepción por ventanilla UGJ. No comprende sistemas del Ministerio Público,
notificación electrónica, guardias urgentes ni excepciones constitucionales.
La ausencia de suspensión o acuerdo particular que altere el cómputo es una
condición expresa del caso: si existe uno no cubierto o su aplicación es
desconocida, debe bloquearse el vencimiento operativo.

## Días del calendario controlado

Se excluyen sábados y domingos y las fechas publicadas siguientes, dentro de
la cobertura seleccionada:

| Mes de 2026 | Fechas inhábiles adicionales |
| --- | --- |
| Enero | 1 |
| Febrero | 2 |
| Marzo | 16, 30 y 31 |
| Abril | 1, 2 y 3 |
| Mayo | 1, 4 y 8 |
| Junio | 19 |
| Septiembre | 14, 15 y 16 |
| Octubre | 30 |
| Noviembre | 2 y 16 |

La tabla general incluye vacaciones del 15 al 31 de julio, pero las páginas
3-4 disponen turnos y vacaciones escalonadas para el sistema penal y las UGJ,
con personal que permanece laborando. No se importa una suspensión civil,
familiar o laboral como exclusión penal automática de todo ese intervalo.
La cobertura termina en noviembre y no decide el régimen de diciembre/enero.

## Cierre del canal ordinario

El artículo 4.I distingue atención de 09:00 a 15:00 de lunes a jueves y de
09:00 a 14:00 los viernes, salvo inhábiles publicados. Para los cinco recursos
escritos se utiliza el cierre semanal siguiente, con desfase -06:00 del caso:

| Día hábil final | Cierre local de ventanilla | Instante UTC |
| --- | --- | --- |
| Lunes a jueves | 15:00 | 21:00 del mismo día |
| Viernes | 14:00 | 20:00 del mismo día |
| Sábado o domingo | Sin cierre disponible en este canal ordinario | Ninguno |

Se trata del cierre del canal seleccionado, no de una hora universal de
caducidad, preclusión o recepción por cualquier medio. El fin de audiencia de
la revocación oral conserva su instante propio. En investigación y prórroga,
el cierre a las 18:00 procede exclusivamente de la resolución ficticia expresa
del caso y no se atribuye al horario de esta ventanilla.

## Resultados de contraste

Con notificación del 14-10-2026 y efectos el 15, las fechas civiles de los cinco
recursos escritos son **16, 19, 21, 19 y 28 de octubre**, respectivamente.
Como extremo, notificación el 28-10 y efectos el 29: el día 1 es jueves 29;
se excluyen 30-10, 31-10, 01-11 y 02-11; el día 2 es **03-11-2026**, con
cierre ordinario a las 15:00, equivalente a 21:00 UTC.

Las [fichas de recursos](legal-corpus-resources.md) y la
[ficha de investigación](legal-corpus-investigation.md) conservan sus propios
supuestos. Los ejemplos mensuales aislados de 2027/2028 usan otro calendario,
explícitamente sintético; no acreditan inhábiles oficiales futuros. Las fuentes,
las condiciones y los esperados delimitados permiten preparar la aceptación
sin exigir un revisor externo de los ocho casos ni prolongar esta investigación.

## Contraste ejecutado

La aceptación local del 12 de octubre de 2026 UTC reprodujo estos supuestos
con HTTP, PostgreSQL y Redis reales, incluidos Agenda, alertas, reinicio y
restauración. Qadra mostró los mismos resultados y fuentes en escritorio y
móvil. El [registro de verificación](verification-report.md) separa positivos,
negativos, fallos preparatorios y comprobaciones finales pendientes. Esta
aceptación no constituye una resolución judicial ni una integración productiva.
