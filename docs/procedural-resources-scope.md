# Alcance funcional de los recursos procesales

Estado: alcance completo y conciliación académica, con implementación parcial
documentada abajo. La revisión de fuentes del 15 de septiembre de 2026 se
conserva como antecedente; esta actualización técnica no es una nueva revisión
normativa. No modifica objetivos, criterios ni casos de uso aprobados.

## Implementación disponible y frontera pendiente

El [registro declarado](procedural-resources-api.md) implementa recursos, actos,
archivo/reactivación e historia con resoluciones y soportes exactos. Su evidencia
incluye aceptación API/restauración y navegador real. La ampliación de
[asociaciones existentes](resource-activities-api.md) implementa vínculos
independientes del recurso y acto a revisiones exactas de audiencias o plazos.
Conserva esa historia separada del estado actual de la actividad; al desvincular
no cancela la audiencia, retira el plazo ni modifica sus alertas. Sus pruebas
focales, la aceptación API con restauración y el navegador real están aprobados.
CI del incremento sigue pendiente según el [informe](verification-report.md).

Este incremento no crea audiencias desde un contexto procesal de recurso ni
activa automáticamente términos. Faltan la creación contextual coherente, el
corpus jurídico calificado y su activación durable, y completar la navegación y
trazabilidad de las alertas en el flujo de recursos. Los avisos existentes siguen
perteneciendo a la actividad y conservan su política de destinatarios y episodios;
una asociación no representa una suscripción ni una alerta nueva. Estas
fronteras no reducen los criterios de aceptación completos de esta nota.

## Decisión propuesta

Mantener Investigación, Intermedia y Juicio en el recurso de
[etapas del expediente](case-stages-api.md). Incorporar **Recursos** como registros
vinculados al expediente y a resoluciones con soporte documental exacto, con
historia propia y plazos vinculados al calendario. No agregar una transición
Juicio -> Recursos.
La separación es una inferencia de diseño fundada en los artículos siguientes;
no elimina recursos del alcance aprobado.

## Fuente normativa y consecuencias

