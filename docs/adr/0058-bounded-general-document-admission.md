# 0058. Admision general de documentos con validacion acotada

## Estado

Aceptado e implementado localmente. La verificacion focal esta registrada en
`docs/document-upload-admission-api.md`; API/restauracion y navegador real
aprobaron. El cierre de CI y la integracion permanecen pendientes.

## Contexto

Las cargas por expediente y sus nuevas versiones deben rechazar contenido no
admitido antes de cifrar o persistirlo. Un nombre, una extension, el MIME de una
peticion o una cabecera reconocible no prueban que el archivo completo sea
valido. Los decodificadores nativos tambien consumen CPU, memoria y tiempo bajo
el control de una entrada potencialmente hostil.

La politica de soportes procesales PDF/DOCX conserva su finalidad independiente.
Ampliar el catalogo documental no autoriza usar una imagen, audio o video como
soporte donde esa politica exige PDF/DOCX. Tampoco justifica alterar evidencia
historica ni acoplar la herramienta criptografica de linea de comandos al
catalogo de carga web.

## Decision

Inyectar obligatoriamente `DocumentUploadAdmission` en el servicio documental por
expediente. Aplicar la misma admision a carga binaria, carga clasificada y nueva
version, hasta 16 MiB inclusive. Autenticar y comprobar acceso antes de admitir;
validar nombre y tamano antes del parser. Despues de admitir y preparar el
contenido, volver a autenticar el principal completo antes del commit auditado.
La persistencia conserva sus controles durables de membresia, cierre y version
esperada. El parser no mantiene abierta una transaccion de persistencia.

Admitir PDF, DOCX, TXT, JPEG, PNG, MP3, WAV y MP4 con los subtipos y restricciones
explicitos de `docs/document-upload-admission-api.md`. Reutilizar los parsers
PDF/DOCX existentes. Para multimedia exigir estructura coherente, inventario de
pistas permitido y decodificacion completa de todas las pistas a salida nula;
no basta una sonda, una muestra del contenido ni copiar paquetes comprimidos.
No transcodificar, normalizar ni reemplazar los bytes que se cifran.

Copiar la entrada acotada a un memfd sellado contra escritura, crecimiento y
reduccion. Usar solo ese descriptor rebobinado entre procesos secuenciales.
La inspeccion usa 5 segundos de CPU, 256 MiB de espacio de direcciones y hasta
10 segundos de tiempo transcurrido. La sonda y la decodificacion tienen cuotas
respectivas de 2 y 8 segundos de CPU, y 512 MiB de espacio de direcciones cada
una. Las cuotas suman 15 segundos de CPU; las tres etapas comparten un unico
vencimiento monotono de 20 segundos. Mantener los limites de archivos nuevos y
volcado de memoria en cero y acotar la salida del protocolo.

Ejecutar la sonda y el decodificador mediante `exec` desde un modo privado del
binario, sin crear un nieto no supervisado. Ese modo instala limites y cierra
los descriptores heredados ajenos antes del codigo nativo. El supervisor crea
un grupo propio, observa la salida del lider sin recolectarlo mediante
`waitid` con `NOWAIT`, termina el grupo y recolecta al hijo directo. Esto conserva
la identidad del grupo hasta limpiarlo, incluso cuando el lider termina antes
que un descendiente. No convierte al servidor HTTP en subreaper.

El entorno del proceso nativo se vacia y recibe exclusivamente
`MALLOC_ARENA_MAX=2`. En glibc, las arenas adicionales pueden reservar bloques
virtuales de 64 o 128 MiB; los hilos del scheduler de FFmpeg no desaparecen al
limitar a uno los hilos de codecs y filtros. Una comprobacion MP4 positiva
reprodujo `pthread_create` con EAGAIN y salida 245, y la traza mostro reservas
rechazadas por ENOMEM bajo los 512 MiB existentes. Acotar las arenas elimino
esas reservas rechazadas en la comprobacion trazada, sin ampliar AS, CPU,
tiempo o archivos. No heredar opciones del llamador, incluido GLIBC_TUNABLES,
ni configurar este ajuste solamente en el runner: el exec privado limpia el
entorno. La aceptacion nativa positiva y negativa sigue siendo obligatoria.

Provisionar FFmpeg 9.0.2 mediante `scripts/install_media_decoder.py`, con fuente
y digest fijos, componentes minimos y red deshabilitada en la compilacion. La
invocacion admite exclusivamente el protocolo de entrada `fd`, decodificadores
permitidos y salida nula. Las rutas absolutas de herramientas son configuracion
administrativa; no proceden del archivo ni del usuario HTTP. Antes de servir,
comprobar ocho muestras que ejercitan las ocho familias; un fallo impide el
arranque y no habilita una admision permisiva.

Exponer cuatro categorias estables: formato no admitido, formato invalido,
limite de validacion y validador no disponible. No devolver diagnosticos nativos
ni contenido del archivo. Qadra conserva el archivo y el borrador rechazados;
una nueva escritura exige accion explicita. No revalidar ni transformar
retroactivamente versiones historicas, firmas, sellos o descargas.

## Consecuencias

Los bytes originales permanecen disponibles para hash y cifrado solo despues
de superar toda la politica. La clasificacion inicial y la nueva version
mantienen sus transacciones existentes; un rechazo no crea contenido parcial.
No se agrega una columna MIME ni se modifican los formatos historicos de firma
y evidencia. La politica de soportes de
`docs/adr/0024-isolated-document-format-admission.md` permanece independiente.

Las cotas pueden rechazar archivos validos que requieran mas recursos; ese
resultado no se presenta como corrupcion. No hay una duracion multimedia
universal garantizada dentro de los 16 MiB. Los limites son por proceso y su
suma de CPU es conservadora: una etapa no toma prestada la cuota de otra.

El memfd evita una copia persistente con nombre, pero sus paginas pueden llegar
a swap. Limites de recursos, lista de protocolos y compilacion sin red no son
un aislamiento completo del sistema operativo ni un antivirus. La admision no
certifica autenticidad juridica, ausencia de contenido oculto o conformidad
exhaustiva con todos los estandares de los contenedores.

El trabajo HTTP usa `spawn_blocking`: desconectar al cliente no interrumpe de
inmediato una validacion ya iniciada. El supervisor conserva su responsabilidad
y el vencimiento acotado hasta finalizar; no se promete cancelacion instantanea.
Las pruebas focales no sustituyen la aceptacion con servicios reales, la
restauracion, el navegador real ni los gates completos de integracion.
