# Portal CA y acceso Owner

El portal CA tiene implementación integrada de acceso, enrolamiento administrativo,
solicitudes, inventario, revocación, publicación de CRL e historial recuperable.
Su [manual de operación](operations.md) describe el código actual y su instalación.
El servicio está activo en VPS3 y el acceso público de `eddndev@gmail.com`
mediante contraseña y TOTP se comprobó en el navegador. La Owner aún debe
aceptar su recorrido real.

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
esquema `qadra_ca`, su rol restringido y la configuración privada de alta.
El acceso del despacho se comprobó nuevamente.

Ese mismo día se preparó `ca.qadra.com.mx` como origen independiente
con DNS y HTTPS verificados. El certificado del nuevo origen conserva su
renovación mediante el webroot y el temporizador existentes; el certificado
del despacho no cambió. El 14 de octubre, tras el enrolamiento real, el proxy se abrió hacia
`127.0.0.1:18088`, sin modificar el vhost del despacho.

El 14 de octubre de 2026 el operador `eddndev@gmail.com` quedó registrado con UUID
`8e76db9c-7ac4-4047-abd6-46e4b53c6ea4`. Con autorización del usuario se generaron
credenciales aleatorias y se guardaron en archivos privados. Las unidades
`qadra-ca.service` y `qadra-ca-web.service` se abrieron con esa release;
los cuatro servicios del despacho permanecieron activos. El acceso
por contraseña y TOTP en `https://ca.qadra.com.mx` mostró la sesión del operador.
En esa apertura inicial el primer factor Owner por certificado permaneció
deshabilitado; su activación posterior se distingue más adelante.

La aceptación de emisión encontró un defecto: un CSR que sólo contenía CN pasó
la revisión previa, pero OpenSSL rechazó la ausencia de país y organización.
La operación quedó pendiente sin emitir ni modificar índice o contador. La
corrección se integró mediante PR 107 en
`b7d1d6d1c4230d9e3796bc235d934dece73b7fd6` y se desplegó como `v0.1.9` el
14 de octubre, tanto en Qadra como en la CA. El journal original se conservó y
cerró como rechazado sin emitir; su recibo se comprobó en el historial público.
La admisión anticipada aplica la política existente antes de reservar una nueva
emisión. Véase la [evidencia del incidente](verification.md).

El respaldo posterior al alta real se restauró en una base aislada: coincidieron
la cuenta CA, sus tres entradas de auditoría y los 37 archivos de CA, TSA y
configuración. Se verificaron los hashes de los tres payloads y la copia externa.
Se reutiliza la aceptación Redis previa; no se afirma reconstrucción completa
del host ni se añade ese ensayo al alcance.

La parada administrativa de la CA anterior expuso un aborto al liberar clientes
PostgreSQL síncronos dentro de Tokio. La reparación local conserva sus dueños
fuera del contexto asíncrono y aprobó la prueba nativa SIGTERM junto con acceso
y enrolamiento. La PR 108 se integró el 14 de octubre en
`f8ebeb9ef8a22e0b8dfde2d1f805311a4f5115c1`, con los gates de la PR aprobados.
Los gates de ese main aprobaron y el candidato `v0.1.10` está en construcción; la release
`v0.1.9` activa aún conserva el defecto de cierre.

## Estado del código

