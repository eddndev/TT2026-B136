# Qadra: manual de uso

**Tu despacho, en orden.** Esta guía describe cómo trabajar con el prototipo
Qadra y reconocer el resultado de cada operación. Los menús disponibles dependen
de tu cuenta y de los expedientes asignados. **Informes** y la consulta de
**Auditoría** están incluidos en la instalación privada. La versión desplegada
`v0.1.1` todavía no incorpora las entregas posteriores de recuperación de contraseña
y reingreso. Los apartados que las describen requieren una versión que las incluya
y, cuando corresponda, su habilitación por el administrador; ver
[estado de entregas e instalación](product-completion.md).

Este manual se basa en las pantallas y contratos del producto. No es un informe
de evaluación de usabilidad con personas ni acredita por sí mismo la puesta en
servicio del sistema. El administrador proporciona la dirección de acceso;
la preparación de esa instalación corresponde al administrador del servicio.

## 1. Entrar y conservar el acceso

### Primera configuración del segundo factor

Al crear una cuenta, Qadra muestra **Configura el segundo factor**. Registra la
**Clave de configuración** en una aplicación de autenticación compatible con
códigos temporales. También puedes usar la URI de configuración manual mostrada
en esa pantalla. La clave y los códigos de recuperación se muestran una sola vez.

Guarda los códigos de recuperación en un lugar privado. Después marca
**Ya guardé la clave y los códigos** y selecciona **Finalizar**. No incluyas
contraseñas, claves, códigos temporales ni códigos de recuperación en capturas
para soporte, presentaciones o expedientes.

### Inicio de sesión habitual

1. En **Accede a tu despacho.**, escribe el correo y la contraseña de tu cuenta.
2. Cuando aparezca **Un paso más.**, abre tu aplicación de autenticación e
   introduce el **Código de 6 dígitos** vigente.
3. Selecciona **Verificar y entrar**. El acceso correcto muestra **Tu mesa de
   trabajo**, el correo y el rol de la sesión.

El OTP es el código temporal generado por la aplicación que configuraste durante
el alta; no es una contraseña fija ni un código enviado automáticamente por correo.
Si vence la solicitud o se rechaza el código, vuelve a iniciar sesión. Si el reloj
del dispositivo está desajustado, corrígelo antes de intentarlo nuevamente.

Si no dispones de la aplicación, selecciona **Usar código de recuperación** e
introduce uno que no hayas utilizado. Cada código permite un solo uso. Si perdiste
ambos medios, contacta al administrador: cambiar la contraseña no recupera el
segundo factor perdido ni genera nuevos códigos. Si sólo perdiste la contraseña,
usa el procedimiento siguiente cuando esté habilitado en tu instalación.

Usa **Cerrar sesión** al terminar. Cerrar sesión conserva los expedientes y sus
archivos. No dependas de un cierre por inactividad para proteger una sesión abierta.

### Vincular mi certificado público

Esta función pertenece a la ampliación local de identidad y todavía no está en
la release privada `v0.1.1`. Su aceptación e integración se registran por separado
en [el estado del producto](product-completion.md).

El Owner dispone de **Mi certificado** para vincular un certificado público a
su propia cuenta mediante una firma externa. Primero consulta si ya existe un
vínculo sin retirar. Para registrar otro, prepara la declaración, descarga sus
150 bytes, fírmala en el equipo que custodia la clave y selecciona únicamente la
firma separada de 384 bytes. El [recorrido y ejemplo OpenSSL](owner-certificate-interface.md)
explican cada paso. Nunca adjuntes la clave privada, un archivo PFX o una contraseña.

**Consultar recibo** y **Descargar recibo** conservan la evidencia del registro.
**Retirar vínculo** exige una confirmación expresa y conserva el historial; no
revoca el certificado en la autoridad emisora. Si un envío queda sin respuesta,
comprueba su recibo exacto antes de decidir otro intento. Un vínculo sin retirar
no demuestra vigencia actual, no permite iniciar sesión por certificado y no
firma documentos del expediente.

