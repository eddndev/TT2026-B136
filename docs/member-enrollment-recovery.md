# Recuperar un alta de integrante tras vencer la sesión

En **Equipo**, Qadra puede conservar el correo escrito, el rol seleccionado y
si el envío quedó sin confirmar. El correo conserva sus mayúsculas y texto
parcial. El borrador permanece sólo en la memoria de la pestaña.

La contraseña inicial, la clave TOTP, la URI de configuración y los códigos de
recuperación quedan fuera del borrador. Esta función no recupera el acceso ni
el segundo factor del nuevo integrante.

## Retomar un formulario

1. Vuelve a ingresar con contraseña y MFA de la misma cuenta Owner.
2. Abre **Equipo** y pulsa **Retomar alta de integrante**.
3. Qadra consulta la identidad actual antes de mostrar los campos. Mientras
   espera, el formulario y su envío están bloqueados.
4. Si el alta todavía no se había enviado, revisa correo y rol e introduce
   otra vez la contraseña inicial antes de decidir su envío.

Entrar con otra cuenta o cerrar sesión expresamente descarta el borrador.
Una denegación de autoridad al consultar el acceso también lo descarta.
Recargar o cerrar la pestaña pierde su contenido. Autenticarse de nuevo no
crea una cuenta ni reenvía una solicitud.

## Cuando no se conoce el resultado del envío

El formulario conserva la intención sin contraseña y bloquea **Crear usuario**.
**Consultar directorio para esta cuenta** comprueba de nuevo la autoridad Owner
y busca el correo exacto en todas las páginas del directorio, incluidas las
cuentas inactivas.

Encontrar una cuenta no demuestra que ese envío la creó. No encontrarla tampoco
permite reenviar el intento. La consulta no entrega claves ni códigos MFA.
**Descartar alta y volver al directorio** elimina expresamente la intención y
deja el formulario vacío; cualquier alta posterior exige una nueva entrada y
decisión del usuario.

Salir de **Equipo** conserva un intento incierto. Al volver se ofrecen
**Retomar alta de integrante** y **Descartar alta y volver al directorio**;
retomarlo exige otra consulta de autoridad antes de mostrar correo y rol.
El bloqueo permanece aunque ya se hubiera recuperado tras una sesión vencida.
Los campos de un formulario que nunca se envió pueden reiniciarse al navegar.

Cuando la respuesta de creación sí se confirma, Qadra elimina la captura antes
de mostrar el material MFA de una sola vez y antes de refrescar el directorio.
Una respuesta de la sesión anterior no vuelve a mostrar ese material después
de una nueva autenticación.

## Alcance comprobado

La aceptación local con HTTP controlado aprobó **9 escenarios en 22,5 segundos**:
ocho nuevos de recuperación, autoridad, aislamiento, resultado incierto y
navegación, más el recorrido existente de alta con MFA y auditoría en pantalla
móvil. El registro aprobó **20 pruebas Node en 298,159 ms**: seis nuevas de
captura individual y catorce regresiones existentes.
Los casos están en
[recuperación de alta](../web/tests/browser/session-member-enrollment-drafts.spec.mjs),
[resultado incierto](../web/tests/browser/session-member-enrollment-outcomes.spec.mjs),
[navegación](../web/tests/browser/session-member-enrollment-navigation.spec.mjs)
y [flujo general](../web/tests/browser/workflow.spec.mjs). Las comprobaciones del
registro se conservan en
[captura individual](../web/tests/draft-handle-capture.test.mjs) y
[regresiones del registro](../web/tests/draft-registry.test.mjs).

Esta evidencia cubre vencimiento, reingreso y navegación antes y después de
recuperar una intención incierta. No acredita una campaña de alta con servicios
reales, restauración operativa, CI ni despliegue. Tampoco añade recibos de alta,
invitaciones o reemisión de enrolamiento.

Consulta también [la recuperación de editores](session-editor-recovery.md)
y [el contrato de cuentas](members-api.md).
