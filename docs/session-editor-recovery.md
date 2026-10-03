# Recuperar trabajo después de autenticar de nuevo

## Comportamiento de la interfaz

Cuando vence el plazo confirmado de sesión, Qadra bloquea nuevas peticiones,
conserva en memoria los borradores de los editores adaptados y muestra el acceso.
Volver a una pestaña oculta exige consultar de nuevo la sesión. Si esa consulta
falla, la interfaz permanece bloqueada y permite reintentar explícitamente.
Mover el foco, recibir una respuesta o ejecutar una lectura no renueva la sesión.

Autenticarse con contraseña y MFA de la misma cuenta permite volver a abrir el
editor. Antes de mostrar el borrador, Qadra comprueba el acceso actual al expediente
y al recurso. No guarda ni reenvía cambios por el hecho de autenticar de nuevo.
Conserva los textos tal como estaban escritos, incluidos espacios y valores que
todavía no pasan la validación, así como los archivos seleccionados.

La recuperación dura mientras esa pestaña siga abierta. Recargarla o cerrarla
pierde los borradores. Cerrar sesión expresamente, cancelar un editor o entrar
con otra cuenta descarta su contenido. Contraseñas, códigos MFA, claves y otros
secretos de autenticación no se guardan como borradores.

## Revisión antes de enviar

Si otra persona cambió la revisión o versión, se conserva la base original junto
con los valores escritos. Hay que consultar y comparar el estado vigente y
decidir explícitamente antes de reemplazarlo. Recuperar acceso de lectura a un
expediente cerrado no habilita sus mutaciones. Una denegación de un recurso
descarta su contexto; una denegación del expediente descarta sus borradores.

Una respuesta perdida no demuestra que la escritura falló. La carga principal y
el alta manual de participante exigen consultar el listado completo y aceptar
el riesgo de duplicación antes de otro envío. No identifican un resultado propio
por coincidencias de nombre o texto. Una versión documental incierta exige
comparar el estado actual incluso cuando el número de versión no cambió.
Un resultado confirmado retira el borrador antes de esperar consultas posteriores.
Una carga anidada conserva su propietario exacto: cancelar o volver a declarar
un candidato retira sólo los descendientes que ya no pertenecen a esa declaración.
Cancelar el editor raíz retira su árbol completo.

La revisión de coincidencias de identidad siempre es explícita. Un motivo escrito
puede conservarse para consulta, pero no equivale a una decisión válida sobre un
candidato nuevo. Si falla la lectura de un soporte durante el reingreso, los
campos siguen presentes y bloqueados; «Volver a consultar el contexto» revalida
permisos y versiones sin iniciar otro envío ni otra revisión automática.

## Alcance comprobado

