# Conteo civil de dias sobre un calendario exacto

Estado: componente aritmetico implementado y comprobado en el dominio. No existe
todavia un flujo operativo de plazos, endpoint o persistencia para este resultado. El calendario usado conserva el contrato de
[judicial-calendars-api.md](judicial-calendars-api.md).

## Frontera

El componente recorre fechas civiles clasificadas por valores inmutables de un
calendario. Obtiene una fecha candidata al alcanzar una cantidad de dias
computables, o se detiene con una causa precisa. No interpreta una notificacion,
resuelve su eficacia, selecciona un calendario ni decide la regla juridica.

La primera fecha incluida y la cantidad son entradas de la operacion matematica.
El futuro caso de uso debera derivarlas de hechos y de un perfil aplicable,
conservar sus soportes y vincular identidad, revision y digests del calendario.
Proporcionar valores al componente no acredita esa vinculacion ni autorizacion.
Tampoco convierte una fecha candidata en una obligacion activa o vencida.

## Entradas y limites

- Valores validos `JudicialCalendarValues`, sin consultar otra revision.
- Primera fecha incluida `CivilDate`, gregoriana, entre 0001-01-01 y 9999-12-31.
- Cantidad positiva representada por `NonZeroU32`; cero no forma una entrada.
  La cantidad no determina una reserva de memoria ni el presupuesto de recorrido.

La cobertura del calendario tiene como maximo 1096 fechas. Un recorrido puede
inspeccionar como maximo esas fechas y una fecha posterior que evidencie falta
de cobertura: 1097 pasos. Si el inicio esta fuera de cobertura, el primer paso
bloquea. No se busca otra cobertura ni se salta hasta el primer dia disponible.
Este limite es operativo y procede del calendario; no es un maximo legal de plazo.

## Recorrido y resultado

1. Clasificar la primera fecha incluida. Conservar en la traza el dia completo,
   con origen de regla semanal o excepcion, fuentes, explicacion y acumulado.
2. Si es computable, incrementar el acumulado; si alcanza la cantidad requerida,
   detenerse y devolver esa fecha como candidata. No inspeccionar dias posteriores.
3. Si es excluido, conservar el acumulado y continuar al dia civil siguiente.
4. Si esta sin resolver, conservar el acumulado y detenerse en esa misma fecha
   con causa `Unresolved`. No reemplazarla por dia computable o excluido.
5. Si falta cobertura, conservar el acumulado y detenerse en esa fecha con causa
   `OutsideCoverage`. No asignar fuentes o una regla que no existe.
6. Si se alcanza 9999-12-31 sin completar ni encontrar otra causa de bloqueo,
   devolver `DateRangeExhausted` despues de esa fecha. No crear el ano 10000,
   saturar el contador de fecha ni repetir el ultimo dia.

La traza es contigua e incluye el dia que produjo el bloqueo, cuando existe.
Cada acumulado cuenta unicamente los dias computables de ese prefijo. El agotamiento
del rango de fechas no anade un dia ficticio a la traza. El resultado conserva
primera fecha, cantidad y acumulado final; una salida bloqueada no tiene candidata.

La clasificacion procede del calendario completo. Un sabado declarado computable
se cuenta; no existe una regla lunes-viernes incorporada en el algoritmo. Una
excepcion sustituye la regla semanal por la semantica ya definida del calendario.
No hay otra correccion de fin de semana despues de obtener la candidata.

## Corpus aritmetico sintetico

Los casos siguientes no representan un calendario oficial ni acreditan una regla
del CNPP. El fixture expresa lunes-viernes computables y fin de semana excluido,
con fuentes sinteticas. La primera fecha ya es una entrada del conteo; no se
infiere como dia siguiente de una notificacion.

| Primera fecha incluida | Cantidad | Excepcion adicional | Fecha candidata |
| --- | --- | --- | --- |
| 2026-01-06 | 10 | Ninguna | 2026-01-19 |
| 2026-01-10 | 10 | Ninguna | 2026-01-23 |
| 2026-01-10 | 10 | 2026-01-14 excluido | 2026-01-26 |
| 2028-02-22 | 10 | Ninguna | 2028-03-06; incluye 2028-02-29 |

Las pruebas negativas deben separar falta de cobertura y clasificacion sin
resolver, inicio anterior a cobertura, reglas completamente excluidas y agotamiento
del ano maximo. Una cantidad de `u32::MAX` debe terminar de forma acotada al
agotarse la cobertura. Una incertidumbre posterior a la candidata no invalida el
prefijo ya calculado. La traza debe preservar origen y fuentes de cada regla.

## Integracion posterior

El caso de uso de plazos debe resolver por separado los hechos de resolucion y
notificacion, destinatario y representacion, perfil de calculo, aplicabilidad,
calendario exacto, responsable, canal de recepcion y regla temporal. Necesita
persistencia y auditoria atomicas, control de concurrencia, reevaluacion durable
y autorizacion antes de comunicar resultados. Ninguno de esos controles se
simula mediante este componente puro.

La fecha candidata carece de hora, desfase y zona. No se le asignan medianoche,
23:59:59 o la zona del navegador; tampoco se programan alertas de 48/24 horas.
La [aritmetica temporal](deadline-arithmetic.md) agrega meses civiles, horas
transcurridas y politicas diarias explicitas, con corpus matematico propio.
La investigacion complementaria requiere ademas un perfil normativo sustentado;
no se convierten meses en treinta dias ni se declara cumplido ese supuesto con
unicamente el algoritmo. Los extremos pendientes estan documentados en
[la investigacion de reglas](deadline-rule-research.md).