### Si olvidaste la contraseña

La recuperación está implementada e integrada en una versión posterior a
`v0.1.1`; la instalación privada actual necesita actualizarse y configurarse
antes de habilitarla. Confirma su disponibilidad con el administrador:
ver el enlace del formulario no demuestra que el servicio de correo esté activo.

1. En la pantalla de acceso, selecciona **Olvidé mi contraseña**, escribe tu correo
   y pulsa **Solicitar enlace**. La respuesta es neutra: no confirma que exista una
   cuenta ni que se haya enviado o entregado un mensaje.
2. Abre el enlace privado recibido. En **Elige una nueva contraseña.**, completa
   **Nueva contraseña** y **Repite la nueva contraseña** con el mismo valor. La
   interfaz admite de 12 a 1024 bytes y señala valores fuera del límite.
3. Pulsa **Cambiar contraseña** una sola vez. Ante **Contraseña actualizada.**,
   vuelve al inicio de sesión y usa la nueva contraseña junto con tu segundo
   factor habitual. El enlace no abre una sesión ni sustituye el MFA.
4. Si el enlace ya no puede usarse, solicita otro desde el acceso. Si Qadra no
   pudo confirmar el cambio, prueba iniciar sesión con la contraseña nueva o
   solicita otro enlace; no des por fallido el cambio ni repitas el mismo envío.

Mantén privados el enlace y las contraseñas; no los incluyas en capturas de
soporte. Cambiar la contraseña invalida el acceso anterior. El procedimiento no
reactiva una cuenta deshabilitada ni modifica su rol o sus asignaciones.

### Volver después de que venza la sesión

En una versión que incluya reingreso, el vencimiento bloquea las peticiones y
vuelve a mostrar el acceso. Autentícate con la **misma cuenta** y completa MFA;
abre de nuevo el expediente y el editor. Los editores aceptados consultan tus
permisos y los registros vigentes antes de mostrar lo que habías escrito. Esa
consulta puede fallar o revelar un cambio de acceso; recuperar la sesión no
conserva permisos anteriores.

Los borradores admitidos se mantienen sólo en memoria de esa pestaña. **No la
recargues ni la cierres si quieres recuperarlos.** Cerrar sesión expresamente,
cancelar el editor o entrar con otra cuenta descarta la captura. Las contraseñas,
claves y códigos MFA nunca forman parte de ella. La lista de editores aceptados
y sus límites está en [recuperación de editores](session-editor-recovery.md);
no supongas que todos los formularios ofrecen la misma recuperación.

Si cambió la revisión, consulta y compara los valores actuales antes de decidir.
Una aprobación anterior no se conserva. Si el envío perdió su respuesta, usa la
comprobación que ofrece ese editor: recuperar el acceso no reenvía cambios ni
confirma una operación por encontrar datos parecidos. Los archivos y cambios
que el servidor ya confirmó permanecen guardados aunque venza la sesión.
La duración de inactividad depende de la configuración explícita del servicio;
la instalación privada conserva el límite absoluto actual y no tiene activada
la política de inactividad pendiente de selección.

## 2. Qué permite cada cuenta

| Cuenta en Qadra | Alcance y tareas principales |
| --- | --- |
| **Administrador** (Owner) | Todos los expedientes; gestión de cuentas y asignaciones, registro procesal, documentos y sellado. Accede a Equipo, Auditoría, Incidentes de integridad y Calendarios jurisdiccionales. |
| **Litigante** | Expedientes con asignación vigente; registro procesal, documentos, clasificación y sellado. Consulta el tablero de sus expedientes. |
| **Asistente legal** | Expedientes asignados; carga, clasificación, consulta, verificación y descarga documental. Consulta participantes, actividades y agenda; no sella ni gestiona los registros procesales. |
| **Cliente** | Consulta básica de expedientes asignados. El acceso documental y los módulos de participantes, agenda, alertas e informes permanecen restringidos. |

Las asignaciones se aplican a cuentas de acceso. Una ficha de participante
procesal no crea una cuenta ni concede permisos. Tampoco la existencia de una
alerta concede acceso adicional a un expediente.