| Editor | Recuperación aceptada localmente | Límite |
| --- | --- | --- |
| Clasificación documental | Textos, etiquetas y revisión original; autorización y comparación frescas. | No equivale a una restauración de todos los formularios del documento. |
| Alta y edición de expediente | Textos parciales, delito pendiente y base administrativa original. | Conserva declaraciones; no acredita su validez jurídica. |
| Carga documental principal | Archivo, nombre y clasificación sin confirmar; decisión ante incertidumbre. | Las cargas abiertas desde otros editores necesitan contrato propio. |
| Nueva versión documental | Archivo, nombre y versión esperada; comparación explícita. | El historial existente sigue siendo inmutable. |
| Ficha manual de participante | Cuatro textos, estado, revisión y resultado incierto; consulta de sólo lectura si se tipificó. | No sustituye el editor tipificado ni el de identidad representada. |
| Identidad representada | Campos crudos, motivos y base original; candidatos consultados de nuevo y comparación de revisiones. | Una decisión anterior no aprueba un candidato con otra revisión. |
| Soportes anidados de identidad | Búsqueda sin aplicar, versión seleccionada y archivo pendiente, separados por campo y candidato propietario. | La autorización del documento se revalida; otra versión o una denegación no hereda permisos. |
| Participante tipificado | Alta, completar ficha manual y reemplazo con identidad exacta, rol, soportes y envío incierto separados. | La revisión de coincidencias y la declaración se preparan de nuevo; una firma anterior sólo queda como evidencia inerte. |
| Etapa procesal | Adopción y transición originales, fechas y textos crudos, soportes y cargas por propietario; consulta completa de historia ante incertidumbre. | Las etapas no tienen recibo técnico consultable: una coincidencia no confirma un envío ni acredita procedencia jurídica. |
| Audiencias y resultados | Programación, corrección, cancelación, sesiones, continuaciones y retiro; campos crudos, bases originales, referencias históricas y cargas separadas por propietario. | Un envío incierto sólo se confirma por su recibo de revisión exacta; una cabecera o valores parecidos no bastan. |
| Resoluciones y notificaciones | Textos y tiempos crudos, referencias históricas, base original y archivos separados por campo y representación. | El envío incierto requiere el recibo exacto; reabrir el expediente no prepara ni envía automáticamente. |
| Acceso de miembros | Selección de rol y estado, cuenta exacta y revisión decimal original; Owner y destino consultados de nuevo. | Un cambio incierto exige consultar y decidir; un cambio propio confirmado retira el borrador antes de cerrar la sesión. |
| Calendarios jurisdiccionales | Ámbito, fuentes estables, excepciones parciales, motivo y base original; Owner y catálogo o cabecera consultados antes de recuperar. | Su contexto es global; el retiro incierto se concilia por recibo exacto y no por pertenencia a un expediente. |
| Recursos procesales y actos | Texto y motivo crudos, base original, resolución y participantes históricos, soportes y archivos por fila. | Corregir un acto conserva su identidad y revisión separadas de la cabecera; sólo el recibo exacto confirma un envío incierto. |
| Actividades vinculadas | Recurso, acto y actividad históricos, selección parcial, motivo y base originales. | No crea actividades; sólo el recibo exacto confirma el vínculo y un reenvío requiere consultar su ausencia de nuevo. |
| Plazos creados desde recursos | Ambos identificadores nuevos, campos crudos, referencias históricas y sobre incierto conservados tras nueva autorización. | Dos recibos separados no prueban origen conjunto; el servidor debe confirmar explícitamente la operación compuesta. |
| Plazos ordinarios | Alta, corrección, atención y retiro: campos crudos, referencias, base y comando incierto exactos tras nueva autorización. | La preparación pierde aprobación y una coincidencia no confirma el resultado; se consulta el recibo exacto. |
| Solicitud de informes | Fechas parciales, filtros y solicitud incierta exacta; cuenta y selector autorizado consultados antes de recuperar. | Una fila similar o un informe en cola no concilian la respuesta perdida: repetir es una decisión explícita con el mismo identificador. |
| Preferencias de alertas | Horas y canales crudos, revisión original y comando incierto; cuenta y preferencias consultadas antes de mostrar campos. | Sólo el recibo exacto confirma; una denegación global de la bandeja descarta incluso capturas sin abrir. |

El alcance de la tabla procede de focales con respuestas HTTP controladas. Un
focal de etapas aceptó ocho escenarios en 28,4 segundos, incluidos reingreso,
soportes independientes, incertidumbre y descarte posterior a una confirmación.
Otro focal aceptó diez escenarios de audiencias y resultados en 33,8 segundos,
con autorización fresca, referencias originales, comparación explícita, archivos
propietarios y recibos exactos ante incertidumbre. Ninguno de estos focales
constituye aceptación de CI ni una campaña de esas familias con servicios reales.
Ocho escenarios de resoluciones y notificaciones aprobaron en 29,0 segundos; incluyen recuperación tras fallo transitorio de referencia, cierre y reapertura del expediente, aislamiento de archivos y conciliación por recibo. Estos resultados son locales y no afirman integración ni despliegue. Un
recorrido adicional con servicios reales aceptó dos vencimientos y nueva MFA,
conservando campos de un alta de expediente y un archivo de carga principal.
Ese recorrido no valida contra servicios reales todos los editores de la tabla.
La recuperación de acceso de miembros aprobó seis escenarios en 18,3 segundos;
las once regresiones existentes aprobaron en 25,1 segundos. Conserva revisiones
decimales superiores al rango entero seguro de JavaScript y limpia el borrador
antes de invalidar una sesión cuyo propio rol cambia. Los inventarios y
resultados integrados se registran separadamente en
[el informe de verificación](verification-report.md).

Seis escenarios nuevos de calendarios aprobaron en 19,1 segundos con HTTP
controlado. Comprueban autorización administrativa fresca, campos parciales,
fuentes y filas estables, comparación explícita de revisión, incertidumbre,
recibo ajeno rechazado y descarte antes del refresco confirmado. No validan la
interpretación jurídica de las fuentes ni una activación operativa.

Ocho escenarios de recursos procesales aprobaron en 27,8 segundos con HTTP
controlado. Incluyen conservación del acto histórico al cambiar la cabecera,
pérdida de aprobación de una preparación anterior, lectura de soportes, archivos
por fila y cierre o reapertura del expediente. La recuperación no determina
procedencia jurídica ni activa plazos. Las doce regresiones existentes de
campos, permisos, conflicto y conciliación aprobaron en 26,6 segundos, con
recorridos de escritorio y móvil.

La recuperación de actividades vinculadas aprobó ocho escenarios nuevos en dos
focales: dos de contexto y seis de selección o resultado (21,5 segundos para
estos últimos). Doce recorridos existentes aprobaron en 21,8 y 11,2 segundos.
Conserva las referencias históricas, exige nueva consulta antes del reenvío
exacto y elimina el borrador confirmado antes de refrescar el listado.

