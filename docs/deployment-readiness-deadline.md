# Plazo compartido de comprobación de disponibilidad

`Runtime.check(target, *, deadline=None)` admite un plazo absoluto obtenido de
`time.monotonic()`. El llamador conserva un único plazo y lo transmite al
comprobar los servicios; no debe construir una duración nueva para cada sonda.
La llamada existente sin ese argumento conserva sus límites: diez segundos por
comando de dependencia y cinco por apertura HTTP.

Con un plazo explícito, la comprobación rechaza valores no finitos, booleanos y
plazos agotados antes de iniciar las sondas. PostgreSQL, Redis, la API directa,
los archivos y respuestas de versión, el HTML y el proxy comprueban el remanente
antes y después de cada operación. Los procesos y aperturas HTTP reciben como
máximo su límite existente y el tiempo restante. Una respuesta correcta que
llega tarde, incluido el 401 final esperado, no confirma disponibilidad.

La lectura HTTP con plazo limita el cuerpo de versión a 64 KiB y el HTML a
1 MiB. Usa lecturas parciales `HTTPResponse.read1` y reduce el timeout del socket
antes de cada lectura, comprobando el plazo otra vez al regresar. Un exceso de
contenido o de tiempo rechaza la sonda y cierra la respuesta. La implementación
utiliza la superficie `fp.raw._sock` de `HTTPResponse` de CPython para ajustar
el socket; si una respuesta abierta no la expone, rechaza la operación en lugar
de continuar sin ese límite. No reemplaza el parser HTTP ni crea trabajadores,
hilos o alarmas para abandonar operaciones que sigan vivas.

Los errores HTTP de versión y HTML también se cierran explícitamente antes de
propagarse. Con plazo se devuelve un error neutral; sin plazo se conserva la
excepción original y sus datos, liberando siempre su respuesta.

## Alcance del plazo

El timeout de `urllib` es un límite de inactividad de socket. La comprobación
posterior rechaza encabezados que terminen después del plazo, pero no impone
una interrupción total de la lectura interna de encabezados o delimitadores de
transferencia si el interlocutor los entrega lentamente sin agotar cada timeout.
Tampoco puede interrumpir una
operación bloqueada del filesystem: comprueba el plazo cuando ésta regresa.
Por ello esta API no promete una duración total dura frente a esas situaciones.

La comprobación está destinada a servicios privados de loopback y a archivos
locales del release. El coordinador debe conservar el cierre ante error o una
respuesta tardía. Superar readiness no autoriza por sí solo una instalación,
reapertura o cambio de confianza. Los ensayos focales del plazo y la aceptación
del procedimiento que lo utilice son evidencias separadas.
