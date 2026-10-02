# Admision de contenido en nuevas cargas documentales

La admisión de los ocho formatos está implementada en aplicación, HTTP, Qadra
y el adaptador aislado. La aceptación API con restauración y el navegador real
aprobaron; el cierre global y la integración siguen pendientes. La evidencia y
el estado de provisión se distinguen al final de este contrato.

## Entradas y autorizacion

La misma politica obligatoria se aplica a:

- `POST /api/v1/cases/{case_id}/documents`: contenido binario.
- `POST /api/v1/cases/{case_id}/documents/with-metadata`: partes multipart
  `file` y `metadata`, con clasificacion inicial atomica.
- `POST /api/v1/cases/{case_id}/documents/{document_id}/versions?expected_version=N`:
  nueva version binaria, conservando identidad y control de concurrencia.

Se mantienen bearer, `X-Document-Name`, permisos y autorizacion por expediente
existentes. Client continua sin permiso documental. El nombre es una etiqueta
segura: ni su extension, el MIME enviado ni la clasificacion seleccionan o
habilitan un formato. No existe un parametro HTTP para omitir la validacion.

El limite es **16 MiB por archivo inclusive**. La aplicacion autentica y
comprueba acceso antes del parser; valida nombre y tamano antes de admitir bytes.
La admision precede al hash y al cifrado, sin mantener abierta una transaccion
de persistencia. Antes del commit se vuelve a autenticar y se compara el
principal completo, incluidos correo y rol. Un cambio que conserve permisos
invalida la sesion de esta operacion; una perdida de permisos mantiene la
denegacion correspondiente. El store conserva sus controles durables de
membresia, cierre y version esperada dentro del commit auditado.

No se altera ni revalida retroactivamente el contenido historico, las firmas,
los sellos, la descarga ni la evidencia existente. El CLI criptografico queda
separado de esta admision de cargas por expediente. La politica de soportes
procesales `pdf_docx_v1` conserva sus reglas independientes: admitir multimedia
como documento no la convierte en soporte procesal permitido.

## Formatos y contenido permitido

| Familia | Politica de admision |
| --- | --- |
| PDF | Parser nativo y controles estrictos existentes de PDF; no solo la firma inicial. |
| DOCX | Contenedor ZIP, partes y relaciones OOXML validadas con los limites existentes. |
| TXT | UTF-8 estricto y no vacio; BOM inicial opcional. Permite TAB, CR y LF; rechaza NUL y otros caracteres de control. |
| JPEG | Una imagen estatica con segmentos y final EOI completos, sin imagenes concatenadas ni contenido posterior; decodificacion completa. |
| PNG | Una imagen estatica, chunks y CRC validos, orden coherente y final IEND exacto; APNG no admitido y decodificacion completa. |
| MP3 | MPEG audio Layer III, frames completos y envolturas ID3 permitidas; no MP2 ni un ultimo frame truncado. |
| WAV | RIFF/WAVE con PCM de 8, 16, 24 o 32 bits, incluida la variante extensible PCM admitida; longitudes y alineacion de muestras coherentes. No float, RF64 ni RIFX. |
| MP4 | Contenedor regular no fragmentado, video H.264 y/o audio AAC; tablas y referencias internas coherentes, sin DRM, pistas cifradas ni referencias externas. |

El PCM admite `pcm_u8`, `pcm_s16le`, `pcm_s24le` y `pcm_s32le`. Audio:
hasta 8 canales y 192000 Hz. Imagenes y cada frame de video: hasta 16777216
pixeles, con dimensiones positivas. MP4 admite hasta 8 pistas; los otros
formatos multimedia requieren una sola pista. Se rechazan pistas de datos,
subtitulos y caratulas anexas. Las marcas principales MP4 admitidas son
`isom`, `iso2`, `iso3`, `iso4`, `iso5`, `iso6`, `mp41`, `mp42`, `avc1` y `M4V `.
El indice MP4 puede estar antes o despues de los datos multimedia.

La estructura se inspecciona dentro del worker limitado. Para multimedia,
ffprobe comprueba el inventario y FFmpeg decodifica **todas** las pistas
permitidas hasta EOF a salida nula. No se usan recorte temporal, muestreo de
frames, copiado de paquetes ni sustitucion de errores para declarar exito.
Las listas de edicion no reducen el conjunto de muestras validado. El contenido
admitido conserva exactamente sus bytes originales: no se transcodifica ni se
sustituye por la salida del decodificador.

## Presupuesto y aislamiento

| Etapa secuencial | CPU maxima | Espacio de direcciones | Tiempo transcurrido |
| --- | --- | --- | --- |
| Inspeccion estructural y PDF/DOCX/TXT | 5 s | 256 MiB | Hasta 10 s, dentro del vencimiento global. |
| Sonda multimedia | 2 s | 512 MiB | Tiempo restante del vencimiento global. |
| Decodificacion multimedia | 8 s | 512 MiB | Tiempo restante del vencimiento global. |

Las tres etapas comparten **20 segundos de tiempo transcurrido** y sus cuotas
suman 15 segundos de CPU. Las cuotas no se reinician por pista ni se transfieren
entre etapas. Los limites de memoria son espacio virtual por proceso, no una
promesa sobre RSS total del servidor. La inspeccion produce un protocolo de
7 bytes, la sonda hasta 64 KiB y la decodificacion no produce contenido en
stdout. Los diagnosticos nativos se descartan. CORE y FSIZE permanecen en cero.

El supervisor crea un memfd acotado, verifica sus sellos contra escritura,
crecimiento y reduccion, y lo rebobina antes de cada proceso. El protocolo de
entrada es exclusivamente `fd`. La salida nula no crea una version transformada
ni un archivo temporal de texto plano. Las paginas de un memfd pueden llegar a
swap; su uso no garantiza que los bytes nunca alcancen almacenamiento fisico.

