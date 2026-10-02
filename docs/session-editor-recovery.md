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
Los inventarios y resultados integrados se registran separadamente en
[el informe de verificación](verification-report.md).

La duración operativa del límite de inactividad sigue sin aprobarse. El backend
mantiene por defecto su límite absoluto de veinticuatro horas. La configuración
de doce segundos del [ensayo reproducible](../web/README.md#reingreso-con-vencimiento-real)
pertenece exclusivamente a sus servicios desechables.

## Editores que todavía requieren adaptación

El vencimiento cierra también los editores siguientes, pero su contenido no tiene
una aceptación de recuperación. No debe habilitarse la inactividad operativa
asumiendo que todos los formularios conservan sus cambios.

| Familia | Componentes | Contexto que debe consultarse de nuevo |
| --- | --- | --- |
| Recursos y actividades | `ResourceEditor.svelte`, `ResourceActivityEditor.svelte`, `ResourceDeadlineEditor.svelte` | Recurso propietario, actividad o plazo elegible, asociaciones y resultado incierto. |
| Plazos | `DeadlineEditor.svelte` | Plazo, fuentes de cómputo y versión consultada. |
| Calendarios | `CalendarEditor.svelte` | Permiso administrativo y revisión del calendario. |
| Acceso de miembros | `MemberAccessEditor.svelte` | Cuenta exacta, revisión, rol vigente y restricciones de administración. |

Una consulta o un selector que no confirma escrituras no necesita conservar sus
resultados privados después de autenticar de nuevo. Sus filtros o selecciones
sólo pueden formar parte del borrador del editor propietario mediante una
proyección explícita. El cierre completo requiere revisar también los botones
y formularios de confirmación fuera de los componentes llamados `Editor`.

El contrato técnico del registro y las obligaciones de cada adaptador están en
[ADR-0065](adr/0065-session-reentry-and-memory-drafts.md). La existencia del registro
no hace recuperable un editor que todavía no esté adaptado y aceptado.
