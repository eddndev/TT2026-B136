# ADR-0024: Validación acotada de soportes PDF y DOCX

## Estado

Aceptado como decisión de diseño. Los resultados realmente ejecutados y la
integración se registran en [el informe de verificación](../verification-report.md).

## Contexto

Un nombre terminado en `.pdf` o `.docx` no demuestra que el contenido sea un
soporte utilizable. Los parsers trabajan con bytes descifrados y pueden consumir
recursos al recorrer referencias o descomprimir contenido. La preparación de
soportes de [ADR-0023](0023-audited-case-stage-transitions.md) necesita un control
de contenido limitado y reproducible, sin reescribir evidencia histórica.

## Decisión

### Antes del parser

La política `pdf_docx_v1` se aplica al admitir nuevos soportes procesales.
No se aplica retrospectivamente a todos los documentos ni convierte las cargas
generales existentes en cargas con validación de formato.

Se admiten como máximo dos referencias distintas por lote, 16 MiB de contenido
por referencia y 32 MiB en total. El contenedor DVLT1 se limita con su sobrecarga
AES-GCM antes de descifrar. La evidencia JSON se limita a 1 MiB antes de
transferirla y deserializarla desde PostgreSQL; la selección definitiva debe
comprobar nuevamente esos límites en la misma instantánea que sus bytes.

La aplicación comprueba identidad y versión vinculadas al cifrado, longitud y
digest del contenido. Si existe evidencia sellada, valida también la evidencia
capturada. Cada referencia se descifra y autentica una vez; se presta el
contenido al validador sin duplicar el lote completo.

### Un proceso para todo el lote

El adaptador inicia el propio ejecutable con su entrada privada de validación,
antes de configurar registros o servicios. Los bytes pasan por stdin y la
respuesta por stdout; argumentos, errores y registros no contienen texto del
documento. Un protocolo binario acota el número, longitudes y respuesta.

El proceso Linux instala límites del núcleo antes de leer documentos:
5 segundos de CPU, 256 MiB de espacio de direcciones, archivos de salida de
tamaño cero y core dumps deshabilitados. El supervisor impone 10 segundos de
tiempo transcurrido desde el lanzamiento, usa tuberías no bloqueantes y mata
y recoge al hijo al fallar o expirar. No crea hilos de E/S que sigan trabajando
después de devolver el error. La respuesta ocupa como máximo nueve bytes;
una respuesta mayor, incompleta o incompatible se rechaza.

Después de liberar los parsers, la biblioteca y los buffers, el worker escribe
y vacía explícitamente la respuesta. Termina mediante `exit_group` sin ejecutar
handlers `atexit`: estos podrían escribir o consumir recursos después de haber
terminado la validación. Se conservan todos los límites también al instrumentar
pruebas; el proceso hijo no escribe perfiles LLVM. Las pruebas de biblioteca
se instrumentan en su proceso de pruebas y los ensayos nativos siguen ejecutando
el worker real. No se excluye el crate de la medición.

Los límites son de **un proceso y un lote**, no una concesión nueva por cada
archivo. No incluyen descifrado o consultas del proceso padre, ni constituyen
un aislamiento general de red, sistema de archivos o descendientes. El worker
no extrae archivos, resuelve enlaces externos ni inicia procesos adicionales.

### PDF

Se carga qpdf 12.4.1 mediante su API C únicamente dentro del worker, desde una
ruta de configuración del operador. Se exige esa versión exacta, se deshabilita
la recuperación y se rechaza cualquier error o advertencia del parser.
Se rechaza un PDF cifrado incluso si la contraseña de apertura está vacía.

Además de `qpdf_check_pdf`, se comprueban catálogo, árbol de páginas, tipos,
padres, contadores y referencias sin ciclos ni repetición de páginas. La
política limita 4096 páginas, profundidad 128 y tamaño declarado del espacio
de objetos. Los detalles están en
`crates/infrastructure/src/document_formats/pdf/tree.rs`.

No se promete un contador de bytes descomprimidos PDF: su cota general es la
del proceso. La comprobación no renderiza, valida todas las reglas del estándar,
busca malware ni certifica la autenticidad jurídica del acto.

La [API oficial de qpdf](https://qpdf.readthedocs.io/en/stable/library.html)
y la [versión fijada](https://github.com/qpdf/qpdf/releases/tag/v12.4.1)
fundamentan la integración. Su código nativo no forma parte del inventario de
dependencias Cargo: su actualización requiere revisar artefacto, checksum,
licencias y pruebas de admisión.

### DOCX

Se admite un perfil ZIP32 con 1 a 1024 entradas físicas, métodos Stored o
Deflate, sin prefijo, huecos, ZIP64, división en discos, cifrado ni rutas
ambiguas. Se comparan directorio central y cabeceras locales antes de indexar;
se rechazan duplicados físicos y nombres de partes equivalentes. Se comprueban
descriptores, tamaños, CRC32 y consumo exacto del flujo Deflate.

Cada entrada se descomprime una vez. Todos los DOCX del lote comparten límites
de 64 MiB de bytes realmente expandidos y un millón de eventos XML. El XML
admite UTF-8, declaración coherente y profundidad máxima 128; rechaza DTD,
entidades personalizadas, estructura incompleta y espacios de nombres inválidos.

Se exigen tipos de contenido, relaciones internas resolubles y un documento
principal Word con un cuerpo. Se admiten las familias Transitional y Strict
sin mezclarlas. Se rechazan macros, OLE, ActiveX, paquetes embebidos, plantillas
adjuntas y `altChunk`. Las relaciones externas solo admiten hipervínculos
inertes HTTP, HTTPS o mailto; nunca se siguen durante la validación.

No es una validación XSD completa de OPC/OOXML ni comprueba toda referencia de
marcado, apariencia o semántica. Puede rechazar DOCX válidos fuera de este
perfil, por ejemplo XML UTF-16, ZIP64 y otras codificaciones o relaciones.

La descompresión usa `zlib-rs` 0.6.7 mediante `flate2`. Se leyó su archivo
`LICENSE` distribuido por crates.io: permite uso, modificación y redistribución,
exige conservar el aviso en distribuciones de fuentes, no falsear su origen y
marcar fuentes modificadas. Se incorpora `Zlib` a la lista permisiva de
`deny.toml`; no se añade una excepción a avisos de seguridad ni se altera el
código de esa dependencia. Una distribución de fuentes debe conservar su aviso.

## Consecuencias

La admisión es reproducible y conserva los bytes originales, el cifrado y la
evidencia. Una referencia histórica que incumpla el perfil puede seguir
existiendo como documento, pero no se acepta como nuevo soporte procesal.

El formato rechazado, tamaño excesivo y presupuesto agotado tienen errores
distintos; la corrupción almacenada y los fallos de configuración permanecen
opacos para HTTP. Un fallo del proceso nativo no autoriza la mutación.

El despliegue soportado del validador es Linux x86_64 con el artefacto fijado.
La instalación y el arranque deben completarse antes de aceptar tráfico;
se describen en [operación del validador](../document-format-operations.md).