| Capacidad | Implementación actual | Límite de la evidencia |
| --- | --- | --- |
| Alta inicial | Comando local, archivo nuevo privado y confirmación TOTP antes de insertar cuenta y auditoría en una transacción. | Operador real enrolado en VPS3; se conserva además la evidencia de seis casos nativos locales. |
| Acceso CA | Contraseña y TOTP o código de recuperación, consulta de sesión, actividad y cierre. Credenciales y sesiones separadas del despacho. | Acceso público con contraseña y TOTP comprobado en navegador; recuperación y cierre tienen evidencia local independiente. |
| Solicitud y emisión | CSR público, revisión explícita y emisión de `partner` o `document-signing`, con operación recuperable y recibo público. | Partner real `1002` y certificado técnico de firma documental `1003` emitidos desde el portal instalado; el incidente original conserva su rechazo sin emitir. |
| Inventario | Serie, sujeto, propósito, huellas, fechas, estado local y comparación con la CRL publicada. Descarga del certificado público exacto. | Descarga de `1003` comprobada en navegador y contrastada con su CSR. El TSA existente se consulta como `time-stamping`; no se emite ni revoca desde el portal. |
| Revocación | Confirmación de serie, huella y motivo; registro local recuperable. | `1003` revocado y publicado en R4; la comprobación criptográfica con esa CRL lo rechaza y admite al Partner `1002`. |
| Publicación | Puente al mantenimiento CRL existente, con revisión esperada, confirmación de mantenimiento y reanudación. | Publicación instalada R3 a R4, CRL 4099, completada desde el portal; mantenimiento aceptado y seis servicios activos. |
| Historial | Consulta y reanudación de emisiones, revocaciones y publicaciones con sus identificadores y parámetros originales. | Rechazo original, emisión/revocación de `1003` y publicación R4 comprobados en la UI instalada; un estado parcial o una identidad distinta sigue bloqueando el avance. |
| Instalación | Binario `qadra-ca`, configuración privada y composición de operaciones. | Esquema restringido, operador, ambas unidades y HTTPS activos en VPS3; la apertura no acredita por sí sola los recorridos de mutación. |

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

## Recorrido Owner y aceptación pendiente

El acceso por certificado ya dispone de backend e interfaz, documentados en
la [guía de operación Owner](../owner-certificate-login-operations.md), la
[interfaz de acceso](../owner-certificate-login-interface.md) y el
[vínculo propio](../owner-certificate-interface.md). El 14 de octubre de 2026 se
emitió el Partner `1002` para `hatziryvh@gmail.com`; la declaración de posesión
se firmó fuera del portal, sin cargar la clave privada. Una persona registró
el vínculo R1 en Chrome a las 08:29:28 UTC. Se conservan la cuenta y sus factores
anteriores; emitir el certificado no creó ni sustituyó esa cuenta.

El [perfil de activación](owner-activation.md) se aplicó sobre `v0.1.9`, con
`enabled:true`, cuotas, ventanas y tiempos Redis explícitos y checkpoint de la
configuración anterior. La firma real del desafío de acceso fue admitida y el
recorrido llegó al segundo factor. **MFA sigue pendiente**: todavía no se afirma
una sesión Owner completa por certificado. También falta repetir el acceso
alternativo por contraseña y MFA después de la activación; su observación previa
no sustituye esa comprobación. Si el desafío ha vencido, se necesita uno nuevo.

El cierre de este recorrido requiere registrar ambos accesos completos y
conservar sus recibos y configuración no secreta. No requiere volver a emitir
`1002`, crear otro vínculo ni borrar cuentas, certificados o auditoría.

## Cierre pendiente y exclusiones

Falta terminar el paquete y desplegar la reparación del cierre del proceso,
y completar los dos accesos Owner pendientes. La emisión, descarga, revocación,
publicación e historial ya tienen
la evidencia instalada descrita arriba. La
correlación de ambos perfiles con Qadra, rechazo de nuevos usos y conservación
de evidencia histórica ya tiene aceptación nativa controlada; se conserva
separada de la operación pública. El ensayo del respaldo poblado complementa
las pruebas SQL, Redis y archivo privado ya aceptadas dentro de sus límites.

El alcance conserva un operador designado y dos propósitos de emisión. Excluye
autoservicio de alta, bootstrap HTTP público, plataforma de invitaciones, SSO,
ejecución libre de shell, sustitución de raíz, jerarquías nuevas, OCSP, FIREL/PSC,
PAdES y renovación automática de CRL. El verificador de ZIP es una entrega
separada. Las claves privadas de los titulares no se reciben en el portal.
