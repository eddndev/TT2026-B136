# Instalación recuperable de controladores

## Contrato

`ops/deploy/controller_installation.py` compone el cierre de servicios, las
máscaras, la publicación y la aprobación bajo una sola adquisición de
`deploy.lock`. Se ejecuta desde un [bootstrap privado](deployment-controller-bootstrap.md)
distinto de las fuentes que sustituye. La aplicación privada conserva una
aceptación operativa independiente de las pruebas del instalador.

La intención privada fija por hashes externos el destino, intérprete, launcher,
paquete candidato y fuentes anteriores. También fija las identidades del bloqueo
y directorio de unidades, la caché que debe conservarse, el predecesor de aprobación
y el enlace y metadatos del release. No deriva autorización de archivos encontrados
tras una discrepancia. Las unidades de bases antiguas sin prestart sólo se admiten
cuando esa ausencia está explícitamente incluida en la intención.

Antes de detener servicios valida esos materiales y la forma exacta de las cuatro
unidades. Conserva las referencias de systemd hasta sincronizar su terminación
normal tipada, y después establece máscaras persistentes con recarga y observación
nuevas. Sólo entonces retiene la caché exacta, intercambia fuentes y aprueba la
generación. Conserva las unidades anteriores, sus máscaras y los candidatos nuevos.
No modifica configuración, datos, PKI o release, ni habilita unidades.

## Estados y recuperación

El diario integrado está en
`maintenance/controller-installation/<UUID>/installation.json`. Su operación
conserva además el diario de máscaras, `stop.json`, `closed.json`, `reopen.json`,
originales, candidatos y caché retenida. La publicación y aprobación mantienen
sus recibos en `maintenance/controllers/<UUID>/`, con el mismo identificador.

La llamada inicial retorna `installed_closed`: las fuentes están instaladas y
aprobadas, pero los servicios permanecen cerrados. El recibo contiene
`operation_id`, `intent_sha256`, `state` y `closed_sha256`. El hash cerrado vincula
la parada, aprobación, inventario instalado, launcher y candidatos exactos.

Para reabrir se necesita otro archivo privado y su hash externo. Su formato es
`qadra-controller-reopen`, versión 1, con `operation_id`, `root`, `intent_sha256`
y `closed_sha256`. Se conserva antes de retirar la primera máscara. La reapertura
intercambia exclusivamente los candidatos y máscaras propios; arranca bases, API
y web, y comprueba la disponibilidad del release conservado antes de devolver
`installed`. No interpreta una respuesta perdida como autorización nueva.

Toda reentrada conserva UUID, intención, fuentes, rutas y pins. Reconciliar una
reapertura parcial vuelve a exigir su autoridad explícita; no inicia servicios
por la mera existencia del diario. No hay rollback ciego ni limpieza de objetos
ajenos después de un fallo. Los errores públicos son neutros y se conserva la
evidencia para la misma operación.

## Límites operativos

El presupuesto monotónico total admitido es de 300 a 900 segundos, predeterminado
600. El cierre tiene hasta 240 segundos y las máscaras hasta 30. Cada arranque
necesita disponer de 240 segundos completos; los comandos y sondas consumen el
remanente. Los límites de encabezados HTTP y filesystem se explican en
[disponibilidad](deployment-readiness-deadline.md); no se promete interrupción dura
de esos accesos.

La ventana administrativa externa debe mantener suspendidas las entradas SSH
y los lectores Python anteriores, incluso después de una interrupción. Las
máscaras de systemd y el bloqueo local no impiden esos accesos externos.
Las entradas administrativas versionadas de despliegue y CRL seleccionan el
launcher y una generación aprobada, pero instalar y configurar esos puntos de
entrada sigue siendo una acción operativa explícita.

## Verificación

Los ensayos focales componen los módulos reales y operaciones Linux de archivo,
lock, rename y fsync. Sustituyen únicamente gestor, procesos, listeners y readiness
para inyectar respuestas perdidas y comprobar conservación y reentrada. El ensayo
nativo opt-in usa cuatro servicios inocuos, systemd, procesos, listeners y el
launcher real en una cuenta desechable; sus sondas de datos son sintéticas.
No representa restauración SQL/RDB, aceptación de PostgreSQL/Redis reales ni
instalación del producto. Los resultados ejecutados se registran en
[el informe de verificación](verification-report.md).
