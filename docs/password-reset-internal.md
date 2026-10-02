# Recuperación de contraseña: frontera interna

## Alcance

El trabajo de `application::identity::password_reset` es interno. No hay rutas,
formulario, flags de servidor ni correo operativo para restablecer contraseñas.
La API de recuperación MFA existente sigue consumiendo un código después de
verificar la contraseña; no debe presentarse como recuperación de contraseña.
La [decisión transaccional](adr/0066-atomic-password-recovery.md) explica el diseño.

## Contrato de aplicación

`PasswordResetService` recibe repositorio, entrega, limitador, fuente de entropía,
hasher SHA-256 y hasher de contraseñas. `ResetPolicy` exige duración positiva
representable y cupo positivo; no tiene valor operativo predeterminado.

- `request(email)` normaliza el correo y consulta un límite independiente del
  contador de contraseñas fallidas. Una solicitud admitida puede emitir una
  capacidad de 32 bytes. El resultado no contiene el token ni distingue entre
  cuenta desconocida, inactiva, limitada o sin cupo.
- Sólo el repositorio elige la cuenta y generación vigentes, los tiempos y el
  identificador independiente `ResetId`. El digest tiene propósito exclusivo de
  recuperación. El enlace nuevo no invalida automáticamente otro enlace vigente.
- La entrega devuelve aceptación, rechazo definitivo o incertidumbre. Sólo el
  rechazo definitivo cancela el ID y digest exactos. Un error o incertidumbre no
  demuestra que el destinatario no recibió el mensaje; no se reenvía ni invalida
  automáticamente la capacidad.
- `complete(token, password)` exige 32 bytes y la política existente de
  contraseñas. Inspecciona provisionalmente la capacidad antes de calcular
  Argon2id. Esa inspección no autoriza la escritura: el repositorio revalida todo
  en la transacción de consumo y devuelve `Changed` o `Rejected`.

Los buffers propios de tokens y la entrada concatenada del digest se borran al
liberarse. El comando de consumo borra su hash al destruirse. El servicio no
reclama poder borrar los buffers del llamador ni las copias internas de drivers.
No se registran tokens, contraseñas ni digests en eventos públicos o auditoría.

## Persistencia y aceptación local

`PostgresPasswordResetRepository` abre el esquema validado con el rol restringido
de ejecución. Las operaciones resuelven la cuenta, generación y tiempos en
PostgreSQL. El consumo bloquea la cadena auditada y las filas implicadas antes de
comprobar la vigencia; sustituye el hash, incrementa revisión y generación y
consume la capacidad en la misma transacción que el evento. El recibo diferido
impide confirmar el cambio sin su evento exacto.

El rol de ejecución no lee ni modifica directamente la tabla de capacidades ni
los hashes. El esquema valida cuerpos, firmas, propietarios, privilegios e
inventario de funciones y triggers. Los errores públicos del adaptador no
incluyen tokens, digests, correos ni detalles del servidor.

La primera campaña PostgreSQL aprobó 19 pruebas en 48.11 s con instancias
propias desechables: consumo único y concurrente, cuotas, generaciones,
expiración después de bloqueos, permisos efectivos, rollback ante fallo de
auditoría y rechazo de alteraciones del catálogo. Otra prueba con Argon2id,
TOTP, protección AES, PostgreSQL y Redis reales aprobó en 11.79 s. Verificó que
la contraseña nueva permite completar el MFA existente, mientras la contraseña,
sesión y desafíos anteriores quedan rechazados por su generación. El cambio
preserva roles, asignaciones y material MFA; no borra las claves Redis para
simular su invalidación. Entrega, entropía y limitador de esa prueba son dobles
explícitos. Otros dos casos aprobaron en 5.07 s: vencimiento después de esperar el bloqueo
de la fila de usuario y ejecución con un esquema vacío de sólo USAGE delante
del esquema válido. El segundo reprodujo primero una invocación en el namespace
equivocado; el adaptador ahora obtiene el namespace de la tabla validada.
Estos resultados focales no sustituyen el cierre global de CI.

## Invalidación administrativa después de restaurar

`invalidate_restored_password_resets` usa una conexión privada del propietario
administrativo. Recibe UUID estable de operación, base, esquema y cabeza auditada
esperados. Fija un `search_path` seguro antes de resolver relaciones o bloquear;
valida nuevamente identidad, permisos, catálogo e inventario después de adquirir
el bloqueo del esquema real. No ejecuta migraciones ni repara un catálogo alterado.

Tras bloquear la cadena auditada y todas las capacidades pendientes, obtiene el
reloj de PostgreSQL. Cancela sólo las pendientes, incluidas las expiradas, y añade
un único recibo auditado en la misma transacción. Una capacidad emitida en el
futuro provoca rechazo completo. Mantiene contraseñas, generaciones, MFA,
pertenencias y recibos consumidos. Una operación sin pendientes también produce
un recibo. Ante resultado incierto se conserva el mismo UUID y predecesor: un
reintento devuelve el recibo original, pero rechaza un predecesor diferente o
nuevas capacidades pendientes.

Doce pruebas focales aprobaron en 46.46 s, incluido un respaldo y restauración
reales en un esquema desechable. Verificaron recibos, reintentos, bloqueos,
rollback, reloj posterior a esperas, permisos, catálogo, cadena y rechazo del
sombreado de funciones a través del `search_path` recibido.

Es una primitiva interna para un controlador administrativo. El llamador debe
mantener detenidos los escritores y cerrado el acceso durante toda la restauración
SQL/Redis. El controlador de despliegue todavía no la invoca, no existe comando
público y no se ha demostrado la recuperación operacional completa. Restaurar SQL
puede recuperar una contraseña antigua; cancelar enlaces no corrige ese efecto.

## Activación pendiente

La activación requiere, además, entrega coherente y configurada, limitación del
transporte público, protección de URLs y secretos, comportamiento ante respuestas
perdidas, retención y coordinación de restauración con bloqueo durable del acceso.
La aceptación de contraseña nueva más MFA existente ya es local; no sustituye
la aceptación del recorrido HTTP ni de la entrega real. El estado detallado de pruebas pertenece al
[informe de verificación](verification-report.md).
