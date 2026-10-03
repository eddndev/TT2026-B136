# Catálogo para moderación

Las tarjetas que se entregan a participantes están en [participant.md](participant.md).
Esta hoja contiene las condiciones y respuestas observables. Las referencias de
trazabilidad apuntan a archivos versionados y no cambian los objetivos aprobados.
Los límites de tiempo son organizativos: no definen éxito del requisito funcional.

## Tareas base

| Tarea | Roles y preparación | Resultado observable | Límite sugerido |
| --- | --- | --- | --- |
| T01 · Acceder | Administrador, Litigante o Asistente legal; cuenta enrolada y credenciales entregadas privadamente; sesión cerrada. Anotar TOTP o recuperación. | Inicia sesión con contraseña y segundo factor; reconoce correo de práctica y rol; llega a Inicio. No basta validar sólo la contraseña. | 5 min |
| T02 · Localizar expediente | Mismos roles; expediente asignado `Práctica Qadra Sxxx`, referencia `USO-Sxxx`, más un distractor ficticio visible de título diferente. | Abre el expediente correcto y distingue su título, referencia y estado. Si filtra, interpreta que una lista filtrada no equivale al total del despacho. | 4 min |
| T03 · Cargar y clasificar | Mismos roles; expediente activo; archivo `practica-v1.txt`; no existe todavía en ese expediente. | Carga confirmada, nombre correcto, clasificación `Práctica de usabilidad` y etiqueta `practica`. Comprueba la ficha guardada; no marca éxito por ver un archivo seleccionado. Puede clasificar en la carga o después. | 6 min |
| T04-S · Sellar y verificar | Administrador o Litigante asignado; usar el documento creado en T03, aún sin sello. | Confirma sellado, solicita verificación explícita y reconoce su resultado; termina la descarga de evidencia ZIP de esa versión. No afirma firma personal ni constancia PSC. | 8 min |
| T04-V · Verificar sin permiso de sellar | Asistente legal; documento `verificacion-preparada.txt` previamente sellado por una cuenta autorizada del entorno sintético. | Verifica y guarda la evidencia de esa versión; reconoce que su cuenta no puede sellar. La ausencia de ese control es una condición esperada, no una falla de interfaz. | 6 min |
| T05 · Consultar versiones | Mismos roles; documento T03 existe; archivo `practica-v2.txt` distinto. | Añade una versión al documento existente y vuelve a consultar la primera; distingue identidades de documento/versión y estado del sello. No crea un documento nuevo por error ni atribuye a la segunda versión el sello de la primera. | 6 min |
| T08 · Cerrar sesión | Mismos roles; tarea previa concluida. | Cierra la sesión y vuelve a la pantalla de acceso; comprende que cerrar sesión no elimina los archivos guardados. No se prueba expiración por inactividad. | 2 min |

Los flujos están descritos en [manual de uso](../user-guide.md),
[contrato documental HTTP](../http-api.md),
[metadatos](../adr/0020-audited-document-classification.md) y
[contenido por versión](../document-content-api.md). El comportamiento visual se
puede contrastar con `web/src/components/Auth.svelte`, `Cases.svelte`,
`UploadDocument.svelte`, `MetadataEditor.svelte`, `DocumentDetail.svelte`,
`AppendVersion.svelte` y `DocumentVersions.svelte` del directorio de componentes.

## Módulos adicionales implementados

| Tarea | Roles y preparación | Resultado observable | Límite sugerido |
| --- | --- | --- | --- |
| T06 · Agenda y actividad | Administrador, Litigante o Asistente legal; audiencia programada y plazo con vencimiento operativo vigente en expediente asignado, preparados por el moderador. Entregar intervalo y desfase de consulta. | Encuentra ambos tipos de actividad en el periodo indicado, reconoce expediente/fecha y abre el detalle elegido. Distingue audiencia de plazo; no suma grupos solapados ni interpreta una fecha como resolución judicial. | 6 min |
| T07 · Administrar una asignación | Sólo Administrador; cuenta auxiliar ficticia activa, existente y sin asignación al caso de práctica. Entregar su correo, no sus credenciales. | Asigna la cuenta al expediente correcto, confirma que aparece entre asignados y retira la asignación cuando se lo pide la tarjeta. No cambia el rol, no elimina la cuenta y no habla de invitación enviada. | 6 min |

