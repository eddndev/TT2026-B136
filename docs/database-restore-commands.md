# Comandos de base de datos para una restauración controlada

El CLI ofrece dos operaciones para verificar una base restaurada y cancelar sus
capacidades de recuperación pendientes. Son piezas administrativas: no restauran
archivos, detienen servicios, limpian Redis ni habilitan el correo público.
El controlador operacional con bloqueo durable del acceso sigue pendiente.

La frontera transaccional se describe en la
[decisión sobre recuperación](adr/0066-atomic-password-recovery.md) y en el
[contrato interno](password-reset-internal.md). Los comandos existentes
`database migrate` y `database import` conservan sus funciones separadas.

## Validación con el rol de ejecución

Con `DATABASE_URL` ya suministrada mediante un entorno privado y apuntando al
rol restringido de ejecución:

```bash
despacho-cli database check --json
```

El resultado satisfactorio es un único objeto:

```json
{"validated":true}
```

`check` usa la validación completa de inicio de PostgreSQL: esquema, funciones,
propietarios, permisos efectivos e inventarios de las familias persistidas.
Mantiene los bloqueos de validación durante esa comprobación y los libera antes
de terminar. No construye adaptadores de trabajo ni inicia tareas en segundo
plano. La operación funciona con `default_transaction_read_only=on`.

La conexión debe usar el rol runtime aceptado por ese esquema. Una conexión del
propietario administrativo se rechaza; no sustituye una prueba de los permisos
restringidos. También se rechazan catálogos, privilegios o inventarios alterados.
El comando no aplica DDL, concede privilegios ni repara una base antigua.

Un resultado válido describe el estado observado bajo los bloqueos. No prueba la
compatibilidad del paquete, integridad de archivos privados, persistencia Redis,
disponibilidad HTTP ni corrección de cambios posteriores. No autoriza por sí
solo reabrir el acceso después de restaurar.

## Cancelación administrativa de capacidades pendientes

Antes de invocar la cancelación, el operador debe mantener detenidos todos los
escritores y cerrado el acceso durante la restauración SQL/Redis completa.
El comando no crea ni comprueba un bloqueo durable de servicios. Sus bloqueos SQL
protegen la transacción; no sustituyen esa coordinación operacional.

La conexión `DATABASE_URL` debe corresponder al propietario administrativo
autorizado de la base restaurada. El proceso recibe la conexión por el entorno,
sin flags de URL, contraseña o concesión de permisos. No coloque secretos en los
argumentos de ejemplo ni registre el entorno del proceso.

Prepare y conserve antes del primer intento:

| Argumento | Valor requerido |
| --- | --- |
| `--operation-id` | UUID canónico en minúsculas, con guiones y distinto de cero; identifica esta operación. |
| `--expected-database` | Nombre exacto y no vacío de la base restaurada. |
| `--expected-schema` | Nombre exacto y no vacío del esquema de aplicación. |
| `--expected-empty-audit` | Afirmación explícita de que la cadena original está vacía. |
| `--expected-audit-sequence` | Secuencia original desde cero, entre 0 y 9223372036854775807. |
| `--expected-audit-head` | Digest SHA-256 original, exactamente 64 caracteres hexadecimales en minúsculas. |

Use `--expected-empty-audit` **o** ambos argumentos de cabeza, nunca una mezcla.
La base, el esquema y el predecesor deben proceder del conjunto restaurado
verificado. El CLI no los descubre para suplir entradas ausentes ni genera el
identificador de operación.

Para una cadena original no vacía, las variables siguientes representan valores
no secretos previamente verificados y conservados:

```bash
despacho-cli database invalidate-restored-password-resets \
  --operation-id "$RESTORE_OPERATION_ID" \
  --expected-database "$RESTORED_DATABASE" \
  --expected-schema "$RESTORED_SCHEMA" \
  --expected-audit-sequence "$ORIGINAL_AUDIT_SEQUENCE" \
  --expected-audit-head "$ORIGINAL_AUDIT_HEAD" \
  --json
```

