# Lista de capturas del manual

Cada captura se guarda en `figures/capturas/` con el nombre indicado. Use datos de prueba: no muestre clientes ni expedientes reales. Al incorporarla, reemplace su `\capturapendiente` por `\includegraphics[width=0.9\textwidth]{capturas/...}`.

Además de mostrar la pantalla, cada captura confirma que los nombres de botones, campos y mensajes del texto coinciden con la interfaz.

| Archivo | Pantalla | Rol | Qué debe verse | Marcadores que resuelve |
|---|---|---|---|---|
| `capturas/A-01-configurar-despacho.png` | Creación de la primera cuenta de Administrador | sin cuenta (sistema vacío) | Pantalla Configura tu despacho con nombre, correo y contraseña | — |
| `capturas/A-02-configurar-segundo-factor.png` | Configuración del segundo factor y códigos de recuperación | Administrador | Pantalla Configura el segundo factor con clave y códigos (datos de prueba) | — |
| `capturas/A-03-aceptar-invitacion.png` | Inicio del alta desde una invitación | persona invitada (Litigante) | Pantalla Acepta tu invitación con el campo Tu contraseña | — |
| `capturas/A-04-confirmar-segundo-factor.png` | Confirmación del segundo factor durante el alta | persona invitada (Litigante) | Pantalla Confirma tu segundo factor con clave, códigos, casilla y código | — |
| `capturas/A-05-inicio-sesion.png` | Inicio de sesión con correo y contraseña | Litigante | Pantalla Accede a tu despacho con correo y contraseña | — |
| `capturas/A-06-segundo-factor.png` | Confirmación con el segundo factor | Litigante | Pantalla Un paso más con el campo Código de 6 dígitos y el aviso de minutos | [MENSAJE: número de minutos] en Iniciar sesión |
| `capturas/A-07-recuperar-acceso.png` | Solicitud del enlace de recuperación | Litigante | Pantalla Recupera tu acceso con el campo de correo | — |
| `capturas/A-08-nueva-contrasena.png` | Cambio de contraseña desde el enlace | Litigante | Pantalla Elige una nueva contraseña con ambos campos | — |
| `capturas/A-09-ingresar-certificado.png` | Inicio de sesión con certificado | Administrador | Pantalla Ingresa con tu certificado con el recibo cargado y la sección Firmar en este dispositivo | — |
| `capturas/A-10-menu-cuenta.png` | Menú de la cuenta | Litigante | Menú Tu cuenta abierto con nombre, correo, rol y Cerrar sesión | — |
| `capturas/A-11-menu-principal.png` | Menú principal del Administrador | Administrador | Pantalla completa con el menú lateral desplegado, incluida la sección Administración | — |
| `capturas/A-12-inicio-tablero.png` | Inicio del Administrador y del Litigante | Litigante | Inicio con indicadores, próximos plazos y audiencias y carga por litigante | — |
| `capturas/A-13-inicio-asistente.png` | Inicio del Asistente legal | Asistente legal | Inicio con Tu expediente actual, Accesos y Tu siguiente paso | — |
| `capturas/A-14-archivo-expedientes.png` | Archivo de expedientes | Litigante | Archivo de expedientes con filtros y tarjetas (Activo, Ficha completa) | — |
| `capturas/A-15-nuevo-expediente.png` | Registro de un expediente penal | Litigante | Formulario Nuevo expediente penal con identificadores y delitos registrados | — |
| `capturas/A-16-resumen-expediente.png` | Resumen del expediente | Litigante | Pestaña Resumen con ficha penal completa, estado y Registro inicial de etapa | — |
| `capturas/A-17-cerrar-expediente.png` | Cierre administrativo de un expediente | Litigante | Ventana Cerrar administrativamente con su aviso y el botón de confirmación | — |
| `capturas/A-18-archivo-documental.png` | Archivo documental del expediente | Litigante | Pestaña Documentos con buscador, filtros y lista con varios estados | — |
| `capturas/A-19-subir-documento.png` | Carga de un documento | Asistente legal | Ventana Subir documento con archivo elegido, nombre sugerido y clasificación | — |
| `capturas/A-20-ficha-documental.png` | Ficha documental | Litigante | Ficha documental de un documento sellado con acciones y pestañas Resumen, Verificación y Evidencia | — |
| `capturas/A-21-editar-clasificacion.png` | Clasificación de un documento | Asistente legal | Ventana Editar clasificación con tipo, clasificación y etiquetas | — |
| `capturas/A-22-versiones.png` | Historial de versiones de un documento | Litigante | Tarjeta Versiones de contenido con versión Actual e Histórica | — |
| `capturas/A-23-sellar-documento.png` | Confirmación del sellado | Litigante | Confirmación ¿Registrar el sello del servidor? con el botón Confirmar sellado | — |
| `capturas/A-24-resultado-verificacion.png` | Resultado de la verificación | Litigante | Pestaña Verificación con Verificación válida y las cuatro comprobaciones en Correcto | — |
| `capturas/A-25-evidencia.png` | Paquete de evidencia y su validación | Litigante | Pestaña Evidencia con Tu paquete de evidencia y el validador con resultado Completo | — |
| `capturas/A-26-firma-personal.png` | Firma personal de un documento | Litigante | Sección Firma personal de esta versión con PDF revisado y firma local | — |
| `capturas/A-27-agregar-participante.png` | Registro de un participante | Litigante | Ventana Agregar participante tipificado con Datos de identidad y Ficha de participación | — |
| `capturas/A-28-etapas.png` | Registro de un cambio de etapa | Litigante | Pestaña Etapas con Etapa actual registrada y el formulario de paso a Intermedia | — |
| `capturas/A-29-programar-audiencia.png` | Programación de una audiencia | Litigante | Formulario de audiencia con tipo, modalidad, fecha, sede y participantes | — |
| `capturas/A-30-decision-cautelar.png` | Registro de una decisión cautelar | Litigante | Formulario Registrar decisión cautelar con un efecto agregado | — |
| `capturas/A-31-resoluciones.png` | Resoluciones y notificaciones | Litigante | Pestaña Resoluciones y notificaciones con una resolución y su notificación | — |
| `capturas/A-32-recursos.png` | Recursos procesales | Litigante | Pestaña Recursos procesales con un recurso y la sección Actividades del recurso | — |
| `capturas/A-33-registrar-plazo.png` | Registro de un plazo | Litigante | Revisión de un plazo preparado con resultado y pasos del cálculo | — |
| `capturas/A-34-agenda.png` | Agenda del despacho | Litigante | Agenda del despacho en vista Mes con audiencias y plazos | — |
| `capturas/A-35-mis-alertas.png` | Bandeja de alertas | Litigante | Mis alertas con alertas sin leer y el botón Marcar como leída | — |
| `capturas/A-36-solicitar-firmas.png` | Solicitud de firmas de un contrato | Litigante | Formulario Solicitar firmas con correo del Cliente, firmante, vigencia y folio | — |
| `capturas/A-37-detalle-contrato.png` | Detalle de un contrato | Litigante | Detalle de un contrato con su solicitud, línea de tiempo y botones de descarga | — |
| `capturas/A-38-por-firmar.png` | Firma del contrato por el despacho | Litigante | Por firmar con los pasos 1. Revisa, 2. Confirma y 3. Firma y el acuse parcial validado | — |
| `capturas/A-39-informes.png` | Solicitud y descarga de un informe | Litigante | Informes de expedientes con la solicitud y un informe Disponible con Descargar PDF y CSV | — |
| `capturas/A-40-generar-clave.png` | Generación de la clave y la solicitud | Litigante | Sección Generar clave y solicitud con Datos de la solicitud y contraseña | — |
| `capturas/A-41-hoja-huella.png` | Hoja de huella de firma documental | Litigante | Vista de impresión de la Hoja de huella de firma documental con ambos ejemplares (datos de prueba) | — |
| `capturas/A-42-registrar-vinculo.png` | Registro del vínculo del certificado | Litigante | Registrar certificado documental con certificado cargado y Firmar en este dispositivo | — |
| `capturas/A-43-acceso-certificado.png` | Certificado para ingresar a Qadra | Administrador | Pestaña Acceso con certificado con Vínculo de mi cuenta y Descargar recibo | — |
| `capturas/A-44-invitaciones.png` | Invitaciones al despacho | Administrador | Equipo con el formulario Invitaciones al despacho y la lista de invitaciones con sus estados | — |
| `capturas/A-45-nuevo-integrante.png` | Alta directa de un integrante | Administrador | Formulario Nuevo integrante con nombre, correo, contraseña inicial y rol | — |
| `capturas/A-46-administrar-acceso.png` | Cambio de acceso de una cuenta | Administrador | Directorio de cuentas con el editor de acceso abierto y la confirmación del cambio | — |
| `capturas/A-47-asignaciones.png` | Asignación de una cuenta a un expediente | Administrador | Pestaña Asignaciones con Cuentas disponibles y la ventana Asignar cuenta | — |
| `capturas/A-48-bitacora.png` | Bitácora del despacho | Administrador | Bitácora con Cadena íntegra y la lista de Movimientos registrados | — |
| `capturas/A-49-calendarios.png` | Calendarios jurisdiccionales | Administrador | Calendarios jurisdiccionales con un calendario y Días según esta revisión en vista de mes | — |
| `capturas/B-01-mi-firma-digital-cliente.png` | Mi firma digital del Cliente | Cliente | Mi firma digital con Generar clave y solicitud y la organización fija | — |
| `capturas/B-02-firmar-contrato-cliente.png` | Firma del contrato por el Cliente | Cliente | Mis contratos con los pasos 1. Revisa, 2. Confirma y 3. Firma y el PDF comprobado | — |
| `capturas/B-03-contrato-formalizado.png` | Descarga del acuse y del contrato firmado | Cliente | Contrato en estado Formalizado con Descargar acuse parcial y Descargar contrato firmado | — |
| `capturas/B-04-validar-evidencia.png` | Validación de un paquete de evidencia | Cliente | Validar evidencia con un contrato firmado y Resultado de la validación Completo | — |
| `capturas/B-05-verificacion-openssl.png` | Verificación independiente con OpenSSL | sin cuenta | Terminal con Verified OK, certificado.pem: OK y Verification: OK | — |
