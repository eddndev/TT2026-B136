# Transporte publico de recuperacion de contrasena

## Frontera implementada y activacion

Los adaptadores de correo, HTTP y formulario tienen verificacion focal local.
El router acepta componentes opcionales: si no se inyectan, responde 503 y no
emite enlaces. El constructor anterior de la API conserva ese modo deshabilitado.
El servidor compone un consumidor supervisado solo con activacion explicita y
configuracion completa. El despliegue privado todavia no activa recuperacion.

La capacidad, su consumo transaccional y la invalidacion despues de restaurar
se describen en [el contrato interno](password-reset-internal.md). Restablecer la
contrasena conserva el segundo factor y no inicia una sesion.

## Rutas

| Peticion | Entrada JSON | Resultado |
| --- | --- | --- |
| `POST /api/v1/auth/password-reset/request` | `email` | 202 para admision o cupo local ocupado; 503 si el servicio completo no esta disponible. |
| `POST /api/v1/auth/password-reset/complete` | `token`, `new_password` | 204 al cambiar; 400 si la capacidad no se puede usar o la politica rechaza la contrasena; 503 ante resultado incierto o servicio no disponible. |

No aceptan query string, campos desconocidos, objetos JSON invalidos ni cuerpos
mayores de 16384 bytes, incluido el espacio en blanco. El tipo de contenido debe
ser JSON. Un token tiene exactamente 32 bytes representados en base64url canonico
sin relleno. Las contrasenas conservan el intervalo de 12 a 1024 bytes.

La solicitud publica no espera busqueda de cuenta, emision, entrega ni otra tarea
sincrona. Su respuesta no confirma existencia de la cuenta ni envio de un correo.
Los errores no incorporan mensajes de PostgreSQL, Redis o del proveedor.
El cambio usa el mismo presupuesto de trabajo bloqueante que las rutas privadas.
Cancelar la espera HTTP no libera el permiso de una operacion que sigue ejecutandose.
Todas las rutas conservan el limite compartido de admision y `Cache-Control: no-store`.

## Entrega

`ResendPasswordResetDelivery` exige remitente simple, raiz HTTPS aprobada y clave
privada. El destino de transporte es fijo: `https://api.resend.com/emails`.
No sigue redirecciones ni hereda proxies del entorno; tampoco reintenta. Conexion y tiempo
total deben configurarse expresamente dentro de sus limites tecnicos de 3 y 10
segundos. No se seleccionan valores operativos por defecto.

El mensaje incluye una sola URL con el token en `#password-reset=...`, nunca en
el path o query. El fragmento no viaja en la peticion HTTP inicial del navegador;
si sigue siendo visible en el cliente de correo o el historial del navegador,
ese material conserva la misma autoridad que el enlace original. El emisor usa
una clave de idempotencia independiente, sin prometer deduplicacion durable tras
un reinicio. Una respuesta valida de aceptacion no demuestra lectura ni entrega
final al destinatario.

El adaptador lee como maximo 8 KiB mas un byte de deteccion. Solo reconoce como
rechazo definitivo los codigos y tipos documentados que lo acreditan; respuestas
ambiguas, truncadas o inesperadas conservan el resultado incierto. Sus buffers
propios se borran al liberarse, sin garantizar el borrado de copias internas del
cliente HTTP o del proveedor. Ninguna prueba focal envio correos externos.

## Composicion y cierre del servidor

`serve` mantiene la recuperacion deshabilitada por defecto. La clave compartida
`RESEND_API_KEY` no habilita por si sola ni recuperacion ni alertas de correo.
Las opciones de recuperacion se agrupan bajo `--password-reset-*` y las variables
`TT_PASSWORD_RESET_*`; la clave no se admite como argumento de linea de comandos.
Para habilitar se exige remitente, raiz HTTPS, vida de la capacidad, maximo de
pendientes, cuotas y ventanas globales y por identidad para solicitud y consumo,
y tiempos de conexion y lectura. La configuracion incompleta se rechaza antes de
abrir el servidor. Las ventanas del limitador van de 1 a 86400 segundos.

La admision guarda como maximo una solicitud entre espera y ejecucion. Ocupacion
no demuestra existencia de una cuenta ni envio; se conserva la respuesta neutra.
El consumidor obtiene un permiso del mismo presupuesto bloqueante que HTTP antes
de ejecutar SQL, Redis, hash o correo. El permiso pertenece al trabajo real,
incluso si el solicitante abandona la conexion. La cola no es durable y una
solicitud aceptada puede descartarse al detener el servidor antes de iniciarse.

La detencion cierra la admision, descarta trabajo no iniciado y espera el que ya
comenzo. No cancela ni desprende una llamada sincrona en curso. Los propietarios
de adaptadores sobreviven al runtime asincrono para que su cierre no bloquee
un hilo de Tokio. Un fallo del consumidor solicita el cierre de los otros
consumidores y de HTTP. Esto no acredita un tiempo maximo de detencion frente a
un socket SQL que deja de responder.

## Formulario

El formulario toma el fragmento y lo retira de la URL antes de presentarlo. Si no
puede retirarlo o no es canonico, no admite un cambio. Nunca envia el enlace al
montarse. La nueva contrasena exige confirmacion explicita y coincidencia entre
ambos campos; el intervalo se comprueba en bytes antes de enviar.

No almacena tokens ni contrasenas en storage, borradores, parametros de navegacion
ni sesion autenticada. El cliente tampoco adjunta bearer ni cookies a estos POST;
usa `no-store`, rechaza redirecciones y omite referrer. El formulario elimina sus
referencias al token y a ambos campos al iniciar el envio. JavaScript no permite
prometer borrado verificable de las copias internas de strings o del transporte.

Una respuesta perdida puede corresponder a un cambio ya confirmado. Se retiran
los campos y no se ofrece repetir automaticamente el mismo POST: el usuario
puede probar la contrasena nueva al iniciar sesion o solicitar otro enlace. Un enlace nuevo reemplaza al anterior y descarta sus respuestas tardias. Si se
abre durante una sesion activa, se capturan los borradores y se requiere nuevo
acceso antes de recuperarlos. Una respuesta MFA previa al enlace no puede abrir
el despacho ni reemplazar el formulario. No
se suprime ni sustituye el MFA y no se fabrica una sesion a partir del enlace.

## Aceptacion y pendientes

Resultados locales: 14 pruebas de transporte de correo con proveedor simulado,
10 de HTTP aislado, cinco de composicion y presupuesto compartido, cinco de enlace
y once escenarios distintos de navegador controlado en cuatro ejecuciones focales. Incluyen cancelacion HTTP con trabajo retenido,
respuesta incierta, ausencia de envio automatico y errores neutros. No equivalen
a una aceptacion con proveedor real, a la regresion de CI ni a activacion en VPS3.

La aceptacion HTTP compuesta aprobo 1/1 en 13.31 s con PostgreSQL, Redis,
entropia OS, Argon2id y MFA reales; solo la entrega se capturo. Confirmo el
presupuesto compartido, consumo unico, cambio auditado e invalidacion de
credenciales anteriores sin crear sesion. El nuevo acceso exigio el MFA existente.

Antes de activar faltan configuracion operativa explicita y coordinacion del
controlador de restauracion que cierre el acceso durante toda la operacion.
Los limites del driver sincrono deben comprobarse sobre el transporte real:
limitar una future no cancela una llamada SQL/Redis ya iniciada.