Referencias: [agenda](../agenda-api.md), [plazos](../deadline-tracking-api.md) y
[miembros](../members-api.md). La preparación de T06 usa únicamente datos
sintéticos bajo los catálogos vigentes; no pide inventar normas, fechas de una
actuación real ni fuentes jurídicas para completar una tarea. Si no se puede
preparar ese estado, T06 no se habilita y se informa la falta de cobertura.

## Módulos condicionales: comprobar versión antes de habilitar

| Tarea | Condición y preparación | Resultado observable | Límite sugerido |
| --- | --- | --- | --- |
| T09 · Solicitar y recuperar informe | Administrador o Litigante; informes con aceptación integrada comprobada y consumidor activo en la versión evaluada. Entregar fechas UTC que incluyan creación de dos casos de práctica, uno activo y otro cerrado, con asignación autorizada. | Solicita el informe, navega fuera y vuelve a consultar su disponibilidad; termina PDF y CSV, reconoce identidad/captura común y acusa el aviso expresamente. Explica que el periodo filtra creación y que no reconstruye un censo histórico. Si no termina, se registra lo ocurrido, no una descarga supuesta. | 8 min |
| T10 · Consultar actividad y verificar cadena | Sólo Administrador; consulta filtrada de actividad con aceptación integrada comprobada. Entregar periodo UTC y un texto exacto de recurso de práctica con más de 20 eventos reales ya registrados. | Filtra y consulta más de una página, reconoce que la página sustituye la anterior y que actualizar toma otro corte. Conserva la fecha y actor mostrados, sin inferir IP o UUID. Ejecuta además Verificar cadena y distingue su resultado del listado filtrado. | 8 min |

Ver [informes](../case-reports-api.md), el estado de módulos en
[product-completion.md](../product-completion.md) y los resultados en
[verification-report.md](../verification-report.md). No habilites T09 o T10
porque exista un menú: comprueba la revisión y una aceptación real de su recorrido.
La lectura de auditoría no debe prepararse insertando un historial ficticio:
crea los eventos con acciones autorizadas sobre datos de práctica.

## Selección y orden

- Secuencia base para Administrador/Litigante: T01, T02, T03, T04-S, T05, T08.
- Para Asistente legal: sustituye T04-S por T04-V; su documento preparado es otro.
- T06, T07, T09 y T10 son módulos seleccionables; colócalos antes de T08.
- T03 precede a T04-S y T05. Si T03 no se completa, registra ese resultado y ofrece
  después un documento preparado equivalente para observar las tareas siguientes.
  Registra el cambio de recurso; no conviertas la intervención en éxito de T03.
- No uses una variante no autorizada para medir a una persona. No compares tiempos
  de T04-S y T04-V como si fueran la misma tarea.
- La repetición a 768 px conserva un nuevo número de intento y registra el
  aprendizaje previo. No mezcla sus tiempos con el primer recorrido de escritorio.

## Pendientes excluidos de las tarjetas

El conjunto actual de tarjetas no solicita invitación por correo, recuperación
de contraseña o del enrolamiento perdido, login por certificado, sello personal,
expiración por inactividad con conservación de borradores, acceso documental de
Cliente ni activación jurídica automática de plazos. La recuperación de contraseña
y varias familias de borradores tienen implementación posterior; excluirlas de
estas tarjetas no significa que carezcan de código. Su evaluación humana requiere
seleccionar y comprobar una versión, un entorno habilitado y tareas propias antes
de incorporarlas; no se añaden resultados de participantes que no se hayan observado. Tampoco se asume envío de
correo o estimación de terminación de informes. Su exclusión no los elimina del
[alcance pendiente](../product-completion.md).
