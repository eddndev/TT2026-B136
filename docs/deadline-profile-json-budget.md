# Presupuesto JSON de las definiciones de perfiles de plazo

## Estado y alcance

Se propone un límite de **16 MiB (16 777 216 bytes)** para el cuerpo JSON
editable de un perfil. La suma conservadora descrita aquí es **15 607 604
bytes**, con un margen de **1 169 612 bytes** frente a ese límite.

Es una derivación del tamaño de una representación, no una medición de una
petición ejecutada ni un resultado de pruebas HTTP. Suma máximos de campos y
variantes que no necesariamente pueden coexistir: **no afirma que esa cota sea
alcanzable por una definición válida**. Tampoco fija límites normativos ni
acredita aplicabilidad de un perfil.

El alcance funcional está en [el contrato de ciclo de vida](deadline-lifecycle.md).
La estructura y sus límites proceden de:

- [Modelo de definición](../crates/application/src/deadline_profiles/model.rs),
  [validación del corpus](../crates/application/src/deadline_profiles/validation.rs)
  y [corte civil](../crates/application/src/deadline_profiles/cutoff.rs).
- [Ámbito de calendario](../crates/domain/src/judicial_calendars/scope.rs),
  [referencias](../crates/domain/src/judicial_calendars/source.rs),
  [reglas](../crates/domain/src/judicial_calendars/rules.rs) y
  [colecciones](../crates/domain/src/judicial_calendars/values.rs).
- [Proyección JSON de calendario](../crates/web/src/judicial_calendars/projection.rs),
  reutilizada completa en cada ejemplo.
- [Textos declarados](../crates/domain/src/procedural_facts/text.rs),
  [tiempo declarado](../crates/domain/src/procedural_time.rs) y
  [tipos aritméticos](../crates/domain/src/deadline_arithmetic/types.rs).

## Reglas de la cuenta

Se cuenta JSON compacto, sin espacios sintácticos superfluos, con valores ya
normalizados y UUID con guiones. Para sobreestimar también las formas ASCII
escapadas se usan estas funciones:

- `U(n) = 2 + 12*n`: cadena de hasta `n` escalares Unicode; las comillas se
  incluyen y cada escalar puede ocupar dos escapes de sustitutos UTF-16.
- `A(n) = 2 + 6*n`: cadena ASCII de hasta `n` caracteres, incluso si cada
  carácter se escribe mediante un escape Unicode. Se aplica también a claves,
  UUID, fechas, etiquetas de variantes y URL oficiales, que son ASCII.
- `L(n,x) = 2 + n*x + max(n-1,0)`: lista de hasta `n` elementos de tamaño `x`.
- `O(fields) = 2 + sum(A(len(key)) + 1 + size(value)) + max(count(fields)-1,0)`:
  objeto con comillas de claves, dos puntos, comas y llaves.

Los UUID cuestan `A(36)`, las fechas civiles `A(10)` y los enteros se cuentan
por sus dígitos, incluyendo el signo cuando corresponde. `u32` ocupa hasta 10
caracteres; segundos Unix firmados, 20; nanosegundos, 10; desfases, 6. Para los
opcionales se carga la variante presente cuando es mayor que `null`.

No hay cota finita para espacios JSON arbitrarios o texto adicional que un
constructor eliminaría al normalizar. Rechazar esos cuerpos por tamaño sigue
siendo una política de recursos. No se promete aceptar toda escritura
posible equivalente a un mismo valor normalizado.

## Calendario incrustado

El ámbito contiene cuatro textos de 200 escalares, uno de 1000, jurisdicción
de hasta siete caracteres y hasta 32 códigos de entidad de dos caracteres.
Cada referencia incluye UUID, dos textos de 200, URL ASCII de 2048, dos fechas
y localizador de 512 escalares. Cada regla contiene clasificación de hasta 13
caracteres, hasta 16 UUID y explicación de 256 escalares.

Se incluyen siete reglas semanales y hasta 64 excepciones; cada excepción
agrega UUID y dos fechas. Se conservan las claves actuales `scope`, `coverage`,
`sources`, `weekly_pattern` y `exceptions` y los nombres de sus proyecciones.