### Administrar el equipo y las asignaciones

El administrador abre **Equipo** para crear una cuenta o consultar el directorio.
Al crearla, conserva y entrega de forma privada la configuración inicial del
segundo factor. El alta es directa: **no hay invitaciones por correo ni enlaces de
aceptación implementados**.

El directorio permite filtrar por correo, rol y estado. Revisa la cuenta antes de
confirmar cambios de acceso. Cambiar rol o estado invalida el acceso anterior;
reactivar exige un nuevo inicio de sesión y conserva las asignaciones existentes.
No se permite dejar al despacho sin un administrador activo.

Para asignar cuentas, abre un expediente y entra en **Asignaciones**. Consulta
asignados o disponibles, selecciona la cuenta y confirma asignación o retiro.
Una cuenta inactiva puede conservar su asignación, pero no puede usarla para entrar.
Si el resultado del envío es incierto, consulta las asignaciones actuales antes
de decidir otra acción. Véase [gestión de miembros](members-api.md).

## 3. Orientarse desde Inicio

**Inicio** reúne accesos a expedientes, documentos pendientes de sello, documentos
sellados y alertas según tu rol. El expediente seleccionado se conserva como
contexto de navegación; comprueba siempre su título y referencia antes de escribir.

Administrador y Litigante disponen de **Indicadores operativos**. Usa **Actualizar
indicadores** para consultar una nueva observación. La pantalla distingue
expedientes activos, contratos pendientes de sello y plazos vencidos, próximos o
por revisar, además de la carga por litigante.

- El administrador observa todo el despacho; el litigante, sus expedientes asignados.
- Los plazos de menos de 48 horas están incluidos en los de menos de siete días;
  no sumes ambas cifras como grupos independientes.
- **Por revisar** indica que falta un vencimiento operativo vigente; no significa
  que el plazo esté resuelto ni que deje de requerir atención.
- La carga por litigante cuenta asignaciones y puede incluir el mismo expediente
  en más de una persona. Los indicadores no califican resultados jurídicos.

La [descripción del tablero](dashboard-api.md) precisa su alcance.

## 4. Crear, completar y cerrar un expediente

1. Abre **Expedientes** y selecciona **Nuevo expediente penal** si tu rol lo permite.
2. Registra título, referencia interna, NUC, carpeta judicial, autoridades y
   descripciones de delito que correspondan a la ficha.
3. Revisa los datos y confirma. Abre **Resumen** para comprobar la ficha guardada.
4. Desde el expediente entra en Documentos, Participantes, Etapas, Audiencias,
   Resoluciones, Recursos o Plazos según la tarea.

Una ficha anterior pendiente se puede completar; esto no reconstruye etapas ni
fechas pasadas. El registro inicial de Investigación de una nueva ficha completa
es un registro del sistema, no una acreditación de actuación judicial.

Administrador y Litigante asignado pueden cerrar administrativamente o reactivar
un expediente desde **Resumen**. El cierre conserva consulta e historial
permitidos, verificación y descargas; impide nuevas modificaciones de ficha,
documentos y registros procesales. No cambia la etapa ni reactiva participantes
archivados. Consulta el [alcance administrativo](case-administration-api.md).

## 5. Trabajar con documentos

### Cargar y clasificar

Abre primero el expediente correcto y su sección **Documentos**. Selecciona o
arrastra el archivo, revisa el nombre sugerido y, si corresponde, completa tipo,
clasificación y etiquetas. Cada etiqueta se agrega por separado.

Se admiten archivos de hasta **16 MiB**: PDF, DOCX, TXT, JPEG y PNG sin animación,
MP3, WAV PCM y MP4 H.264/AAC. La admisión comprueba el contenido; cambiar la
extensión no convierte un archivo a otro formato. Un archivo reconocible puede
ser rechazado por contenido inválido, subtipo no admitido o límites de validación.
Consulta el [catálogo de admisión](document-upload-admission-api.md) si necesitas
las variantes concretas.

