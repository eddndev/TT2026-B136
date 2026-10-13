# Portal CA y acceso Owner

El portal CA tiene implementación integrada de acceso, enrolamiento administrativo,
solicitudes, inventario, revocación, publicación de CRL e historial recuperable.
Su [manual de operación](operations.md) describe el código actual y separa los
pasos de instalación todavía pendientes. La release que incluye CA está instalada en VPS3; el servicio permanece
cerrado hasta el alta privada del operador. La Owner aún debe aceptar su
recorrido real.

Consultar también la [verificación ejecutada](verification.md) y el
[procedimiento de restauración](restore.md).

## Identidades y límites de autoridad

El operador designado de la CA es `eddndev@gmail.com`. Se enrola mediante
`qadra-ca enroll-operator`, con credenciales, MFA, recuperación, auditoría y
sesiones propios de CA. El servicio admite únicamente el UUID y correo
configurados. No necesita una cuenta del despacho ni un alta Owner.

La cuenta `hatziryvh@gmail.com` pertenece al despacho y ya existe como Owner
activa con MFA. Esa condición no concede acceso al portal CA. El valor interno
`Client` del registro CA permite reutilizar el formato de credenciales; no es
una asignación de permisos del despacho. Las rutas del despacho no se componen
en `qadra-ca`.

El destino es el mismo VPS3, bajo la cuenta de despliegue
`qadra`, con un servicio separado y HTTPS en `ca.qadra.com.mx`. Comparte los
servicios PostgreSQL y Redis existentes mediante su configuración y autoridad
propias. La interfaz vive en `web/apps/ca/` y reutiliza `web/packages/ui/`.

## Estado de la instalación pública

El 13 de octubre de 2026 Qadra se activó en `v0.1.8`, revisión
`95a9aacf975e4a9cde5c4f8b60544fab3baff84f`, tras calificar el paquete y el retorno
aislados. El respaldo externo se verificó; se conservaron las 77 tablas del
despacho, sus datos, correo y material criptográfico. Después se adoptaron el
esquema `qadra_ca`, su rol restringido y la configuración privada de alta, sin
crear todavía una cuenta CA. El acceso del despacho se comprobó nuevamente.

Ese mismo día se preparó `ca.qadra.com.mx` como origen independiente
con DNS y HTTPS verificados. El certificado del nuevo origen conserva su
renovación mediante el webroot y el temporizador existentes; el certificado
del despacho no cambió. El proxy de CA devuelve `503` tanto en la página como
en la API mientras no exista el alta privada del responsable. Ese cierre es
intencional y no acredita que el servicio CA esté activo.

El alta real de `eddndev@gmail.com` requiere contraseña y confirmación de TOTP
en una terminal privada. No se generó una identidad ficticia para arrancar el
servicio. Después del alta se podrá preparar su configuración con el UUID
persistido, iniciar las unidades separadas y abrir el proxy hacia el puerto
CA de loopback. La cuenta Owner conserva su acceso actual; el primer factor
por certificado permanece deshabilitado hasta completar su aceptación.

## Estado del código

| Capacidad | Implementación actual | Límite de la evidencia |
| --- | --- | --- |
| Alta inicial | Comando local, archivo nuevo privado y confirmación TOTP antes de insertar cuenta y auditoría en una transacción. | Seis casos nativos locales de enrolamiento aprobados; el esquema ya está preparado en VPS3, sin alta del operador. |
| Acceso CA | Contraseña y TOTP o código de recuperación, consulta de sesión, actividad y cierre. Credenciales y sesiones separadas del despacho. | La implementación y comprobaciones locales no acreditan disponibilidad pública. |
| Solicitud y emisión | CSR público, revisión explícita y emisión de `partner` o `document-signing`, con operación recuperable y recibo público. | Pendiente el recorrido integral instalado con los titulares reales. |
| Inventario | Serie, sujeto, propósito, huellas, fechas, estado local y comparación con la CRL publicada. Descarga del certificado público exacto. | El TSA existente se consulta como `time-stamping`; no se emite ni revoca desde el portal. |
| Revocación | Confirmación de serie, huella y motivo; registro local recuperable. | El recibo local no acredita publicación ni rechazo posterior en Qadra. |
| Publicación | Puente al mantenimiento CRL existente, con revisión esperada, confirmación de mantenimiento y reanudación. | El nuevo acceso desde el portal aún requiere aceptación de su composición instalada. |
| Historial | Consulta y reanudación de emisiones, revocaciones y publicaciones con sus identificadores y parámetros originales. | Un estado parcial o una identidad distinta bloquean el avance; no se reparan automáticamente. |
| Instalación | Binario `qadra-ca`, configuración privada y composición opcional de operaciones. | Paquete, esquema restringido y HTTPS preparados; faltan alta real, configuración operativa y arranque de servicios. |

