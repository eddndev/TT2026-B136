# Observación de procesos terminados en las pruebas de restauración

La prueba de tiempo máximo exige que el proceso hijo termine y que un proceso
ajeno continúe vivo. Un proceso terminado puede desaparecer de `/proc` entre
comprobar su existencia y leer su estado. Linux también puede devolver
`ProcessLookupError` al leer una entrada que acaba de ser retirada.

La observación realiza una sola lectura. Solamente `FileNotFoundError` y
`ProcessLookupError` representan ausencia; el estado `Z` representa un proceso
terminado pendiente de recolección. Los errores inesperados siguen fallando.
El nombre del proceso puede contener espacios. Se conservan el límite de un
segundo del comando, los dos segundos de observación y la comprobación del
proceso ajeno. El ejecutor de restauración no cambia.

## Verificación reproducida

El 3 de octubre de 2026 se añadió primero la regresión determinista, que falló
por la ausencia del observador. Tras implementarlo, aprobaron seis pruebas en
1.066 s: tres del observador y las tres pruebas existentes del ejecutor. Cubren
desaparición durante lectura, estados activo/zombi, errores de permisos, límites
de salida, diagnósticos privados y limpieza de procesos reales por tiempo máximo.
La regresión completa remota se verifica para la revisión publicada.
