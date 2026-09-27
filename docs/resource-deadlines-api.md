# Creacion contextual de plazos

Desde las actividades de un recurso, Owner o Litigante asignado pueden preparar
y confirmar un nuevo plazo con su asociacion. Paralegal y Cliente no gestionan
este flujo. La confirmacion exige expediente y recurso activos y conserva las
mismas comprobaciones de perfil, fuente, calendario y responsable que el alta
ordinaria de plazos.

## Rutas

Base: `/api/v1/cases/{case_id}/procedural-resources/{resource_id}/activities/deadlines`.

- `POST /prepare`: prepara el plazo y el vinculo; no crea ninguno.
- `POST /submit`: confirma ambos en una transaccion o no registra ninguno.

Ambas rutas requieren bearer, rechazan filtros y propiedades desconocidas,
aceptan un objeto JSON de hasta 1 MiB y devuelven `Cache-Control: no-store`.
Los identificadores de los padres en el cuerpo deben coincidir con la ruta.

## Comando y preparacion

El comando contiene `case_id`, `resource_id`, `association_id`,
`expected_resource_revision`, `resource`, `act` y `deadline`.

- `resource` selecciona `{id, revision, capture_digest}` historicos.
- `act` es obligatorio y nullable; cuando existe contiene
  `{id, revision, resource_revision, capture_digest}`. Identifica el acto y la
  revision exacta del recurso que lo conserva.
- `expected_resource_revision` es la cabeza actual revisada, independiente de
  la captura historica seleccionada.
- `deadline` es el comando humano existente de [plazos](deadlines-api.md),
  exclusivamente `register` con revision esperada cero y politicas explicitas.
  Su `operation_id` identifica ambas escrituras. La interfaz genera los UUID.

La preparacion devuelve el comando normalizado, `deadline` con su calculo y
recibo propuestos, `association` con las capturas de recurso/acto, actor,
administracion y cabeza observadas, y `submission_digest` conjunto. La
asociacion propuesta refiere al ID, revision 1 y digest de captura del plazo;
no contiene un detalle historico ni fecha de registro ficticios.

El recurso/acto aportan contexto organizativo, no una fecha de vencimiento.
La persona selecciona la fuente temporal admitida por el perfil y declara
su calificacion. Un calculo bloqueado conserva sus motivos y ausencia de
vencimiento; no se sustituye por una fecha manual.

## Confirmacion y repeticion

`/submit` recibe `{command, expected_submission_digest}` y devuelve 201 con
`{deadline, association, submission_digest}`. Los dos detalles son revisiones
reales y contienen sus recibos originales. La captura del plazo en la
asociacion coincide exactamente con el plazo registrado.

Se vuelven a comprobar autorizacion, pertenencia, estado y cabezas bajo el
bloqueo de auditoria. El cambio de una dependencia, recurso archivado,
expediente cerrado, cuenta revocada o recibo distinto impide ambas escrituras.
El fallo al insertar la asociacion o su auditoria tambien revierte el plazo.

Una repeticion autorizada exacta devuelve los mismos registros, sin otra
revision. Exige los dos recibos y el marcador auditado de creacion conjunta;
una operacion ordinaria con identidad coincidente produce conflicto. La
repeticion exacta puede consultar un resultado ya confirmado aunque el
expediente se haya cerrado; conserva la autorizacion vigente.

Ante respuesta incierta, Qadra consulta ambas revisiones originales. Una pareja
coincidente solo prueba las capturas y recibos individuales, no su origen
conjunto. La interfaz conserva el resultado incierto y ofrece **Confirmar origen
del envio**: esta accion explicita repite el mismo comando y digest por
`/submit`, sin regenerar identidades, para que el servidor verifique el marcador.
Solo su respuesta valida confirma el conjunto; un conflicto conserva la
incertidumbre. No se envia una escritura automaticamente al consultar.

Si ambas revisiones estan ausentes, se ofrece el reintento exacto explicito solo
mientras el expediente permita nuevas escrituras. Un registro aislado o una
pareja diferente no habilitan el reintento ni acreditan exito.

## Lecturas posteriores

Las rutas ordinarias de plazos, actividades de recurso, historial y agenda
conservan sus contratos. La proyeccion actual puede evolucionar mediante
correccion o reevaluacion sin modificar la captura historica del vinculo.
Desvincular no retira el plazo. La creacion contextual no implementa activacion
automatica, reglas juridicas adicionales ni una audiencia propia del recurso.

La decision esta en [ADR 0057](adr/0057-atomic-resource-deadline-creation.md).
La evidencia nueva se registra por separado en el informe de verificacion.