Qadra puede sugerir un nombre compatible sin espacios ni acentos para el archivo.
Revisarlo no modifica los bytes originales. Tipo, clasificación y etiquetas se
mantienen separados del contenido; editar esos datos no sustituye el archivo ni
sus evidencias. Los filtros por clasificación buscan valores completos y
distinguen mayúsculas y acentos.

Un rechazo conserva el archivo y el borrador en la pantalla. Lee la causa,
corrige o selecciona otro archivo y confirma de nuevo. No supongas que se guardó
si no aparece la confirmación. Los soportes de actos procesales mantienen su
política propia de **PDF o DOCX**; el catálogo general no amplía esos soportes.

### Versiones y contenido original

Usa **Agregar versión** para conservar otro archivo bajo la misma identidad del
documento. Las versiones anteriores permanecen disponibles. El historial muestra
primero las más nuevas; selecciona la versión exacta antes de sellar, verificar
o descargar. La clasificación actual pertenece al documento y tiene un historial
separado del historial de archivos.

La descarga de contenido puede recuperar una versión pendiente o sellada tras
comprobar su integridad. No agrega un sello. Si aparece un aviso de integridad,
conserva el mensaje y comunica al administrador el expediente, documento y versión;
no sustituyas el archivo para ocultar el incidente. El administrador dispone de
**Incidentes de integridad**. Véase [contenido y avisos](document-content-api.md).

### Sellar, verificar y guardar evidencia

1. Administrador o Litigante selecciona la versión correcta y confirma su sellado.
2. Usa **Verificar integridad** para obtener un resultado actualizado. Abrir la
   ficha por sí solo no ejecuta esa comprobación.
3. Revisa el resultado de integridad, firma, certificado y sello de tiempo.
   Un fallo requiere atención; no presentes esa verificación como satisfactoria.
4. Descarga la evidencia ZIP de la versión seleccionada y comprueba que el
   navegador terminó de guardarla. Conserva el paquete completo.

El sellado emplea la identidad técnica del servicio y un sello de tiempo local;
no demuestra una firma personal del usuario que pulsó el botón. Tampoco equivale
a una constancia NOM-151 emitida por un PSC autorizado. Las declaraciones externas
usadas en determinadas fichas de participantes son un flujo distinto.

## 6. Participantes, etapas y actuaciones declaradas

En **Participantes**, Administrador y Litigante asignado pueden registrar,
completar, editar, archivar y reactivar fichas. El Asistente legal asignado puede
consultarlas. Selecciona la identidad y el perfil adecuados, revisa coincidencias
y confirma los soportes exactos. Personas con nombres iguales no se fusionan
automáticamente. Archivar organiza el directorio; no cambia su situación jurídica.

Cuando una ficha exige certificado y declaración firmada, descarga la declaración
preparada, fírmala externamente y carga la firma separada. Qadra no necesita recibir
tu clave privada. Cambiar la declaración o el certificado exige prepararla otra
vez. La comprobación de demostración usa una autoridad interna; no acredita por sí
misma identidad civil, profesión ni FIREL oficial. Consulta
[participantes tipificados](typed-participants-api.md).

En **Etapas**, declara la etapa conocida o el avance permitido, fecha, motivo y
soporte PDF/DOCX exacto. Diferencia fecha sin hora de fecha con hora y desfase UTC.
La recepción por el tribunal y la emisión de un auto son datos distintos. Revisar
el historial permite ver qué se declaró y cuándo se registró. Una carga documental
confirmada puede permanecer guardada aunque el acto posterior sea rechazado.

En **Audiencias**, revisa expediente, etapa, tipo, fecha, hora, desfase UTC,
modalidad, sede o conexión y revisiones de los participantes. Corregir, reprogramar
y cancelar requiere revisar el estado actual y conservar el motivo. Programar
una audiencia no declara que se celebró. Los resultados y resoluciones se registran
por separado, con sus soportes y antecedentes; no se deducen de la programación.
Los catálogos disponibles delimitan lo que puede registrarse.

