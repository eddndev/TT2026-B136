# Preparación contextual en pruebas de navegador

El helper de `web/tests/browser/resource-deadline-helpers.mjs` distingue la
respuesta HTTP de la presentación de su resultado. Instala la espera antes del
clic y exige el POST del expediente y recurso exactos, sin query adicional,
con respuesta 200. Después conserva la comprobación original de confirmación
deshabilitada y marca el reconocimiento explícito de las capturas.

No cambia timeouts, reintentos, comportamiento de producto ni autorizaciones.
La regresión `resource-deadline-preparation.spec.mjs` devuelve una preparación
válida seis segundos después de la única solicitud. Antes, esos seis segundos
consumían el límite de cinco segundos de la aserción de interfaz; después,
la aserción empieza tras la respuesta y sigue usando su límite original.
La regresión exige una preparación, ningún envío y revisión visible antes de
habilitar la confirmación mediante el reconocimiento explícito.

## Evidencia del 3 de octubre de 2026

El CI de `main` falló esperando el botón de confirmación en un caso de cierre
administrativo. Los dos casos originales aprobaron un único focal local sobre
ese mismo código en 11.4 s; por tanto no se atribuye con certeza el fallo remoto
a una respuesta lenta. La regresión controlada sí reprodujo el defecto de
sincronización del helper. Tras corregirlo, los tres casos aprobaron en 19.6 s
con un worker sobre el frontend de `main` y estos cambios de pruebas.

Un primer intento de verificación volvió a cargar por error el helper anterior
al proyectar el frontend base; reprodujo el mismo RED y no constituye evidencia
de la corrección. Se corrigió el verificador local, se comprobó la identidad del
helper ensayado y se restauraron todas las fuentes locales por hash al terminar.
La estabilidad completa requiere la campaña remota de la cabeza corregida.
