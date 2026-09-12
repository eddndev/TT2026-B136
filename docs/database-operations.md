# Base de datos y migración de documentos

El servidor usa PostgreSQL para usuarios, expedientes, documentos y una sola
cadena de auditoría. Redis conserva las sesiones y controles efímeros. La decisión
está en [ADR-0016](adr/0016-case-document-transactions.md).

## Preparar un despliegue nuevo

Crear previamente una base y un rol de conexión sin privilegios administrativos.
Por ejemplo, desde una sesión de administración PostgreSQL:

```sql
CREATE ROLE tt_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE;
```

Configurar su autenticación en PostgreSQL según el entorno. Ejecutar la migración
con `DATABASE_URL` administrativa; el comando no crea usuarios PostgreSQL:

```bash
cargo build --workspace
export DATABASE_URL='postgresql://administrador@localhost/despacho'
target/debug/despacho-cli database migrate --runtime-role tt_runtime
```

Para `serve`, cambiar `DATABASE_URL` por la del rol operativo. El arranque valida
sus privilegios y no ejecuta DDL. Conservar KEK, certificados, claves y configuración
TSA fuera del repositorio y preparar Redis. Los argumentos criptográficos de
`serve --help` siguen vigentes.

## Migrar un almacenamiento local existente

1. Detener todos los escritores, incluidas versiones anteriores del servidor y
   comandos `audit append` que apunten al archivo compartido. Conservar respaldo
   de la base existente, directorio local, KEK y material criptográfico.
2. Aplicar el esquema nuevo sobre la base que ya contiene usuarios y expedientes.
   No iniciar todavía el servidor: el primer import exige destinos documental
   y de auditoría vacíos para conservar la cadena original como prefijo.
3. Elaborar el mapa explícito. Cada JSON documental debe aparecer exactamente una
   vez y el expediente debe existir en la base. No se infieren asociaciones:

```json
{
  "documents": [
    {
      "document_id": "11111111-1111-4111-8111-111111111111",
      "case_id": "22222222-2222-4222-8222-222222222222"
    }
  ]
}
```

4. Inspeccionar con la KEK original. El modo predeterminado no crea archivos ni
   escribe en la base; valida contexto de cifrado, digest, evidencia y cadena:

```bash
export KEK_BASE64='valor-de-la-kek-original'
target/debug/despacho-cli --json database import \
  --data-dir /ruta/a/runtime-data --mapping /ruta/a/mapping.json
```

5. Revisar conteos, huella de fuentes y cabeza histórica. Aplicar sobre las mismas
   fuentes detenidas y la misma base administrativa:

```bash
target/debug/despacho-cli --json database import \
  --data-dir /ruta/a/runtime-data --mapping /ruta/a/mapping.json --apply
```

6. Conservar fuentes y salida de reconciliación. El import añade un evento propio;
   `audit_entries` en el recibo cuenta solamente entradas históricas. Una repetición
   verifica el recibo y no duplica documentos ni eventos. Si el proceso termina
   después del commit y antes de escribir ambos marcadores, repetirlo recupera
   estos marcadores. Las barreras `.migration-pending` se escriben antes de
   insertar y conservan bloqueados ambos orígenes incluso si falla el commit.
   Un fallo posterior al commit lo indica explícitamente: reparar la causa de
   archivos y repetir las mismas fuentes y mapa. No borrar barreras para volver
   a escribir sobre un origen de estado incierto.
7. Arrancar con el rol operativo y `--data-dir` apuntando al origen preservado.
   El servidor comprueba ambos marcadores, hashes y recibo, y reconcilia documentos,
   asociaciones y cadena completa; rechaza restauraciones parciales. Validar
   roles, expedientes y exportaciones antes de habilitar tráfico.

Los marcadores `.migrated` no son una protección frente a ejecutables antiguos o
un administrador que los borre. No reiniciar escritores antiguos sobre el origen.
Después de empezar a servir, no reutilizar una base vacía ni volver al servidor
antiguo como recuperación: se perderían cambios posteriores al corte.

## Respaldo y restauración

Respaldar PostgreSQL completo, conservar las fuentes originales y proteger KEK,
claves y certificados por separado. Una copia documental sin su KEK no basta.
Crear una base de restauración independiente antes de probar recuperación:

```bash
pg_dump "$DATABASE_URL" --format=custom --file=/ruta/segura/despacho.dump
pg_restore --dbname="$RESTORE_DATABASE_URL" --exit-on-error /ruta/segura/despacho.dump
```

Restaurar también roles/permisos según el procedimiento administrativo del entorno.
Comprobar conteos, contexto y bytes cifrados, evidencia sellada, secuencias y hashes
de auditoría y recibos. Verificar ZIP con OpenSSL y comparar con la exportación
anterior. `scripts/api-demo.sh` incluye un ensayo desechable de importación y
restauración con documentos realmente sellados; no utiliza datos del usuario.

Las sesiones Redis no sustituyen el estado durable. En una recuperación operativa
se deben invalidar sesiones anteriores y ensayar el nuevo acceso con MFA. El
respaldo y los recibos no resuelven por sí solos el anclaje externo de auditoría.
