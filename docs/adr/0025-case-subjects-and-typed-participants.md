# 0025. Identidades representadas y participantes tipificados

## Status

Accepted.

## Context

El directorio descrito en [0021](0021-audited-case-participants.md) conserva
fichas manuales con nombre, rol libre y estado organizativo. Esas fichas sirven
para registrar información inicial, pero no distinguen una persona representada
de los distintos roles que puede ejercer en un expediente. Un nombre coincidente
no establece identidad; un nombre distinto tampoco demuestra que sean personas
diferentes.

Los datos procesales deben permanecer separados de las cuentas que acceden a
Qadra. Registrar a una persona como defensora, imputada o juzgadora no debe
otorgarle permisos de la aplicación. Asimismo, actualizar una identidad no debe
cambiar retrospectivamente una declaración, su soporte o una firma capturada.

## Decision

### Identidad estable dentro del expediente

Se introduce una identidad representada con UUID propio y revisiones inmutables.
Puede representar una persona física o un órgano institucional. No existe un
directorio global que permita buscar identidades entre expedientes.

Cada identidad registra su nombre y un identificador declarado cuando se conoce.
Para personas físicas, la CURP se valida por forma; ello no acredita identidad
civil ni consulta un registro externo. Un dato desconocido requiere una razón
explícita. Una persona sin nombre conocido conserva una etiqueta y el motivo de
la falta de identificación; esa etiqueta no cuenta como coincidencia de nombre
conocido.

La identidad requiere soporte documental cifrado, identificado por documento,
versión, SHA-256 y localizador legible de página o sección. El localizador es una
declaración del operador, no una extracción automática ni una URL externa.

### Roles con datos propios

Cada ficha tipificada vincula una revisión exacta de identidad y un rol entre
imputado, víctima, defensor, fiscal, asesor de víctima, juez de control, tribunal
de juicio, perito, policía, supervisor de medidas cautelares y otro. Cada variante
admite solamente sus campos definidos. El tribunal de juicio representa un
órgano institucional; los roles personales requieren una persona física. Otro
admite ambos tipos con etiqueta y descripción declarada.

Licencia profesional, adscripción, custodia, contacto y protección permanecen
como datos declarados con estados explícitos. El dato de contacto o protección
documentado remite a soporte cifrado autorizado. No se incorpora información
personal de esos soportes al índice general de participantes.

La combinación de expediente, identidad y tipo de rol es única entre las fichas
vigentes, incluidas las archivadas. Archivar no permite duplicar el mismo rol.
Cambiar de rol conserva la historia y libera el anterior para una nueva ficha.
Una ficha tipificada no puede cambiar su UUID de identidad representada.

### Revisión explícita de posibles duplicados

La aplicación genera candidatos por nombre conocido, identificador declarado,
huella del certificado público y pareja de digest documental y localizador.
Usar sólo el digest confundiría a personas diferentes descritas en el mismo
documento. También se consideran las fichas manuales existentes.

El operador selecciona una identidad existente o justifica una nueva. Debe
explicar, con soporte, por qué los demás candidatos representan identidades
distintas. No se fusionan identidades automáticamente. El resultado sin
candidatos no acredita ausencia de duplicados en la realidad. El límite de 16
candidatos es una restricción de capacidad y no debe eludirse cambiando datos.

La revisión incorpora una huella del directorio completo del expediente. Se
calcula con un recorrido ordenado y memoria acotada de las cabeceras actuales de
identidades y fichas. Una modificación concurrente invalida la revisión, incluso
si ocurrió fuera de la página visible en Qadra.

### Evidencia, concurrencia y consultas

Las revisiones de identidad y ficha se conservan en la misma transacción que la
auditoría. La sesión se vuelve a comprobar antes de entrar a persistencia. Bajo
el bloqueo común se vuelven a comprobar permisos,
membresía, apertura del expediente, revisiones esperadas, unicidad, revisión de
identidad y soportes capturados. Una preparación no reserva UUID ni persiste
borradores incompletos.

La codificación de identidad usa `SUBJ1`; la ficha tipificada usa `PART2` y
contiene el UUID, revisión y digest de la identidad vinculada. `PART1` permanece
inalterado para la historia manual. La revisión explícita se codifica con
`PREV1` y la operación completa con `PTXN1`. Los enteros tienen tamaño fijo y
orden de bytes definido; el texto conserva UTF-8 sin normalizar su interior.

El conjunto de soportes distintos de una operación está limitado a dos versiones,
16 MiB por versión y 32 MiB en total. La validación del formato y de la evidencia
capturada se realiza antes de aceptar la mutación. Los límites de soporte se
aplican al conjunto, incluidas las decisiones sobre candidatos.

Los listados seleccionan primero la revisión vigente de cada ficha manual o
tipificada y luego aplican filtros y paginación. Sólo el detalle autorizado
expone campos sensibles. El nombre mostrado en una ficha histórica procede de
su revisión exacta de identidad; consultar la identidad actual es otra acción.
Completar una ficha manual agrega la primera revisión tipificada sin reescribir
su historia. Después de tipificarla, no se permite volver a la edición manual.

### Credenciales internas y resultados inciertos

Defensores y jueces de control requieren la declaración interna descrita en
[0026](0026-internal-participant-declarations.md). Esa evidencia no equivale a
FIREL, no acredita una profesión y no es la firma personal de una cuenta de
Qadra. La autenticación de cuentas y su firma personal siguen siendo flujos
distintos.

La preparación devuelve el digest de operación para cambios sin firma. Cuando
se necesita firma, primero se entrega la declaración exacta y posteriormente
se verifica la firma externa. La revisión guardada conserva el origen de la
operación y, cuando exista, el de su credencial. Cambiar sólo el estado del
directorio no afirma que esa firma cubra el nuevo estado.

Tras una respuesta perdida, Qadra consulta la revisión exacta y comprueba el
origen pertinente antes de atribuir éxito al envío. La edición separada de
identidad permite consultar la revisión resultante y revisarla, sin atribuir
autoría a ese envío por mera igualdad de valores. No hay reintento automático.

## Consequences

- Se conservan las fichas manuales y las evidencias históricas, con migración
  explícita por ficha cuando el operador completa el perfil.
- Aumentan las revisiones y validaciones necesarias para registrar participantes.
  Esto permite detectar conflictos sin sobrescribir evidencia ni inferir identidad.
- No se resuelven retrospectivamente identidades estables duplicadas por una
  fusión automática. La coincidencia documental y la revisión humana siguen
  teniendo límites que deben quedar visibles.
- Los perfiles y credenciales representadas no alteran el control de acceso de
  Owner, Litigator, Paralegal o Client.
