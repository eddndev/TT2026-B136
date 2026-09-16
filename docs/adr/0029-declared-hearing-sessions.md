# 0029. Sesiones y resultados declarados con fuentes históricas exactas

## Status

Accepted. Implementado y verificado localmente; pendiente de integración.

## Context

La programación conserva citas, cambios organizativos y sus participantes
vinculados. Una fecha pasada no acredita actuaciones, asistencia ni conclusión.
El catálogo aprobado también requiere resultados, asistentes y acuerdos; sus
plazos y calendario necesitan datos y reglas propios.

El [CNPP oficial](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf), consultado
el 16 de septiembre de 2026 y con última reforma indicada del 28 de noviembre de
2025, distingue registro, resolución oral/escrita y continuidad de actuaciones
(artículos 61, 67, 307, 313-315 y 351-352). Estas distinciones orientan el modelo;
las restricciones siguientes son decisiones de ingeniería, sin acreditar actos.

## Decision

Cada sesión o acto informado tiene una raíz y revisiones propias. Fija una
revisión exacta de programación del mismo expediente, incluso histórica o
cancelada. Una continuación crea otra raíz con antecedente exacto preexistente
e inmutable; una rectificación no representa la continuación de un acto.
La programación puede seguir cambiando sin sustituir esas fuentes históricas.

Ocurrencia y alcance son declaraciones separadas. Las comparecencias son
fichas históricas exactas, con calidad y observación declaradas; se admiten
aunque no se inicie el acto. No se deducen ausencias ni notificaciones. Los
acuerdos tienen UUID y texto con orden explícito, sin clasificación judicial
inferida ni efectos automáticos. Relato, clase de antecedente, localizador y
soporte documental exacto conservan su propia procedencia.

Alta, rectificación y retiro usan preparación sin reservas y recibos exactos
de operación. Retirar es terminal y conserva contenido e historia; no anula
el acto. Se comprueban permiso y expediente activo bajo el bloqueo transaccional
común, capturando administración y reloj vigentes. Una etapa posterior o una
edición administrativa no impiden por sí mismas registrar hechos anteriores.
La precisión temporal conserva fecha o instante, sin inventar una hora.

Los cánones HRES1 y HRTX1 vinculan valores y envío respectivamente.
Auditoría y persistencia comparten transacción. La operación comprueba fuentes,
secuencia y permisos; el inventario de arranque valida además todas las aristas,
la ausencia de ciclos y el catálogo. La rectificación vuelve a admitir el soporte
propuesto, aunque su referencia sea idéntica; retiro e historia conservan la
admisión original.

El contrato detallado es [hearing-results-api.md](../hearing-results-api.md).
El cuerpo JSON tiene límite propio de 512 KiB; los límites de programación
existentes permanecen sin cambios. La historia devuelve resúmenes acotados y
el detalle exacto se consulta por selección explícita. Detalle, revisión exacta,
preparación y escrituras rechazan parámetros de query no admitidos.

## Consequences

- Se conservan sesiones y continuaciones sin cambiar los cánones de programación.
- Las referencias históricas pueden estar retiradas o archivadas: su estado queda
  visible y no equivale a una acreditación del hecho que describen.
- Una corrección de ancla o antecedente exige retirar el registro equivocado y
  crear otro; no se sustituyen las fuentes de una raíz existente.
- Las referencias a acuerdos futuros necesitarán raíz, revisión y UUID exactos.
- El cálculo de plazos requerirá antecedentes estructurados, sujetos, reglas y
  calendarios verificados; no se extraerá de texto libre ni de la asistencia.
- La navegación de continuación lleva al antecedente exacto. No se promete un árbol
  inverso, una búsqueda global ni un nuevo calendario con esta entrega.