Ocho casos nuevos de plazos desde recursos aprobaron en 34,9 segundos y trece
regresiones existentes en 34,0 segundos, con un worker y HTTP controlado. Una
preparación recuperada pierde aprobación. Cada vencimiento exige otra consulta
antes de decidir un reenvío; un expediente cerrado permite confirmar el origen
conjunto de registros existentes, pero no repetir una creación ausente.

Ocho casos nuevos del editor ordinario de plazos aprobaron en 27,0 segundos y
siete regresiones existentes en 19,7 segundos, con HTTP controlado. Incluyen
atención con hora parcial, revisión concurrente, cierre y reapertura, recibo
ajeno y limpieza antes del refresco. No añaden un reenvío automático ni atribuyen
al autor una revisión por coincidencia de valores.

Seis casos nuevos de solicitudes de informes aprobaron en 16,5 segundos con
HTTP controlado. Conservan el filtro de litigante si deja de estar autorizado,
bloquean la solicitud y permiten consultar de nuevo sin sustituirlo por todos.
La confirmación retira la captura; sólo una edición posterior crea otro borrador.
No incluyen envío de correo ni una nueva aceptación de generación PDF/CSV real.

Nueve escenarios nuevos de preferencias aprobaron en focales con HTTP controlado:
cinco de contexto y captura, tres de resultado y uno de denegación de la bandeja.
Conservan horas incompletas y canales; cada vencimiento retira una aprobación de
reenvío anterior. Ni valores iguales ni un recibo ajeno confirman el guardado.
La confirmación elimina la captura antes de refrescar la bandeja. Se conservaron
los recorridos existentes de conflicto y paginación.

La duración operativa del límite de inactividad sigue sin aprobarse. El backend
mantiene por defecto su límite absoluto de veinticuatro horas. La configuración
de doce segundos del [ensayo reproducible](../web/README.md#reingreso-con-vencimiento-real)
pertenece exclusivamente a sus servicios desechables.

## Confirmaciones sin campos editables

Cambiar estado administrativo o de participante y confirmar un sellado son
intenciones explícitas, no borradores de texto. Si se pierde su respuesta o el
servidor devuelve un error 5xx, consulta el estado actual antes de decidir otra
operación. Cancelar y reabrir la confirmación no elimina esa obligación; una
consulta no reenvía el comando ni demuestra por sí sola qué envío produjo el
estado observado. En sellado se consulta la versión exacta, se cierra la intención
anterior y, si sigue pendiente, hace falta iniciar otra confirmación.

En el estado del expediente, esa barrera también se conserva al pasar a otra
sección y volver al resumen. La navegación no consulta ni reenvía el comando:
la confirmación permanece bloqueada hasta completar la consulta explícita.
El estado se limita al expediente abierto y se descarta con su sesión.

Trece casos nuevos y diecinueve regresiones relacionadas aceptaron ese
comportamiento localmente con HTTP controlado. Otros cuatro escenarios aceptaron
vencimiento durante cambio administrativo, cambio de participante, retiro de
asignación y sellado: la nueva sesión consulta el estado vigente y no repite el
comando. La confirmación anterior se descarta y su respuesta tardía no modifica
la intención nueva. Los filtros de Asignaciones vuelven a su estado inicial;
no contienen texto de una mutación que deba recuperarse.

## Lectura de avisos

Marcar una alerta o un aviso de informe como leído no contiene campos que
conservar. Tras autenticar de nuevo se consulta la lista autorizada; el informe
requiere abrir su detalle exacto antes de otro acuse explícito. La interfaz no
repite el acuse anterior, ni infiere qué petición produjo el estado consultado.
Una respuesta vieja no termina otra intención ni recupera un detalle denegado.

Cuatro escenarios aprobaron en 13.8 s con HTTP controlado y un worker, sin
cambiar producto: resultado aplicado y no aplicado, misma cuenta o cambio de
cuenta, nueva MFA, bearer nuevo, consulta exacta y denegación. No sustituyen
aceptación de esas rutas con servicios reales ni activan inactividad operativa.

## Alcance que conserva aceptación pendiente

Una consulta o un selector que no confirma escrituras no necesita conservar sus
resultados privados después de autenticar de nuevo. Sus filtros o selecciones
sólo pueden formar parte del borrador del editor propietario mediante una
proyección explícita. El alta de integrantes conserva aparte su pendiente de enrolamiento:
contraseña inicial, secreto MFA y códigos de recuperación no deben entrar al
registro de borradores. La aceptación de los editores no demuestra recuperación
de esos secretos ni de todos los controles que confirman escrituras.

El contrato técnico del registro y las obligaciones de cada adaptador están en
[ADR-0065](adr/0065-session-reentry-and-memory-drafts.md). La existencia del registro
no hace recuperable un editor que todavía no esté adaptado y aceptado.
