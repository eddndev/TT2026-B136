# 0057. Creacion conjunta de plazo y asociacion a recurso

## Estado

Aceptado para implementacion.

## Contexto

Las asociaciones organizativas existentes vinculan recursos y actos historicos
con actividades que ya estan registradas. Crear primero un plazo y despues
su asociacion en dos peticiones permite que la segunda falle y deje una
actividad que no corresponde a la confirmacion revisada por la persona.

El recurso y su acto no son por si mismos una fuente temporal del calculador.
La creacion contextual debe conservar el perfil, la fuente temporal exacta,
el calendario, la calificacion y las politicas de seguimiento ya existentes.
No puede inferir aplicabilidad juridica desde un titulo o una fecha libre.

## Decision

Incorporar un caso de uso de alta contextual con preparacion y confirmacion
conjuntas. Reutilizar las tablas, canon y validaciones de plazos y asociaciones.
Un unico bloqueo y una transaccion auditada vuelven a comprobar actor,
pertenencia, expediente activo, recurso activo, revisiones y fuentes antes de
insertar ambos registros y sus eventos. Cualquier fallo revierte el conjunto.

Conservar la captura historica elegida del recurso y acto separada de la cabeza
actual esperada. El destino propuesto es la primera revision del nuevo plazo,
identificada por su digest de captura. La vista previa no inventa un instante
de registro ni un detalle persistido del destino.

La identidad de operacion del plazo se utiliza tambien en la asociacion. El
recibo conjunto es SHA-256 de los bytes ASCII `RDLTX1`, seguidos por los 32 bytes
del digest de envio del plazo y los 32 bytes del digest de envio de la
asociacion, en ese orden. Ambos recibos mantienen sus formatos independientes.

Un evento adicional de creacion conjunta liga actor, expediente, recurso,
plazo, asociacion, operacion y digest compuesto dentro de la misma transaccion.
La repeticion exige ambos recibos originales y ese marcador exacto; no adopta
operaciones ordinarias independientes aunque reutilicen las mismas identidades.
La interfaz comprueba ambas capturas ante una respuesta incierta, pero dos
recibos coincidentes no acreditan el marcador conjunto. Conserva la incertidumbre
hasta que la persona confirme explicitamente el mismo envio por el endpoint
compuesto. Esa repeticion valida el marcador en el servidor y devuelve ambos
registros originales, incluso despues de un cierre administrativo si conserva
la autorizacion. Un conflicto no se presenta como exito ni genera identidades
nuevas. No se reenvian escrituras automaticamente ni operaciones ordinarias
separadas. La ausencia de ambos registros solo habilita un reintento explicito
cuando el expediente permite nuevas escrituras; un resultado parcial no basta.

## Consecuencias

El plazo es visible en su modulo y, cuando tiene vencimiento operativo, en la
agenda. Las alertas mantienen su mecanismo existente. Una correccion posterior
no reescribe la captura historica de la asociacion; desvincular no retira el
plazo ni cancela sus efectos de seguimiento.

La creacion contextual es una decision explicita del usuario. No completa la
activacion juridica automatica, el corpus aplicable ni el catalogo de audiencias
de recursos. No introduce una fuente temporal nueva. El contrato y las
condiciones de concurrencia se describen en `docs/resource-deadlines-api.md`.
