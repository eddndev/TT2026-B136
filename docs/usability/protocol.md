# Protocolo de observación de uso de Qadra

## Propósito y alcance

Observar comprensión, finalización de tareas, errores, ayudas y barreras de
presentación en los flujos integrados. Complementa la validación funcional del
objetivo de validación del prototipo descrito en
[los objetivos aprobados](../../latex/chapters/01-introduccion.tex). No modifica
esos objetivos ni sustituye la matriz positiva/negativa de cada caso de uso.

La evaluación es descriptiva y moderada. No se usa una escala validada de
satisfacción ni se calcula una puntuación que se presente como tal. Los comentarios
abiertos se interpretan como observaciones de estas sesiones, sin generalizarlos
a toda la población profesional.

## Antes de registrar participantes

Registra responsable, revisión del protocolo, revisión del producto, dirección
del entorno sintético, módulos habilitados, roles a observar, dispositivos y
las tareas previstas. Indica si se trata de `ensayo_tecnico`, `piloto_humano` o
`evaluacion_humana`; sus resultados se mantienen separados.

Invita voluntariamente a personas del despacho con tareas de administración,
litigación o asistencia legal. El rol real de trabajo y el rol de la cuenta de
prueba son campos distintos: no asignes experiencia por el rol simulado. Registra
experiencia de manera general, sin nombres, clientes ni asuntos atendidos.
Informa qué perfiles no estuvieron representados. No se fija aquí un tamaño de
muestra que aparente haber sido aprobado o que permita prometer representatividad.

Usa identificadores como `P001` y `S001`; son convenciones de nombrado, no personas
ya reclutadas. La persona puede retirarse o no responder. Obtén consentimiento
antes de registrar sus observaciones. Las capturas requieren autorización separada;
no se realizan grabaciones de audio o vídeo con este kit.

## Entorno y material

Usa una instancia desechable o un espacio aislado sin información real. Prepara
cuentas individuales de práctica y no registres sus contraseñas, semillas TOTP,
códigos ni tokens en las hojas. El enrolamiento y su entrega privada se preparan
fuera de las tareas medidas. El inicio de sesión de la tarea sí se mide; conserva
el método MFA utilizado porque TOTP y recuperación tienen acciones distintas.

Los archivos y expedientes se describen en [fixtures](fixtures/README.md). La
persona moderadora verifica servicios, reloj, permisos y estado inicial antes de
cada sesión. Cada participante usa recursos independientes; para Owner, el aislamiento exige
una instancia de práctica separada o un estado sintético inicial restaurado entre
sesiones. No reutilices un expediente alterado sin comprobar su estado inicial.

Para diseño responsivo, el catálogo aprobado contempla 768 px y escritorio.
Incluye una sesión o repetición documentada a 768 px y registra ancho, alto, zoom,
dispositivo, entrada táctil/ratón y si hubo emulación. Una captura móvil de 390 px
puede aportar evidencia adicional, pero no reemplaza la observación a 768 px.
Los tiempos manuales de tarea no prueban el umbral técnico de 300 ms de respuesta
visual descrito en [análisis y diseño](../../latex/chapters/03-analisis-diseno.tex).

## Desarrollo de una sesión

1. Explica que se evalúa la interfaz, no el desempeño laboral. Ofrece retirarse.
2. Recoge consentimiento y entrega la hoja del participante.
3. Explica el manejo del entorno ficticio, sin enseñar las soluciones de las tareas.
4. Lee una tarjeta por vez. Aclara sólo el significado del escenario antes de iniciar.
5. Inicia el cronómetro cuando la persona confirma que entendió y puede actuar.
6. Observa sin dirigir. Puede expresar lo que espera encontrar; no es obligatorio
   hablar continuamente. Registra esa condición si el pensamiento en voz alta
   afecta comparabilidad de tiempos.
7. Detén el cronómetro al alcanzar el resultado observable, declarar abandono,
   retirar consentimiento o llegar al límite operativo acordado.
8. Formula las preguntas abiertas de la hoja sin corregir de inmediato su respuesta.
9. Cierra sesión, recoge comentarios y explica cómo solicitar el retiro de sus datos.

La selección base sugerida es acceso, localizar expediente, cargar documento,
verificación o sellado según rol, versión y cierre de sesión. Reserva 40–55 minutos
como previsión organizativa, no como resultado ni criterio de aprobación. Añade
agenda, asignaciones o módulos condicionales sólo si cabe en el tiempo aceptado.
Los límites por tarea de `tasks.md` son límites de observación, no promesas de
rendimiento: acuerda sus cambios antes de la sesión y regístralos.

## Medición y clasificación

Registra inicio y fin con zona horaria y cronómetro en segundos. El tiempo total
incluye espera del producto, lectura, correcciones y búsqueda en la interfaz.
Sólo descuenta interrupciones ajenas a la tarea, anotando motivo y duración;
no descuentes lentitud o errores del producto para mejorar el resultado.

Distingue:

- `completada_sin_ayuda`: se alcanza el resultado sin pistas, manual o intervención.
- `completada_con_ayuda`: se alcanza después de usar el manual, una pista o intervención.
- `no_completada`: se intentó, pero no se alcanzó el resultado; conserva el motivo.
- `no_iniciada`: estaba planificada, pero no comenzó; informa la causa.
- `interrumpida_tecnica`: un fallo del entorno impidió observar la tarea; requiere
  evidencia de ese fallo, no sólo dificultad de uso.
- `retirada`: la persona decidió detener su participación; aplica su decisión de
  conservación o eliminación de datos.
- `no_aplicable`: tarea incompatible con el rol o módulo no habilitado; no es fracaso.

Una duda o pausa no constituye automáticamente un error. Cuenta como error una
acción observable que aparta del resultado o una interpretación incorrecta
confirmada. Registra qué ocurrió, cómo se recuperó y si el producto ayudó. No
atribuyas causas internas sin evidencia técnica. Repetir la consigna sin dar
pistas es aclaración neutral; enseñar ubicación o siguiente acción sí es ayuda.

## Análisis y cierre

Agrupa por revisión del producto, tarea/variante, rol de cuenta, tipo de sesión
y condición de dispositivo. Analiza por separado primer intento y repetición;
la práctica previa produce aprendizaje. No mezcles ensayos automáticos con personas.

Informa conteos de tareas planificadas, iniciadas, completadas con/sin ayuda y
no completadas, además de interrupciones, retiros y no aplicables. Conserva los
denominadores y registros faltantes. Para tiempos, informa cuántos registros
válidos sustentan mediana y rango; separa completadas de no completadas para no
ocultar intentos largos fallidos. No rellenes ausencias con cero.

Relaciona cada problema con observaciones y, cuando esté autorizado, capturas
sin secretos. Prioriza por impacto observado: impide completar, requiere ayuda
o dificulta comprensión. Una corrección necesita una comprobación posterior
identificada; no cambies la clasificación original para simular que nunca ocurrió.

La salida es un informe de lo observado, perfiles/dispositivos no cubiertos,
limitaciones, correcciones y nueva evidencia cuando exista. Mantén separados
los objetivos pendientes de firma personal, reglas jurídicas, privacidad Client
y demás alcance de [cierre del producto](../product-completion.md).