La verificación focal de enrolamiento del 12 de octubre de 2026 ejercitó el
binario real con PostgreSQL y Redis desechables: entrega privada y TOTP real,
credencial persistida, auditoría, rechazo de repetición, entradas incompletas,
archivo alterado y terminal sin eco. Los casos están en
[`ca_enrollment.rs`](../../crates/bin/tests/ca_enrollment.rs).
No sustituye la aceptación conjunta del portal, los recorridos de certificados
y el despliegue. La regresión y CI de cierre aprobaron para la revisión integrada; sus
resultados se distinguen en la [verificación ejecutada](verification.md).

## Adopción de la CA existente

Se conservan raíz, clave CA, certificados, índice, contadores, TSA y confianza
publicada. La configuración fija la huella de raíz esperada; el portal compara
material local y confianza persistida antes de las operaciones correspondientes.
No se crea una autoridad nueva para instalar el portal.

La emisión reutiliza la [PKI existente](../../pki/README.md). El titular conserva
su clave privada fuera de la CA y comparte sólo el CSR público. Emitir un
certificado no crea una cuenta, no registra un vínculo en Qadra y no acredita
identidad civil. La vinculación conserva su prueba de posesión. Retirar un
vínculo y revocar el certificado son operaciones distintas.

La publicación reutiliza el [mantenimiento CRL](../deployment-crl.md), su
bloqueo, respaldo, registro de recuperación y control de los servicios de
usuario existentes. La UI informa de la interrupción de Qadra antes de pedir
confirmación. El alta del portal no instala otro controlador de CRL ni programa
renovaciones automáticas.

## Recorrido Owner pendiente

El acceso por certificado ya dispone de backend e interfaz, documentados en
la [guía de operación Owner](../owner-certificate-login-operations.md), la
[interfaz de acceso](../owner-certificate-login-interface.md) y el
[vínculo propio](../owner-certificate-interface.md). La observación de producción
del 12 de octubre de 2026 mostró `enabled:false`; describe aquella observación,
no una carencia de implementación ni el estado de una activación posterior.

El [perfil de activación preparado](owner-activation.md) fija cuotas, ventanas,
tiempos Redis, retorno y pasos de aceptación sin cambiar la inactividad.
Para aceptar el recorrido de `hatziryvh@gmail.com` quedan por registrar:

1. Acceso alternativo a su cuenta mediante contraseña y MFA.
2. Certificado Partner apropiado, con la clave bajo control de la titular;
   emisión desde su CSR externo si hace falta.
3. Raíz y CRL publicadas, vínculo vigente y prueba de posesión, conservando el
   recibo y sin volver a crear un vínculo existente.
4. Configuración revisada del flag, cuotas, ventanas y tiempos de Redis,
   conservando la configuración anterior.
5. Activación controlada, firma externa de un desafío nuevo, MFA de la misma
   cuenta y comprobación del acceso alternativo.
6. Evidencia de revisión y configuración no secreta, con retorno preparado
   sin borrar cuentas, vínculos, certificados o auditoría.

## Cierre pendiente y exclusiones

Falta enrolar al operador real por entrada privada, adoptar el servicio con su
UUID y abrir el proxy HTTPS preparado para completar la aceptación
del portal con la CA conservada. La aceptación debe relacionar los dos perfiles
emitidos con sus recorridos de Qadra, la revocación publicada, el rechazo de
usos nuevos y la conservación de evidencia histórica. El manual indica qué
configuración es ilustrativa y qué decisiones de instalación faltan.

El alcance conserva un operador designado y dos propósitos de emisión. Excluye
autoservicio de alta, bootstrap HTTP público, plataforma de invitaciones, SSO,
ejecución libre de shell, sustitución de raíz, jerarquías nuevas, OCSP, FIREL/PSC,
PAdES y renovación automática de CRL. El verificador de ZIP es una entrega
separada. Las claves privadas de los titulares no se reciben en el portal.
