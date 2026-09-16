# Contrato de aplicacion para hechos declarados

## Estado implementado

El modulo `crates/application/src/procedural_facts/` implementa comandos,
consultas acotadas, seleccion de fuentes y comprobaciones puras de identidad,
estado, administracion y fichas de participantes. Define puertos y estructuras
para preparar y confirmar operaciones. **Todavia no implementa un servicio que
los coordine, un adaptador PostgreSQL, recibos canonicos, rutas HTTP o Qadra.**
Los permisos nuevos son decisiones de rol probadas, no un flujo autorizado ya
expuesto. El [informe de verificacion](verification-report.md) distingue este
corte de las entregas anteriores.

Los valores y los canones PFRES1/PFNOT1 siguen el [contrato de dominio](procedural-facts.md).
La motivacion esta en [ADR-0031](adr/0031-declared-procedural-facts.md).

## Comandos e identidad

`FactChange<V>` distingue alta, correccion motivada y retiro administrativo.
Alta espera ausencia y produce revision 1. Correccion y retiro exigen revision
positiva y producen su sucesora; u32::MAX devuelve agotamiento, nunca wrap.
Retiro no acepta valores nuevos y es terminal para esa raiz. No acredita
nulidad ni modifica otras declaraciones, personas o documentos.

`NotificationCommand` comprueba que los nuevos valores seleccionen una revision
de su padre declarado. `validate_fact_base` tambien contrasta contra la base
almacenada el expediente y el `FactTarget` completo: familia, UUID y padre fijo.
Esto impide cambiar ambos padres del comando y sus valores para eludir la
identidad almacenada. La comprobacion se aplica igualmente al retiro.
Seleccionar otra revision del mismo padre o corregir personas si esta permitido.

La validacion de base rechaza alta sobre raiz existente, base ausente, revision
obsoleta y una raiz retirada. No consulta almacenamiento ni comprueba recibos,
permisos o unicidad de operacion. El servicio debe verificar el contenido de la
base y el adaptador debe repetir la comprobacion contra la cabeza bajo bloqueo.

## Seleccion exacta y material interno

`FactSourceSelection::from_values` obtiene solo referencias declaradas:

| Fuente inmediata | Resolucion | Notificacion |
| --- | --- | --- |
| Padre resolucion | Ninguno | Una revision exacta |
| Fichas personales | Ninguna | Hasta cuatro referencias exactas |
| Resultados de audiencia | Hasta uno | Hasta dos selecciones, con acuerdo opcional |
| Documentos directos | Hasta uno | Hasta dos versiones exactas |

Las cuatro funciones personales son destinatario, receptor, representado y
representante. `Unknown` y `Unlinked` no se convierten en fichas. El orden es
UUID y revision; se deduplica exclusivamente la referencia completa. Se conservan
dos revisiones del mismo participante, dos acuerdos del mismo resultado y dos
versiones del mismo documento, incluso cuando sus digests coinciden. Ausencia
de acuerdo no equivale al UUID cero. Los valores originales conservan todos los
localizadores y funciones cuando la consulta de una fuente se deduplica.

`FactSourceMaterial` contiene snapshots exactos resueltos por el servidor:
resolucion padre opcional, hasta cuatro `ParticipantDetail` y hasta dos
`HearingResultSnapshot`. Dos acuerdos de una misma revision comparten material,
pero se comprueban por separado. La resolucion padre aporta su snapshot, un
resultado opcional y su soporte admitido opcional; no contiene otro padre ni
expande el grafo recursivamente. Estas cotas son obligaciones del futuro
adaptador y validador completo; construir un `Vec` no las garantiza.

El lote `records` contiene exclusivamente los documentos directos. Ningun
antecedente agrega sus documentos a ese lote. Las futuras preparaciones de alta
y correccion deben admitir el lote completo con los limites compartidos de
16 MiB por archivo y 32 MiB en total. Un documento usado directamente se admite
aunque tambien figure entre antecedentes. Retiro conserva fuentes y admisiones
anteriores y exige un lote vacio.