El modo privado instala limites antes de leer contenido, cierra descriptores
heredados ajenos y ejecuta el programa nativo mediante `exec`. Cada proceso tiene
un grupo propio. El supervisor observa el lider con `waitid`/`NOWAIT`, termina
el grupo y recolecta al hijo directo ante finalizacion, error, exceso de salida
o vencimiento. No modifica el servidor para adoptar nietos de otros procesos.
Una desconexion HTTP no cancela instantaneamente `spawn_blocking`: la validacion
iniciada conserva su supervisor y termina dentro del presupuesto establecido.

Los limites y protocolos no constituyen un aislamiento completo del sistema
operativo. La admision tampoco es un antivirus, un certificado de autenticidad
juridica ni una verificacion exhaustiva de todo metadato opcional del formato.
Un archivo puede ser valido y aun asi exceder los recursos permitidos; no se
promete una duracion maxima de audio/video que siempre resulte admisible.

## Dependencias y arranque

`scripts/install_media_decoder.py` provisiona FFmpeg **9.0.2**, con fuente y
SHA-256 fijos, componentes minimos y decodificador compilado sin protocolos de
red. La instalacion fue verificada en los tres VPS propios; cada nuevo entorno
necesita su provision compatible antes de arrancar el servicio.
El validador usa rutas administrativas absolutas, por defecto
`/opt/tt-media/bin/ffprobe` y `/opt/tt-media/bin/ffmpeg`, configurables mediante
`TT_FFPROBE_PATH` y `TT_FFMPEG_PATH`. No acepta rutas proporcionadas por HTTP.
La dependencia qpdf mantiene su configuracion existente.

El servidor valida una muestra completa de cada una de las ocho familias antes
de atender peticiones, ademas de comprobar la politica independiente de soportes.
Una dependencia ausente, incompatible o que rechace esas muestras impide arrancar;
no se habilita una admision basada solo en cabeceras. Las muestras ejercitan el
arranque, no equivalen a un corpus exhaustivo de conformidad.

## Errores y conservacion del borrador

El router devuelve JSON con `Cache-Control: no-store`:

```json
{"error":{"code":"document_format_invalid","message":"document format is invalid"}}
```

| HTTP | Codigo | Significado |
| --- | --- | --- |
| 422 | `document_format_unsupported` | El contenido o subtipo no pertenece al catalogo admitido. |
| 422 | `document_format_invalid` | El formato reconocido no supera la validacion. |
| 422 | `document_validation_limit` | La validacion alcanza un limite de recursos y no admite el archivo. |
| 503 | `document_validator_unavailable` | El validador no puede operar; no hay aceptacion permisiva. |

Los mensajes son categorias publicas fijas; no contienen stderr, rutas,
metadatos extraidos ni bytes del documento. Un rechazo no cifra ni persiste una
version nueva. Qadra explica estas categorias y conserva archivo, nombre y
clasificacion, o archivo, nombre y version esperada al agregar contenido.
No publica un resultado optimista ni reenvia automaticamente la escritura.
Corregir o volver a enviar requiere una accion explicita; una respuesta incierta
conserva sus reglas existentes de consulta antes de reenviar.

Los limites HTTP existentes siguen devolviendo 413. El limite del contenido de
la parte multipart usa `document_too_large`; el limite interno de contenido de
aplicacion usa `document_content_too_large`. Esta entrega no homogeneiza los
rechazos previos del extractor HTTP ni cambia su contrato.

## Verificación y cierre pendiente

Pruebas focales y aceptación aprobadas para esta implementación:

| Frontera | Resultado registrado |
| --- | --- |
| Aplicación: tres ingresos, rechazo y reautenticación | 8 aprobadas. |
| Cliente Node: errores tipados, sesión y envío único | 12 aprobadas. |
| Navegador controlado: rechazo y borrador conservado | 8 aprobadas. |
| Adaptador nativo: formatos reales y corrupción comprimida interna | 5 casos distintos aprobados. |
| Worker privado: límites, descriptores y protocolo | 5 aprobadas. |
| Infraestructura: estructuras, inventario y supervisor | 29 casos distintos aprobados. |
| HTTP: categorías y transporte de errores | 1 prueba unitaria y 1 de rutas aprobadas. |
| Instalador: fuente, configuración y provisión | 15 aprobadas en VPS3. |
| Aceptación API completa, respaldo y restauración | `scripts/api-demo.sh` terminó con salida 0; las ocho familias conservaron sus bytes exactos tras restaurar. |
| Navegador con servicios reales | 4/4 en 27.2 s: 1 escenario nuevo de admisión (8.4 s) y 3 regresiones de contenido. |

Las capturas a 1440 y 390 píxeles se inspeccionaron sin desbordamiento horizontal.
Estos recorridos no constituyen una evaluación de usabilidad con personas ni
una nueva medición global de cobertura. Los conteos de casos distintos no
multiplican una prueba por sus repeticiones focales.

La dependencia quedó verificada con el usuario del runner en los tres VPS.
VPS1 usa AlmaLinux y requirió una compilación nativa; VPS2 reutiliza el binario
compatible preparado en VPS3. Clippy aprobó el workspace y todos los targets.
El PDF de 343 páginas compiló y sus nuevas secciones se inspeccionaron
visualmente. CI general e integración de esta entrega permanecen pendientes.
La decisión se documenta en
[ADR 0058](adr/0058-bounded-general-document-admission.md), y la evidencia nueva
se separa del historial en el [informe de verificación](verification-report.md).