| Bloque | Cota en bytes |
| --- | ---: |
| Ámbito de calendario | 22 566 |
| Referencia pública | 23 947 |
| Regla semanal, con `weekday` | 6 929 |
| Excepción, con `id`, `from` y `through` | 7 314 |
| Calendario completo, con 16 referencias y 64 excepciones | 922 891 |
| Dieciséis calendarios completos | 14 766 256 |

Estos límites son independientes de los **191 910 bytes** máximos de JCAL1:
JSON incorpora nombres de campos, UUID textuales y escapes que no existen en
el formato binario. Usar la cota binaria como límite HTTP omitiría ese coste.

## Definición y sobre de referencia

La definición contiene `title`, `description`, `scope`, `references`,
`trigger`, `template`, `completion`, `conditions` y `examples`. Se cargan 16
referencias, 16 condiciones y 16 ejemplos, con hasta 16 identificadores de
referencia por condición y por ejemplo. El ámbito global se envuelve como
`{kind,value}`; el de expediente resulta más pequeño.

Para mantener una cota conservadora se suman, aunque sean incompatibles:

- En `trigger`, `kind`, `field`, `purpose` y `family`; se reservan 12, 40, 40 y
  20 caracteres ASCII respectivamente.
- En `template`, `kind`, `rule`, `unit` y `maximum`. Cada objeto de regla o
  unidad carga `kind`, `quantity`, `inclusion`, `basis` y `final_day`; las
  etiquetas reservan 13, 12, 18 y 14 caracteres donde corresponden.
- En `completion`, `kind`, `time`, `offset_seconds`, `from`, `through`,
  `channel` y `reference_id`, con etiqueta de 20 caracteres y canal de 200
  escalares. La hora conserva ocho caracteres.
- En el tiempo del ejemplo, `precision`, `year`, `month`, `day`, `hour`,
  `minute`, `second` y `offset_seconds`, sin que esa suma invente precisión.
- En `expected`, `kind`, `outcome` y `block`. El resultado suma `kind`, `date`,
  `instant` y `block`; el instante contiene segundos Unix, nanosegundo y
  desfase. El bloque suma `kind`, `observed`, `year`, `month`, `requested_day`,
  `date`, `maximum` y `supplied`. Se reservan hasta 40 caracteres para su
  etiqueta, 30 para la del resultado y siete para la precisión observada.

| Bloque | Cota en bytes |
| --- | ---: |
| Ámbito global envuelto | 22 667 |
| Desencadenante | 829 |
| Plantilla | 1 367 |
| Finalización | 3 263 |
| Condición | 15 882 |
| Tiempo declarado de ejemplo | 403 |
| Expectativa de ejemplo | 2 295 |
| Ejemplo completo con calendario | 932 113 |
| Definición completa | 15 594 134 |
| Sobre de referencia con definición | **15 607 604** |

El sobre usado para la última fila contiene `operation_id`, `profile_id`,
`change` y `expected_submission_digest`. `change` carga `action`,
`expected_revision`, `definition` y `reason`; el motivo reserva 1000 escalares
y el resumen SHA-256 reserva 64 caracteres ASCII. Esta es una cuenta de
referencia para fijar el presupuesto, no la especificación definitiva de
rutas o envoltorios HTTP.

## Verificación al conectar HTTP

La implementación HTTP debe comprobar el presupuesto sobre bytes recibidos y
rechazar el exceso con `413`. Los casos representativos deben conservar
calendarios completos y texto astral escapado; un único documento pequeño o
un máximo binario no bastan como evidencia del límite JSON.

Al fijar los DTO definitivos se debe cotejar sus nombres, envoltorios y cupos
con esta cuenta. Un cambio en ellos exige revisar la derivación. Las variantes
incompatibles sumadas aquí nunca deben aceptarse como objeto válido: el lector
estricto sigue rechazando campos ajenos. Las pruebas de reproducción del
corpus y [el lector DPRF1](../crates/application/src/deadline_profiles/encoding/mod.rs)
conservan sus propias obligaciones, separadas del presupuesto HTTP.
