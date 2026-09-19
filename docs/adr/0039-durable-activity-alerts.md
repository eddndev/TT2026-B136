# ADR-0039: Alertas durables de actividad y entrega de correo

## Status

Aceptada para la implementación. La aceptación integral y la integración siguen
pendientes; disponer de una fila programada no acredita entrega de una alerta.

## Context

La agenda de [ADR-0038](0038-authorized-combined-agenda.md) permite consultar
audiencias y vencimientos operativos. Un recordatorio debe conservar su identidad
tras un reinicio, comprobar acceso y vigencia antes de divulgar información, y
distinguir lectura del aviso, atención declarada y entrega al proveedor de correo.

Una revisión técnica puede cambiar el recibo de un plazo sin cambiar su fecha.
Utilizar toda revisión como clave produciría avisos repetidos. Una fecha histórica
tampoco acredita un vencimiento vigente mientras se revisan sus dependencias.

## Decision

Las preferencias pertenecen a la cuenta autenticada. Cada familia permite hasta
ocho anticipaciones distintas de 1 a 720 horas; los valores iniciales son 48 y 24.
Los canales interno y correo se configuran por tipo de aviso. Una lista vacía
desactiva las anticipaciones de esa familia. Los avisos tienen cuatro motivos:
proximidad, vencimiento sin atención declarada, revisión humana requerida y cambio
de fecha que afecta una ventana de 48 horas.

Las horas de anticipación son tiempo transcurrido sobre un instante ya calculado;
no son un segundo cómputo jurídico. Su ventana incluye el umbral y excluye el
vencimiento, conservando nanosegundos. Una puesta en marcha tardía genera sólo el
umbral pendiente más próximo ya cruzado y conserva los demás como sustituidos.
Después del vencimiento se evalúa el aviso de falta de atención. Leer un aviso
no declara atención ni altera el plazo.

El responsable vigente del plazo recibe sus avisos si sigue activo y autorizado.
Las audiencias se dirigen a los miembros activos del expediente con permiso de
lectura; esto incluye Owner cuando es miembro. El rol Owner no suscribe todas
las audiencias ni concede acceso a bandejas ajenas. No se usan participantes
procesales o direcciones históricas como destinatarios de cuentas. Client queda
denegado mientras no exista una política de recurso que lo habilite.

PostgreSQL conserva preferencias con revisión y operación idempotente, planes,
episodios, bandeja, intentos y progreso de recorridos acotados. El bloqueo común
de auditoría ordena la comprobación de evidencia, autorización y escrituras.
La activación de bandeja, creación de entrega y avance del recorrido confirman
juntos. Las lecturas vuelven a comprobar permisos; no basta el destinatario de
la fila. El cierre organizativo no suspende términos ni borra avisos.

Una ocurrencia de proximidad se identifica por recurso, instante UTC exacto y
anticipación; destinatario y canal completan su unicidad. Cambios de nota, desfase
representacional o preferencias no vuelven a entregar esa ocurrencia. Revisión
requerida y falta de atención tienen episodios durables; aceptar o atender cierra
el episodio correspondiente. Un cambio posterior puede abrir otro episodio.
El cambio de fecha se vincula a la transición comprobada, sin convertir toda
dependencia cambiada en una decisión humana pendiente.

Los escritores invalidan planes afectados dentro de su transacción. El escáner
reconcilia altas, preferencias y cambios con avance durable y presupuesto de
candidatos y destinatarios. Antes de activar o entregar se vuelve a comprobar
la evidencia actual; no se espera a que la cola de reevaluación quede vacía ni
se utiliza un cálculo antiguo como fecha operativa.

El envío externo ocurre fuera del bloqueo PostgreSQL. Un arrendamiento temporal
identifica el intento y evita que una confirmación antigua complete otro intento.
La clave idempotente y el mensaje se congelan al primer intento. Resend es el
adaptador seleccionado: su [contrato de idempotencia](https://resend.com/docs/dashboard/emails/idempotency-keys)
conserva claves durante 24 horas. El reintento de una respuesta incierta queda
acotado a una ventana menor y usa exactamente la misma clave y cuerpo. Agotada
esa ventana se exige conciliación; no se crea una clave nueva automáticamente.
Una aceptación del proveedor no se presenta como lectura del correo.

El correo contiene un aviso genérico y una URL fija de acceso a la aplicación,
sin título, partes, fechas ni identificadores de expediente. Una revocación
anterior a autorizar el intento lo impide; una posterior no puede deshacer una
petición externa, pero el mensaje no divulga datos del recurso y abrir la
aplicación requiere autorización vigente. Sin transporte configurado se muestra
el canal deshabilitado; no se inventa una entrega satisfactoria.

## Consequences

El estado de lectura, resolución y correo debe mostrarse por separado. Los
errores y respuestas inciertas conservan evidencia y no autorizan reenvíos
arbitrarios. La captura de correo mediante un servidor de prueba comprueba el
transporte; la recepción en un proveedor externo necesita su propia evidencia.

La administración de bandejas, generación periódica, permisos concurrentes,
recuperación, restauración y navegación Qadra requieren aceptación conjunta.
Las primitivas temporales aisladas no completan ese flujo ni califican perfiles
jurídicos, activación de plazos o usabilidad con personas.