Consulta [etapas](case-stages-api.md), [audiencias](hearings-api.md),
[resultados de audiencia](hearing-results-api.md) y [hechos y resoluciones
procesales](procedural-facts-api.md).

## 7. Recursos y sus actividades

En **Recursos**, Administrador y Litigante asignado registran el recurso con su
resolución de origen, personas y soportes correspondientes. Revisa la revisión
histórica seleccionada; una corrección posterior de la fuente no sustituye lo que
quedó capturado. Los actos del recurso son declaraciones expresas y conservan su
historial. Su registro no prueba por sí solo admisión ni efectos jurídicos.

Las actividades permiten asociar una audiencia o plazo existente del mismo
expediente. Revisa la actividad exacta y el estado de su asociación; desvincular
conserva la historia. También se puede preparar un plazo nuevo desde el recurso:
revisa sus insumos, cálculo y vínculo antes de confirmar. Esa confirmación guarda
el plazo y su asociación juntos; preparar no equivale a guardar.

La navegación de recursos relacionados desde una actividad, incluida la abierta
por una alerta, muestra asociaciones actuales y sus capturas. No demuestra que
el recurso haya causado la alerta. Distingue siempre revisión capturada y registro
actual, y usa la acción de regreso para volver al contexto abierto. Esta navegación
está incluida en `v0.1.0`.

No hay creación de audiencias propias del recurso ni activación jurídica automática
de plazos. Referencias: [recursos](procedural-resources-api.md), [asociaciones](resource-activities-api.md),
[plazos desde recursos](resource-deadlines-api.md) y
[navegación desde actividades](activity-resource-links-api.md).

## 8. Plazos, agenda y alertas

### Preparar un plazo

Administrador y Litigante asignado seleccionan perfil, fuente, calendario y
responsable. Declara expresamente aplicabilidad, condiciones y cantidad concedida
cuando se pida. Para las dependencias, distingue conservar una revisión de seguir
cambios. Un máximo permitido no demuestra que esa cantidad haya sido concedida.

**Prepara y revisa antes de confirmar.** Comprueba el cálculo, sus pasos y los
bloqueos. Un registro bloqueado conserva lo pendiente sin inventar un vencimiento.
Si cambian fuentes o perfiles seguidos, revisa los insumos y confirma la corrección
necesaria. La historia conserva cálculos anteriores, separados de la fecha vigente.
Declarar atención o retirar un plazo no acredita por sí solo una presentación válida.

El administrador mantiene los calendarios jurisdiccionales; antes de usar un
cómputo, confirma que calendario, fuente, perfil y datos declarados correspondan
al caso. Qadra organiza esos datos y cálculos, sin sustituir esa revisión profesional.
Véanse [plazos](deadlines-api.md) y [seguimiento](deadline-tracking-api.md).

### Consultar Agenda

Abre **Agenda**, elige día, semana, mes o rango personalizado, desfase de consulta
y filtros. Pulsa **Consultar Agenda**. Revisa las fechas con su desfase; no confundas
la hora original declarada con la representación del intervalo elegido.

La agenda reúne audiencias y vencimientos operativos autorizados. Si indica que
la consulta es parcial, carga más actividades antes de tratarla como completa.
Un plazo bloqueado, retirado o pendiente de revisión puede conservar historia sin
aparecer como vencimiento vigente. Abre la actividad y consulta su registro actual
antes de modificarla. Véase [agenda](agenda-api.md).

### Usar Mis alertas

Administrador, Litigante y Asistente legal consultan sus propias alertas y
preferencias. Las audiencias se dirigen a cuentas activas asignadas al expediente;
el administrador no recibe todas automáticamente. Los plazos se dirigen al
responsable actual autorizado.

Consulta los filtros y usa **Actualizar alertas** cuando necesites una nueva
lectura. Abrir una alerta o su actividad no la marca como leída: usa **Marcar como
leída** cuando corresponda. La alerta conserva el origen observado; compara con
el estado actual antes de actuar.