El [registro oficial de reformas del CNPP](https://www.diputados.gob.mx/LeyesBiblio/ref/cnpp.htm)
identifica el 28-11-2025 como última reforma; también registra la reforma del
artículo 467 del 26-01-2024. No fijar el catálogo de apelaciones desde una edición
anterior. Se verificó el [texto consolidado oficial](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).

| Artículos del CNPP | Base normativa resumida |
| --- | --- |
| 211 | El procedimiento tiene investigación, intermedia y juicio; investigación comprende dos fases. |
| 456-458 | Revocación y apelación impugnan resoluciones; se distinguen parte afectada y condiciones de interposición. |
| 465-468 | La revocación puede presentarse en distintas etapas con intervención judicial; existen apelaciones de resoluciones anteriores al juicio. |
| 463, 472 | No toda interposición suspende la ejecución; hay excepciones, incluida exclusión de pruebas. |
| 470, 475, 479-482 | Admisibilidad y resolución competen al órgano judicial; los resultados pueden incluir reposición. |
| 466 | Revocación oral antes de terminar la audiencia; escrita, dos días siguientes a la notificación. |
| 471 | Apelación: tres días para auto/providencia del juez de control, cinco para su sentencia definitiva, diez para sentencia del tribunal de enjuiciamiento; existe otro supuesto de tres días por desistimiento. |
| 82, 84, 87, 94-96, 473 | Modalidad/efectos de notificación, inhábiles y excepciones, reposición y adhesión afectan el tratamiento de términos. |

Fuente de la tabla: [CNPP, artículos indicados](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).
La tabla orienta el contrato; no sustituye verificar cada supuesto antes de
codificar una regla. Los [días inhábiles publicados por el OAJ](https://www.oaj.gob.mx/transparencia/paginas/diasinhabiles.htm)
usan fundamentos y ámbitos distintos, con excepciones y días inhábiles pero
laborables. No constituyen un calendario penal universal ni el calendario local
del despacho por defecto.

## Trazabilidad y diferencia que debe conciliarse

| Fuente versionada | Contenido conservado y consecuencia |
| --- | --- |
| [Introducción, objetivos específicos y tabla de criterios](../latex/chapters/01-introduccion.tex) | El objetivo de gestión pide estructura/terminología CNPP, expedientes, etapas, audiencias, plazos y participantes. Su criterio OE-2 contiene la clasificación de cuatro etapas reproducida abajo. |
| [Análisis y diseño, HU-14 a HU-16 y RF-05](../latex/chapters/03-analisis-diseno.tex) | Describen avance e historia con documentos habilitantes; RF-05 enumera Investigación, Intermedia y Juicio Oral. No definen el ciclo de un recurso. |
| [Análisis y diseño, CU-07](../latex/chapters/03-analisis-diseno.tex) | Exige perfil completo, estado administrativo activo, asignación vigente y etapa Investigación o Intermedia para un avance ordinario. Su flujo conserva esas dos transiciones; no define una transición desde Juicio ni el ciclo de recursos. |
| [Análisis y diseño, CU-08 a CU-10](../latex/chapters/03-analisis-diseno.tex) | Audiencias, calendario y alertas necesitan vinculación adicional a recurso/acto. CU-08 condiciona audiencia a etapa; la extensión debe describirse antes de evaluar audiencias asociadas a recursos. |
| [Marco teórico, Recursos, Impugnación y Ejecución; tabla de plazos](../latex/chapters/02-marco-teorico.tex) | Presenta recursos después de sentencia y un plazo general de diez días para apelación de sentencia. Debe conciliarse el alcance transversal y distinguir autoridad/resolución. Ejecución no se incorpora silenciosamente al módulo de recursos. |
| [Cierre funcional](product-completion.md) | Distingue registro declarado, asociaciones a actividades existentes y cierre del flujo completo, con evidencia separada por entrega. Los criterios siguientes conservan el alcance restante. |

El criterio aprobado de OE-2 permanece literalmente:

> CRUD completo para expedientes, audiencias y plazos; calendario jurídico operando sobre las cuatro etapas del CNPP (investigación, intermedia, juicio oral y recursos); cómputo automático de términos conforme al artículo 94 del CNPP.

**Redacción exacta propuesta para revisión, todavía no aplicada:**

> CRUD completo para expedientes, audiencias y plazos; calendario jurídico operando sobre las etapas de investigación, intermedia y juicio del artículo 211 del CNPP y sobre los recursos de revocación y apelación; cómputo automático de términos conforme al artículo 94 del CNPP.

La propuesta corrige la clasificación y conserva las funciones exigidas. Para
el catálogo se propone añadir, sin renumerarlo todavía, el flujo **Gestión de
recursos y términos asociados**: registrar resolución impugnada, consultar y
actualizar el seguimiento, registrar actos con soportes exactos, conservar
historia y asociar audiencias, términos calculados y alertas. Su aceptación
queda explicitada abajo; no se atribuye retrospectivamente a CU-07.

## Contrato funcional mínimo propuesto

### Registro, actos y evidencia

- Identidad propia por recurso, `case_id`, revisión positiva e historia inmutable
  auditada. Varios recursos pueden coexistir, incluso sobre la misma resolución.
  No deduplicar por nombre de persona, tipo o documento de forma que impida
  registros distintos; una posible coincidencia solo orienta la consulta.
- Tipo declarado revocación/apelación; modalidad oral/escrita cuando corresponda;
  título organizativo, autoridad emisora, referencia conocida de la resolución,
  parte impugnada y motivo. Autoridad receptora se incorpora cuando se conozca.
- Resolución vinculada a `document_id`, versión positiva y digest del mismo
  expediente. El servidor comprueba autorización, integridad y captura nombre,
  versión y digest; nunca confía en la pertenencia declarada por el cliente.
  Una clasificación actual del archivo no prueba su naturaleza jurídica.
- Recurrente mediante selector del directorio, con identidad de ficha opcional
  y nombre/rol declarados capturados. No exigir UUID manual ni equiparar persona
  procesal con cuenta de acceso. Una edición posterior de la ficha no reescribe
  la autoría o identidad declaradas en un registro histórico.
- Separar preparación organizativa de interposición declarada. Registrar esta
  última exige datos del acto y soporte de presentación o constancia oral;
  nunca convertir una carga de archivo en presentación ante un tribunal.
- Actos posteriores: admisión, inadmisibilidad, desistimiento y resolución
  declarados, cada uno con soporte exacto. Archivo/reactivación son operaciones
  organizativas y no equivalen a desistimiento, firmeza o pérdida de derechos.
- Fechas de resolución, notificación e interposición separadas, con precisión y
  desfase explícitos; actor/correo y fecha de registro del servidor aparte.
  Lo desconocido permanece ausente. No inventar horas ni usar fecha de upload
  como notificación. La etapa observada al capturar es contexto del sistema.
- Las correcciones crean revisiones; los actos y soportes anteriores permanecen
  consultables. Un resultado declarado no modifica ni borra automáticamente
  [la historia de etapas](case-stages-api.md). Los efectos procesales adicionales
  requieren contrato propio; no se dan por resueltos con un texto libre.

La selección de soporte reutiliza lista -> versiones -> detalle exacto. Debe
admitir constancias de actos orales; la política PDF/DOCX de etapas no se hereda
como una exigencia jurídica de recursos. Cualquier formato adicional necesita
límites, validación y pruebas antes de habilitarlo.

### Operaciones, permisos e interfaz

Alta con R1, consulta de cabeza, lista paginada y filtros, edición con revisión
esperada, registro de actos, historia descendente y archivo/reactivación de
intención exclusiva. Cada mutación y su auditoría se confirman juntas. En un
conflicto se conserva el borrador y se compara la cabeza antes de reenviar.
Un resultado incierto se consulta en lista/historia; una coincidencia no prueba
que un envío concreto haya sido confirmado.

Política propuesta: Owner gestiona todos; Litigator asignado gestiona; Paralegal
asignado consulta; Client no accede. Revalidar pertenencia y estado al confirmar.
Cierre administrativo bloquea cambios, conserva lectura/evidencia y permite
revocar acceso; no detiene el transcurso de términos jurídicos.

Qadra muestra **Recursos** dentro del expediente, conservando componentes y
estilos existentes. El historial se abre bajo demanda, sin consultas por fila.
La ficha coordina peticiones y recargas antes de habilitar acciones vecinas.
Cambio de sesión/expediente o denegación invalida respuestas pendientes y limpia
contenido protegido. Una carga confirmada se conserva si falla el registro del
recurso, con explicación separada de ambos resultados.

### Plazos y calendario necesarios para completar el alcance

El mínimo funcional incluye términos **calculados**, audiencias asociadas y
alertas; no solo fechas introducidas a mano. El calendario debe admitir relación
con expediente y recurso/acto, sin exigir cambiar la etapa ordinaria.

Cada cálculo conserva regla/versionado, supuesto, autoridad/ámbito, modalidad y
efecto de notificación, dato desencadenante, precisión, zona aplicable, calendario
vigente con fuente y excepciones, resultado y explicación reproducible. Si falta
un dato determinante, queda pendiente de cálculo con motivo visible. Nunca se
resuelve esa falta suponiendo medianoche, diez días para toda apelación o una
lista universal de feriados.

Cambiar notificación, regla o calendario produce una nueva evaluación trazable;
no borra el cálculo anterior. Las alertas tienen estado de entrega y ausencia de
duplicados verificados. Un vencimiento calculado no declara inadmisibilidad ni
firmeza. El contrato de calendario concretará cada supuesto y corpus normativo
antes de implementar términos judiciales.

## Criterios de aceptación del alcance completo

Los resultados parciales del registro y de las asociaciones se describen arriba
y en el informe. No se atribuyen a todas las filas de esta matriz ni completan
los supuestos jurídicos aún pendientes.

| Caso positivo requerido | Caso negativo o límite requerido |
| --- | --- |
| Registrar revocación oral/escrita y apelación anterior al juicio o de sentencia, con datos completos. | Rechazar datos/soporte necesarios ausentes; no exigir llegar a Juicio para registrar todo recurso. |
| Conservar varios recursos y actos sobre una resolución, con cabeza e historia consultables. | No avanzar etapa, certificar admisión ni suspender plazos por el simple alta. |
| Seleccionar una versión histórica y conservar su evidencia tras append o sellado posteriores. | Rechazar documento ajeno, digest discordante, corrupción y cambio durante preparación; no sustituir automáticamente por la versión actual. |
| Edición y archivo con revisión esperada, recuperación explícita del borrador en Qadra. | Conflicto, cierre/revocación concurrentes o fallo de auditoría no dejan escritura parcial ni generan reintento automático. |
| Lectura autorizada y restauración de datos, historia y evidencia con servicios reales. | Client, usuario no asignado y sesión revocada no leen registros, agregados ni soportes. |
| Cálculos reproducibles para corpus de revocación y apelación, con inicios, horas/días y calendario aplicable. | Cubrir 2/3/5/10 días según supuesto, fin de audiencia, inhábiles/excepciones, notificaciones y datos insuficientes; ninguna regla única de suma de días. |
| Recalcular con trazabilidad y entregar alertas del término vigente. | Cambio de regla/calendario, concurrencia y reintentos no borran historia ni duplican alertas. |
| Navegador real escritorio/móvil: lista, detalle, actos, calendario y conflicto. | Respuesta tardía no restaura datos denegados; fallo del recurso no oculta un upload confirmado. |

## Decisiones y límite de aprobación

La implementación autorizada permite resolver separación de entidades, contratos,
transacciones, permisos conservadores, selección de versiones, coordinación Qadra
y pruebas. Puede desarrollarse este alcance sin cambiar el objetivo aprobado.
El corpus inicial y las reglas deben explicitar cobertura y límites; una
entrega parcial no convierte esos límites en una reducción del objetivo.

**Cambiar literalmente el criterio académico requiere autorización explícita
del usuario.** La redacción propuesta permanece aquí para esa revisión; no se
aplica a LaTeX ni al catálogo. Eliminar recursos o audiencias, sustituir cálculo
automático por fechas manuales o declarar suficiente una cobertura menor también
requiere una decisión explícita de alcance, nunca una inferencia de ingeniería.

[ADR-0040](adr/0040-resources-linked-to-historical-resolutions.md) adopta el
registro ligado a resoluciones históricas y
[ADR-0041](adr/0041-exact-resource-activity-associations.md) separa los vínculos
organizativos de las actividades. Ninguno modifica por sí mismo el criterio
aprobado.
**Ni esta nota, ni las etapas implementadas, ni un registro manual de recursos
satisfacen por sí solos OE-2.** El cierre necesita implementación y evidencia de
aceptación, conciliación académica y la validación integral del producto.
