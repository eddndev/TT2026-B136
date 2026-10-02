# Recuperación de contraseña mediante capacidad consumida en PostgreSQL

## Estado

En implementación interna. No compuesta en HTTP, CLI ni servidor, sin envío de
correo ni activación operativa. Véase [el alcance interno](../password-reset-internal.md).

## Contexto

La contraseña, la actividad de la cuenta y su generación de autenticación se
persisten en PostgreSQL. Las sesiones y los desafíos de MFA en Redis incorporan
esa generación. Cambiarla invalida las credenciales anteriores sin depender de
que se enumeren o borren todas sus claves; el mecanismo está descrito en
[la decisión sobre acceso de miembros](0043-member-access-and-authentication-generation.md).

Un código de recuperación de MFA se utiliza después de verificar la contraseña.
No recupera una contraseña olvidada. Guardar la capacidad de recuperación en
Redis y consumirla antes de una actualización SQL introduciría un estado parcial:
capacidad gastada y contraseña anterior todavía vigente. La auditoría tampoco
puede confirmarse después del cambio en una transacción independiente.

El rol de ejecución no puede modificar libremente los hashes de contraseñas ni
leer los digests de recuperación. El guard de miembros impide las sustituciones
de credenciales ordinarias. Añadir el caso de uso requiere una transición
restringida, manteniendo las reglas de actividad, roles, MFA y último Owner.

## Decisión

La aplicación genera una capacidad aleatoria de 32 bytes mediante un puerto
específico. Su digest SHA-256 incluye el prefijo de propósito ASCII
`qadra:password-reset:v1` seguido de un byte cero. Sólo el digest se persiste.
Cada emisión tiene además un UUID independiente, que identifica la emisión sin
servir de credencial. Una cancelación por entrega definitivamente rechazada debe
coincidir con ambos valores; una respuesta tardía no puede cancelar otra emisión.

La vigencia y el cupo por cuenta son políticas internas explícitas. Solicitudes
nuevas no reemplazan enlaces vigentes. Cuenta inexistente, inactiva, limitada o
sin capacidad disponible produce el mismo resultado lógico de solicitud. Este
resultado interno no demuestra todavía uniformidad temporal ni anti-enumeración
del futuro transporte público.

PostgreSQL decide la identidad, generación y tiempo de emisión. El consumo
revalida vencimiento, identidad, actividad y generación después de adquirir los
bloqueos, incluso cuando el hash Argon2id ya fue calculado. Bajo el bloqueo
compartido de mutaciones auditadas, una transacción consume la capacidad,
sustituye el hash, incrementa revisión y generación, y añade el evento a la
cadena de auditoría. Un recibo diferido exige la operación, recurso, actor y
momento exactos antes de confirmar. Un error revierte todas las modificaciones.

Las operaciones SQL restringidas usan el propietario ya existente de las tablas,
relaciones calificadas y `search_path=pg_catalog`. El rol de ejecución obtiene
EXECUTE sólo sobre las operaciones e inventario necesarios; no obtiene DML de
capacidades ni UPDATE de contraseñas. El catálogo valida exactamente cuerpos,
firmas, propietarios, privilegios y triggers. El propietario administrativo
sigue siendo una autoridad de confianza capaz de alterar el esquema.

Una entrega con resultado incierto conserva la capacidad hasta su vencimiento;
no la cancela ni repite automáticamente. El consumo no emite sesión ni cambia
TOTP, códigos de recuperación restantes, rol, asignaciones o material documental.
La persona deberá autenticarse nuevamente con la contraseña nueva y su MFA.

## Consecuencias

El puerto de aplicación y su adaptador pueden verificarse sin exponer rutas
públicas ni enviar secretos a un proveedor. El transporte de correo, las URLs
HTTPS, los límites públicos, la persistencia de entregas y la configuración
operativa requieren aceptación adicional antes de habilitar ese recorrido.

Se conservan los registros de capacidades y sus recibos en esta frontera interna.
No existe todavía una política operativa de retención ilimitada ni un limpiador.
Restaurar un respaldo antiguo puede revivir una capacidad que se consumió después
de la captura. La primitiva administrativa de invalidación exige propietario,
base, esquema y cabeza auditada exactos. Cancela pendientes y registra un recibo
en una transacción, sin cambiar credenciales ni permisos. El UUID estable y su
predecesor permiten reconciliar resultados inciertos sin repetir efectos. Resuelve
el catálogo con `pg_catalog` primero antes de cualquier bloqueo, y no repara
configuraciones o cadenas alteradas. El controlador operativo deberá invocarla
con escritores detenidos y acceso cerrado; esa composición sigue pendiente. Un respaldo
antiguo tampoco contiene una contraseña modificada posteriormente. El despliegue
normal debe respetar la identidad del esquema y no tratar el cambio como un
rollback transparente a un binario que sólo reconoce el guard anterior.
