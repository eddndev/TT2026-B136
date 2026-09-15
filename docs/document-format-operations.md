# Operación de la validación documental

La política de admisión de soportes procesales se describe en
[ADR-0024](adr/0024-isolated-document-format-admission.md). El entorno soportado
es Linux x86_64. El servidor requiere una biblioteca qpdf 12.4.1 y realiza una
comprobación real en el worker antes de abrir el puerto HTTP.

## Preparar la biblioteca

El instalador usa Python 3 y una caché del usuario; no requiere privilegios de
administración. Descarga el artefacto oficial y verifica el SHA-256 fijado antes
de extraer o ejecutar contenido. La salida estándar contiene únicamente la
ruta absoluta de la biblioteca. No borrar sus bibliotecas acompañantes.

```bash
export DOCUMENT_QPDF_LIBRARY="$(bash scripts/setup-document-formats.sh)"
cargo run --bin despacho-cli -- serve --help
```

Se puede elegir la caché con `TT_DOCUMENT_FORMATS_CACHE`. Para una preparación
sin descarga, `TT_QPDF_ARCHIVE` apunta al ZIP obtenido previamente; se exige
el mismo checksum. Un archivo con otro hash falla, aunque el nombre coincida.

| Campo | Valor fijado |
| --- | --- |
| Versión | 12.4.1 |
| Artefacto | `qpdf-12.4.1-bin-linux-x86_64.zip` |
| SHA-256 | `db9122e88ec00c76ac6a14e09ffb92406db1773d47b968911ff6e69f28c09bf9` |
| Biblioteca | `lib/libqpdf.so.30.4.1` |
| Origen | [Publicación oficial](https://github.com/qpdf/qpdf/releases/tag/v12.4.1) |

El argumento `--qpdf-library` puede sustituir la variable
`DOCUMENT_QPDF_LIBRARY`. Debe señalar una instalación administrada por el
operador y protegida contra escritura de usuarios no autorizados. El nombre
y versión declarados por una biblioteca no sustituyen la comprobación del
artefacto y la protección de su directorio.

Si el instalador o la comprobación de arranque falla, corregir la preparación
antes de abrir tráfico. No sustituir la validación por una respuesta de éxito
ni eliminar la verificación de versión para hacer pasar un documento.

## Ejecutar las pruebas

Las pruebas nativas requieren una biblioteca real. Una variable ausente no
convierte sus pruebas en omisiones silenciosas:

```bash
export TT_TEST_QPDF_LIBRARY="$(bash scripts/setup-document-formats.sh)"
cargo test -p infrastructure --lib document_formats
cargo test -p despacho-cli --test document_format_worker
bash scripts/test-backends.sh
bash scripts/api-demo.sh
bash scripts/web-demo.sh
```

`test-backends.sh` prepara la biblioteca cuando la variable no está definida.
Los guiones API y navegador la configuran también para el servidor. Las
sesiones PostgreSQL/Redis son desechables, según las instrucciones de
[operación de base de datos](database-operations.md).

## Límites y diagnóstico

Cada operación valida hasta dos soportes distintos. El worker recibe como
máximo 16 MiB por archivo, con 5 segundos de CPU, 256 MiB de espacio de
direcciones y 10 segundos de tiempo transcurrido. Los DOCX comparten además
64 MiB de expansión real y un millón de eventos XML. Una máquina cargada puede
agotar el límite de tiempo aunque el archivo sea estructuralmente admisible.

El validador devuelve resultados acotados. Los errores HTTP no incorporan
fragmentos de contenido, mensajes del parser nativo ni rutas de documentos.
Un error de formato no afirma que el archivo sea malicioso; solo indica que
no cumple este perfil. Los detalles de compatibilidad están en el ADR.

El worker libera recursos, vacía stdout y termina sin handlers `atexit`.
No escribe perfiles de cobertura propios: hacerlo infringiría su límite de
archivos de tamaño cero. La instrumentación de las pruebas de biblioteca y las
pruebas nativas del proceso se mantienen; no aumentar ese límite para obtener
un archivo `.profraw` del hijo.

La cota de memoria corresponde al worker, no a todo el servidor. Dimensionar
`--max-blocking-operations` considerando los procesos simultáneos, los buffers
de descifrado del padre y las conexiones. La limitación global de operaciones
no debe sustituirse por crear un pool de parsers sin esa misma frontera.

El binario nativo y sus bibliotecas no los analiza `cargo deny`. Antes de
cambiar versión o artefacto, revisar la distribución oficial, dependencias y
licencias, actualizar el hash y ejecutar de nuevo pruebas de estructuras
inválidas, presupuesto, integración, evidencia y recuperación. Conservar el
artefacto usado junto con el registro operativo de la entrega.