Para una cadena original vacía:

```bash
despacho-cli database invalidate-restored-password-resets \
  --operation-id "$RESTORE_OPERATION_ID" \
  --expected-database "$RESTORED_DATABASE" \
  --expected-schema "$RESTORED_SCHEMA" \
  --expected-empty-audit \
  --json
```

La primitiva comprueba el destino, propietario, catálogo, permisos e inventario,
y verifica la cadena completa. Bloquea las capacidades pendientes antes de leer
el reloj de PostgreSQL. En una sola transacción cancela las filas sin consumo ni
cancelación anteriores, incluidas las expiradas, y añade su recibo auditado. Una
capacidad emitida en el futuro provoca rechazo de toda la operación.

Se conservan contraseñas, revisión y generación de usuarios, MFA, pertenencias,
filas consumidas y cancelaciones anteriores. Una operación sin pendientes añade
también un recibo, con cantidad cero. No se ejecutan migraciones ni se reparan
catálogos, permisos o cadenas alterados.

## Recibo e incertidumbre

La respuesta JSON contiene exactamente estos campos; los valores son ilustrativos:

```json
{
  "operation_id": "01234567-89ab-4def-8123-456789abcdef",
  "applied": true,
  "invalidated": 4,
  "audit_sequence": 1,
  "audit_head": "abababababababababababababababababababababababababababababababab"
}
```

- `applied=true`: este intento confirmó la cancelación y el evento.
- `applied=false`: se reconcilió un recibo ya confirmado con ese identificador y
  predecesor. La cantidad, secuencia y cabeza corresponden al evento original.
- `invalidated`: cantidad cancelada por la operación original, no un contador
  recalculado en el reintento.

El evento usa actor `database-restore`, acción
`identity.password_reset_restore_invalidated` y recurso
`reset-capabilities:restore:<operation-id>:cancelled:<count>`.
No contiene contraseñas, tokens ni digests de capacidades individuales.

Un error o una respuesta perdida puede ocurrir después del commit. Mantenga
cerrado el acceso y conserve la solicitud original completa. Para reconciliar,
repita esa solicitud sobre la misma base restaurada; no sustituya su predecesor
por la cabeza actual ni genere otro UUID. Un identificador reutilizado con otro
predecesor se rechaza. Si aparecieron nuevas capacidades pendientes después del
recibo, el reintento también se rechaza sin cancelarlas.

El código de salida es 0 en éxito, 2 para argumentos inválidos y 1 para errores
de configuración, autoridad, validación o almacenamiento. Un rechazo no imprime
un recibo en stdout. Los diagnósticos operativos no permiten distinguir un
rollback de un commit cuyo reconocimiento se perdió; no habilitan reintentos
automáticos con parámetros nuevos.

## Alcance de verificación y trabajo operacional pendiente

Las pruebas del binario están en
[database_restore_cli](../crates/bin/tests/database_restore_cli/main.rs).
Reutilizan el PostgreSQL desechable de infraestructura y fallan si no se configura
`CASE_TEST_DATABASE_URL`; no cuentan una omisión como aceptación. Comparan estado
privado y catálogo sin imprimirlos, verifican el recibo contra la primitiva y
ejecutan `check` con una conexión runtime de transacciones de solo lectura.

Estas pruebas no equivalen a una restauración completa del despliegue. Quedan la
admisión de un respaldo compatible, bloqueo durable de servicios, recuperación
tras fallos del controlador, invalidación de sesiones y desafíos Redis y
verificación del conjunto antes de reabrir acceso. Cancelar enlaces tampoco
reconstruye cambios de contraseña posteriores a la captura ni incrementa las
generaciones para compensar una restauración antigua. El remitente, origen y
activación de la recuperación pública siguen siendo decisiones explícitas.