En **Preferencias de alertas**, revisa anticipaciones y canales. Si el servidor
indica que el correo está deshabilitado, guardar la preferencia no enviará mensajes.
Cuando esté habilitado, el correo contiene un aviso genérico y acceso a Qadra,
sin datos del expediente. Consultar una audiencia o activar una preferencia no
confirma que se haya enviado un aviso. Véanse [alertas](alerts-api.md).

## 9. Informes

Administrador y Litigante pueden abrir **Informes** para solicitar una captura del
estado administrativo observado. Cada persona consulta únicamente sus propias
solicitudes. El administrador incluye expedientes del despacho; el litigante,
los que tiene asignados y puede consultar.

1. En **Solicitar informe**, elige **Creación desde (UTC)** y **Creación hasta
   (excluida, UTC)**, estado administrativo y, opcionalmente, litigante asignado.
2. El inicio se incluye y el final se excluye, ambos a las 00:00 UTC; el intervalo
   máximo es 366 días. Para incluir todo septiembre, usa 1 de septiembre a
   1 de octubre. El selector considera también compañeros de expedientes cerrados;
   usa **Cargar más litigantes** cuando haya otra página.
3. Selecciona **Generar informe** y conserva su identidad. Puedes salir de la
   pantalla y volver; usa **Actualizar informe** para consultar el progreso.
4. Cuando esté listo, descarga PDF o CSV. Ambos contienen la misma captura y no
   cambian porque el expediente se modifique después.
5. Usa **Marcar aviso como leído** para acusar el aviso interno. Descargar un
   archivo no realiza ese acuse.

En versiones que incluyen **Duración observada**, el detalle disponible o fallido
muestra el tiempo transcurrido desde la solicitud hasta el resultado registrado,
incluyendo espera y reintentos. Leer el aviso no aumenta ese tiempo. No aparece
en trabajos pendientes ni predice cuándo terminarán; esta ampliación aún no forma
parte de `v0.1.0`.

El periodo filtra **cuándo se crearon los expedientes**. El contenido muestra su
estado observado al preparar la captura; no reconstruye el estado al final del
periodo ni mide la actividad o el éxito jurídico. No incluye una firma documental
ni una constancia de PSC. Una pérdida posterior de permisos puede impedir la
consulta o descarga completa.

Si una respuesta es incierta, **Reintentar solicitud** conserva la misma solicitud
y filtros. Si se excede la capacidad, reduce el conjunto y solicita otro informe;
no se recortan filas para aparentar un resultado completo. Consulta
[el alcance de informes](case-reports-api.md).

## 10. Auditoría: consulta de actividad

**Sólo el Administrador** puede abrir **Auditoría** para consultar la actividad
registrada. La acción **Verificar cadena** es independiente de esa consulta.

### Consultar eventos registrados

1. En **Actividad registrada**, elige **Desde (UTC, incluido)** y **Hasta (UTC,
   excluido)**. Las fechas empiezan a las 00:00 UTC y el periodo debe ser positivo,
   de hasta 366 días. Para consultar un día completo, usa ese día como inicio y
   el siguiente como final.
2. Opcionalmente completa **Actor registrado**, **Operación registrada** y
   **Recurso registrado**. Copia el texto exacto de un evento conocido: mayúsculas
   y espacios cuentan. No son búsquedas por parte del nombre. Dejar un campo
   vacío omite ese filtro.
3. Elige 20, 50 o 100 eventos por página y pulsa **Consultar actividad**.
4. Revisa la fecha exacta, actor, operación, recurso y secuencia que quedaron
   registrados. Si no hay coincidencias, amplía el periodo o revisa los filtros;
   una página vacía no demuestra que la bitácora completa esté vacía.
5. Usa **Cargar siguiente página** mientras esté disponible. Cada página
   reemplaza la anterior y conserva el mismo corte de actividad; los eventos
   añadidos después no se incorporan a mitad del recorrido.
6. Usa **Actualizar actividad** para iniciar un corte nuevo desde la primera
   página. Cambiar cualquier filtro también borra el resultado anterior; pulsa
   **Consultar actividad** para obtener otro resultado.