## Comprobaciones de participantes

`resolve_fact_participants` compara el material con toda la seleccion exacta,
rechaza faltantes, sobrantes, duplicados, expedientes o revisiones diferentes,
y ordena el resultado. Para una ficha manual recomputa su canon y exige ausencia
de sujeto tipado. Para una ficha tipada recomputa el canon de ficha y sujeto,
comprueba expediente, UUID, revision y digest del sujeto, y compatibilidad de
su clase con el perfil. Se permiten revisiones historicas inactivas; su seleccion
no afirma elegibilidad juridica.

La salida combina `FactParticipantSnapshot` con un `ParticipantOverview`
derivado de esos valores. No expone el sujeto completo, CURP ni todos sus datos.
Esta comprobacion no verifica una credencial criptografica, admite archivos ni
prueba existencia en una base de datos. El adaptador debe cargar las fuentes
reales del mismo expediente y el servicio debe verificar la respuesta completa.

`FactSourceViews` reserva proyecciones legibles de resolucion, participantes y
resultados. La derivacion y comprobacion de vistas de resoluciones y resultados
siguen pendientes; no deben aceptarse textos arbitrarios como evidencia.

## Administracion observada

`validate_fact_administration` comprueba expediente y canon de una revision
administrativa registrada. Una base `Unrevised` conserva sus metadatos originales:
no fabrica revision, digest persistido, autor ni hora y no exige perfil penal
completo o etapa para declarar un hecho externo. Como ese valor no contiene UUID
de expediente, el adaptador debe resolverlo dentro del expediente autorizado.

Una administracion actual cerrada impide cambios. Con captura anterior, se
rechaza retroceso de revision o contradiccion de una misma revision inmutable,
incluidos autor y hora. Dos bases sin revision deben coincidir. Se permite pasar
de base sin revision a revision registrada, o avanzar a una revision posterior.
La observacion nunca se convierte en CAS administrativo: una actualizacion
administrativa valida no invalida por si sola el comando. El estado historico
capturado no reemplaza el estado vigente para decidir si se puede escribir.

Esta funcion es una comprobacion de preparacion de cambios, no un verificador
para denegar lecturas historicas de expedientes cerrados.

## Puertos pendientes de implementar

`ProceduralFactStore` fija estas obligaciones por operacion:

- Autorizacion vigente: Owner en todos los expedientes, Litigator asignado para
  lectura y gestion, Paralegal asignado solo lectura; Client denegado.
- Listados de cabezas con filtro de estado antes de paginacion: 1..100 filas,
  cursor UUID exclusivo. Historial descendente: 1..20 revisiones y cursor positivo.
- Preparacion sin reservar identidad ni revision; material exacto y batch directo
  acotados antes de decodificar o admitir contenido.
- Transaccion comun con auditoria para raiz, revision y recibo. Bajo bloqueo,
  repetir identidad, membresia, expediente activo, cabeza, operacion unica y
  correspondencia de todos los documentos y fuentes preparados. Capturar Clock
  y administracion en esa frontera.

`ProceduralFactWorkflow` exige autenticacion previa a consultas y preparacion,
reautenticacion del mismo actor despues de admision y comparacion del digest de
envio antes de confirmar. Las estructuras `FactReceipt` y `PreparedFactChange`
no definen todavia un canon ni prueban que esas operaciones hayan ocurrido.

Faltan el canon de fuentes y recibos, coordinacion del servicio, validacion total
de la union de fuentes y de las proyecciones retenidas, persistencia y migraciones,
pruebas de aislamiento/revocacion/concurrencia/rollback, restauracion, HTTP y
recorrido de Qadra. No se habilitan calculos juridicos, recursos o alertas con
estos contratos y comprobaciones puras.
