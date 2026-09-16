# Precision temporal de hechos procesales declarados

Estado: valor temporal del dominio implementado y comprobado.
No crea resoluciones, notificaciones, recursos, plazos, API o persistencia.
El conteo de fechas conserva su contrato separado en
[deadline-day-counting.md](deadline-day-counting.md).

## Motivo

Una constancia puede indicar solo una fecha, una fecha y hora local sin desfase,
o una fecha y hora con desfase. Tambien puede omitir el tiempo del acto. Guardar
un UTC ficticio o completar segundos desconocidos alteraria lo declarado.
El reloj de captura del servidor identifica el registro, no el tiempo del acto.

El valor conserva tres dimensiones: fecha/hora conocida, precision y desfase
opcional declarado. No consulta reloj, zona IANA, navegador, red o calendario.
No selecciona efectos juridicos ni verifica la autenticidad de la declaracion.

## Valores representables

| Precision | Campos conocidos | Desfase | Instante en segundos |
| --- | --- | --- | --- |
| Unknown | Ninguno | Ausente | Ausente |
| Date | Fecha civil | Opcional | Ausente |
| Minute | Fecha, hora y minuto locales | Opcional | Ausente |
| Second | Fecha, hora, minuto y segundo locales | Opcional | Solo si el desfase esta declarado |

Una fecha usa `CivilDate`, con rango gregoriano 0001-01-01 a 9999-12-31.
Los constructores de minuto y segundo reciben componentes enteros y validan
hora 0..23, minuto 0..59 y segundo 0..59. No hay entrada de fracciones de segundo,
segundo intercalar ni normalizacion de 24:00 al dia siguiente. Un constructor
por segundo representa precisamente esa granularidad del modelo; introducir
cero expresamente es distinto de omitir el segundo usando precision Minute.

El desfase opcional usa minutos enteros entre -14:00 y +14:00 inclusivos.
`None` y UTC expreso son valores distintos. No se deduce desfase de una autoridad,
una entidad federativa, un calendario o una fecha. Este dato no expresa reglas
de zona IANA ni cambios historicos de horario.

Cuando hay desfase declarado, el dia completo o minuto completo representados
deben caber en anios UTC 1..9999; para precision Second se comprueba el segundo
indicado. Estas comprobaciones internas de extremos no asignan una hora conocida.
Sin desfase solo se valida el rango civil local. Unknown no exige fecha o zona
ficticias. No se compara el tiempo con el presente: un consumidor posterior
puede necesitar declarar tiempos de emision, recepcion o efectos con reglas
propias, sin heredar indiscriminadamente la politica de otro recurso.

## API y preservacion

El valor y su representacion interna son inmutables. Ofrece constructores de
Unknown, fecha, minuto y segundo; getters de precision, fecha local, hora,
minuto, segundo y desfase devuelven ausencia cuando el dato no fue declarado.
La consulta del segundo devuelve `None` para precision Minute, incluso si su
representacion interna utiliza cero al comprobar el intervalo.

`instant_value` devuelve un `OffsetDateTime` solo para Second con desfase
explicito; conserva ese desfase original. Fecha, Minute y Second sin desfase
no se convierten en un instante. No hay getters publicos de medianoche, limites
UTC, timestamp por defecto u orden temporal total. Unknown tampoco implementa
un valor predeterminado para completar una entrada ausente sin decision.

La igualdad compara los datos declarados y su precision: dos segundos que caen
en el mismo instante UTC con distintos desfases no son la misma declaracion.
Date no equivale a Minute a las 00:00; Minute no equivale a Second con segundo
cero; desfase ausente no equivale a UTC. No existe conversion implicita desde
los tiempos de etapas o resultados de audiencias. Sus formatos e historias
permanecen regidos por sus contratos existentes.

## Corpus de comprobacion

- Unknown conserva todos los campos temporales ausentes.
- Fecha con y sin desfase; febrero bisiesto y ambos extremos civiles.
- Hora local conocida sin desfase, con minuto y con segundo, sin instante.
- Minuto con desfase conserva minuto y no inventa segundo o instante.
- Segundo con desfase conserva fecha/hora/desfase originales y su instante.
- UTC declarado se distingue de ausencia; igualdad conserva precision y desfase.
- Horas, minutos y segundos fuera de rango se rechazan sin normalizarlos.
- Desfases de segundos, mayores de 14 horas y ambos extremos permitidos.
- Intervalos que cruzan anios UTC 0/10000 se rechazan al declarar desfase.
- Valores proximos a esos limites que si caben, incluidos segundos exactos.
- Los contratos y canones anteriores de etapas/resultados permanecen iguales.

Estos casos verifican fidelidad de datos temporales. No son un corpus juridico,
una formula mensual o la habilitacion de alertas. La futura resolucion o
notificacion debe conservar soporte, finalidad del tiempo y revision exactos;
el tipo temporal por si solo no acredita tales relaciones.
