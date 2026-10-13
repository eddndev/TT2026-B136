# Acceso Owner y portal de la CA

Este documento delimita dos entregas: activar el acceso Owner por certificado
ya construido y crear un portal para operar la CA existente. El portal esta
pendiente de implementacion y aceptacion; la reorganizacion del workspace no
acredita funcionalidad ni instalacion del servicio.

## Base existente y ubicacion

El acceso Owner ya tiene backend, interfaz y aceptaciones registradas en
[su guia de operacion](../owner-certificate-login-operations.md). La observacion
de produccion del 12 de octubre de 2026 mostro `enabled:false`; esa observacion
acredita que estaba deshabilitado, no que faltaba el codigo.

La [CA existente](../../pki/README.md) dispone de emision, inspeccion y revocacion.
El [mantenimiento de CRL](../deployment-crl.md) tiene renovacion y publicacion
recuperables. La emision desde CSR publico admite firma documental; queda
pendiente extenderla de forma focal al perfil Partner usado por el acceso Owner.

La estructura de frontend reserva `web/apps/ca/` para el portal y
`web/packages/ui/` para los elementos de presentacion compartidos con
`web/apps/qadra/`. El portal reutilizara la identidad visual y los componentes
de Qadra. Su servicio sera el binario separado `qadra-ca`, en el mismo VPS3,
con HTTPS en `ca.qadra.com.mx`. Esta ubicacion no concede autoridad de CA a las
cuentas Owner del despacho.

## Activacion del acceso Owner

El recorrido reutiliza la [interfaz de acceso](../owner-certificate-login-interface.md)
y el [vinculo propio del certificado](../owner-certificate-interface.md).
Quedan pendientes estas condiciones operativas:

1. Identificar la cuenta Owner existente y confirmar su acceso alternativo con
   contrasena y MFA.
2. Comprobar un Partner vigente cuya clave conserve el titular. Si hace falta
   emitir desde una clave externa, completar primero ese perfil del emisor CSR.
3. Comprobar raiz y CRL publicadas, vinculo vigente y prueba de posesion; registrar
   el vinculo solo si falta y conservar el recibo.
4. Preparar el flag de activacion, cuatro cuotas con sus ventanas y dos tiempos
   Redis. Documentar el perfil elegido y conservar la configuracion anterior.
5. Activar de forma controlada, comprobar disponibilidad, firmar un desafio nuevo
   fuera de Qadra y completar MFA en la misma cuenta. Comprobar tambien el acceso
   independiente por contrasena.
6. Preparar el retorno a la configuracion anterior y registrar revision,
   configuracion no secreta y evidencia del recorrido. Preservar vinculos,
   certificados, auditoria y datos.

## Alcance finito del portal

Todas las operaciones de esta tabla estan pendientes de implementacion y
aceptacion desde el portal. Los mecanismos CLI existentes conservan su alcance.

| Operacion | Resultado requerido |
| --- | --- |
| Acceso de operadores | Solo responsables designados de la CA, con MFA y recuperacion definida. Consultar y reanudar mantenimiento aunque Qadra este detenido. |
| Adopcion de la CA | Conservar raiz, claves, certificados, indice y contadores. Mostrar huella, despliegue, revision de confianza y vigencia de CRL; bloquear cambios ante discrepancias. |
| Solicitudes | Recibir solo CSR publicos con validacion de prueba de posesion, tamano y perfil. Separar acceso Owner/Partner y firma documental Litigante. |
| Revision y emision | Confirmacion del operador sobre titular declarado y proposito. Registrar operador, referencia y criterio; entregar certificado publico y recibo con serie y huella. |
| Inventario | Consultar sujeto, proposito, vigencia, serie, huella y revocacion. Descargar material publico y distinguir estado local de estado publicado. Una sustitucion por vencimiento conserva el certificado anterior. |
| Revocacion | Confirmar serie y huella y registrar operador y motivo. Mostrar pendiente de publicacion hasta que Qadra reconozca la CRL sucesora. |
| CRL y publicacion | Reutilizar estado, renovacion y reanudacion existentes, con bloqueo compartido, revision esperada, respaldo y relectura de la sucesora exacta. Avisar del mantenimiento antes de confirmar. |
| Historial y recuperacion | Atribuir cada operacion y conservar recibos. Consultar o reanudar una operacion interrumpida sin duplicar emisiones ni CRL; serializar cambios del indice y contadores. |
| Aceptacion funcional | Emitir ambos perfiles desde CSR externos, usarlos en sus recorridos de Qadra, revocar y publicar; comprobar rechazo de uso nuevo y conservacion de evidencia historica. Cubrir permisos, respuesta perdida e interrupciones de las partes nuevas. |
| Instalacion | Servicio separado y HTTPS del dominio propio, permisos acotados y separacion de interfaz, ejecutor PKI y API del despacho. Comprobar respaldo y restauracion sin sustituir la raiz. |
| Entrega | Manual del operador, estados y pendientes, trazabilidad y recibos de emision, consulta, revocacion, publicacion y recuperacion desde la interfaz. |

Emitir un certificado no crea una cuenta, no vincula automaticamente el
certificado a Qadra y no acredita identidad civil. El vinculo sigue usando su
prueba de posesion existente. Retirar un vinculo y revocar un certificado son
operaciones distintas.

## Pendientes y limites

Falta identificar al Owner y a los operadores concretos, definir el mecanismo
de acceso y recuperacion de estos ultimos, preparar el perfil de configuracion
Owner y concretar la ventana de mantenimiento. La instalacion compartida en
VPS3 y la separacion del servicio CA ya forman parte del alcance.

Las claves privadas de los titulares permanecen fuera del portal. El alcance
excluye autoservicio, ejecucion libre de shell, regeneracion o sustitucion de
raiz, nuevas jerarquias, OCSP, SSO general, nuevas finalidades de certificado,
FIREL/PSC, PAdES y renovacion automatica de CRL. El verificador de ZIP es una
entrega separada y no forma parte del portal CA ni del despacho.

La evidencia existente se reutiliza dentro de sus limites. Los cambios nuevos
requieren sus comprobaciones focales, y el cierre requiere una regresion
completa pertinente y los gates de la revision que se integre. Este documento
no registra una nueva ejecucion de pruebas ni una activacion productiva.