El actor y el recurso son textos históricos. No se deduce una identidad personal
comprobada ni una dirección IP a partir de ellos. La pantalla conserva la fecha
exacta del registro; no la presenta como una fecha de verificación criptográfica.
Cerrar sesión elimina los resultados privados de la pantalla. Si pierdes permiso,
solicita al administrador que revise tu cuenta; no compartas sesiones para abrirlos.

Si aparece un límite de capacidad, reduce los eventos por página o acota fechas
y filtros. El sistema rechaza esa respuesta completa: no recorta el contenido de
un evento para aparentar que devolvió toda su información. Si incluso la selección
mínima falla, comunica el mensaje a soporte sin adjuntar datos privados innecesarios.

### Comprobar la cadena completa

Pulsa **Verificar cadena** y revisa **Cadena íntegra** o **Alteración detectada**.
Ante una alteración, conserva el resultado y comunica el índice indicado. Consultar
una página de actividad no sustituye esta comprobación ni verifica por sí sola los
documentos mencionados. La cadena local tampoco acredita anclaje externo de su
cabecera. Consulta [el alcance de auditoría](audit-events-api.md).

## 11. Resolver incidencias sin duplicar operaciones

| Situación | Qué hacer |
| --- | --- |
| Sesión vencida o acceso invalidado | Entra de nuevo con la misma cuenta y MFA; en versiones con reingreso, abre el editor para consultar su contexto y recuperar el borrador admitido. Si desapareció un permiso o expediente, pide al administrador que revise rol y asignación. |
| Otra persona modificó el registro | Conserva el borrador, consulta el estado y la historia actuales, compara y confirma de nuevo cuando corresponda. No des por aplicado el envío rechazado. |
| Se perdió la respuesta de una escritura | Usa la consulta o comprobación ofrecida para ese envío. No repitas automáticamente altas, sellados o confirmaciones. Si sigue incierto, conserva los datos y pide ayuda. |
| Archivo rechazado | Lee el motivo, verifica formato y tamaño, conserva el original y vuelve a enviar sólo tras decidir la corrección. Cambiar el nombre no repara el contenido. |
| Falló la integridad o la evidencia | Conserva el aviso y comunica expediente, documento, versión y momento al administrador. No afirmes que la verificación aprobó. |
| Servicio temporalmente no disponible | Conserva el borrador y vuelve a consultar. Un mensaje de error no confirma que una escritura anterior haya fallado o tenido éxito. |
| Descarga iniciada pero sin archivo visible | Comprueba la lista de descargas y su finalización en el navegador antes de darla por guardada. |
| Lista parcial o botón para cargar más | Solicita la continuación antes de interpretar esa pantalla como el conjunto completo. |

Para solicitar soporte, comunica la operación, el mensaje mostrado, el momento y
las identidades del expediente o registro necesarias. Mantén fuera del reporte
contraseñas, códigos y claves privadas; evita adjuntar contenido sensible si no es
necesario para atender el problema.

## 12. Alcance del manual y evaluación pendiente

Este documento permite preparar recorridos de acceso, asignación, carga,
versionado, sellado, verificación, registro procesal y consulta. Las funciones
pendientes —invitaciones, recuperación del enrolamiento, política documental de
Cliente y las entregas señaladas expresamente— no deben incluirse como tareas
completadas en una demostración. La recuperación de contraseña implementada y
los borradores requieren comprobar la versión y la habilitación concretas antes
de demostrarlos; no se atribuyen a la release privada anterior.

La evaluación con usuarios dispone de un [kit y protocolo de observación](usability/README.md):
participantes y consentimiento, tareas, criterios de éxito, observaciones y
resultados reales por registrar. El material está preparado; la evaluación
humana sigue pendiente.
Esta guía no registra participantes, tiempos, satisfacción ni conclusiones de
una evaluación que todavía no se haya realizado. El estado funcional y su
verificación se consultan en [cierre del producto](product-completion.md) y
[el informe de verificación](verification-report.md).
