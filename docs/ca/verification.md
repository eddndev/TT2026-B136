# Verificación del portal administrativo de la CA

Las campañas locales del 12 de octubre de 2026 usaron Linux x86_64,
PostgreSQL 18.6, Redis y OpenSSL locales, con dependencias desechables y un solo
coordinador; al terminar se detuvieron únicamente sus servicios propios. El
cierre remoto, la correlación posterior y la apertura pública se distinguen
más adelante. Las campañas locales no acreditan por sí solas el recorrido de
la titular Owner ni operaciones del portal instalado.

| Frontera | Evidencia ejecutada | Límite |
| --- | --- | --- |
| Solicitud pública externa | Trece casos CLI y dieciocho de aplicación; perfiles Partner y firma documental, conservación del perfil predeterminado y rechazo de solicitudes inválidas. | El titular real conserva y usa su propia clave. |
| Identidad y permisos CA | Aplicación, HTTP y PostgreSQL: UUID/correo configurados, MFA, recuperación y sesiones CA separados; rol mínimo sin autoridad sobre el despacho. | La adopción de producción y su acceso real se registran separadamente abajo. |
| Alta inicial privada | Seis casos nativos en `crates/bin/tests/ca_enrollment.rs`: entrega privada, TOTP antes del commit, rechazo de repetición, entrada incompleta, alteración del archivo y cancelación con restauración del eco. | Son pruebas desechables; el alta real posterior usa su propio UUID y entrega privada autorizada. |
| Servicio nativo | `crates/bin/tests/ca_service.rs` ejecuta el binario real, Argon2, TOTP, recuperación, actividad y cierre; verifica auditoría y ausencia de rutas del despacho. | No prueba HTTPS ni configuración del host público. |
| Operaciones nativas | `crates/bin/tests/ca_operation_service.rs`: HTTP real con MFA, OpenSSL, ambos perfiles, respuesta de emisión perdida, inventario, descarga, revocación local e historial. Rechaza actor/rutas inyectados, token de despacho y raíz distinta. | La revocación local no publica CRL; el ensayo conserva su distinción. |
| Publicación recuperable | Cincuenta y cinco casos focales de publicación, guards, backup, renovación, recuperación y fence. El runtime controlado comprueba las guardas reales. | Las operaciones de servicios/publicación del controlador se simulan en esta frontera; no equivale a mantenimiento público realizado desde el portal. |
| Inventario existente | Siete casos Rust y once Python nativos para inspección y operaciones; TSA conservado como consulta, sin ampliar emisión ni revocación. | Dos propósitos de emisión solamente. |
| Interfaz | Sesenta y tres pruebas Node, build y dos recorridos Playwright a 1440 y 390 píxeles, con capturas inspeccionadas: revisión, emisión incierta/reanudación, descarga, revocación, publicación e historial. | Browser usa un contrato HTTP controlado; criptografía y listener se ejercitan separadamente. |
| Restauración SQL | `crates/bin/tests/ca_restore_service.rs`, un caso nativo en 12.37 s: dump, pérdida del esquema CA y restore; conservación exacta de cuenta/auditoría, recuperación ya consumida, nueva MFA y ausencia de acceso al despacho. | Reutiliza roles del mismo clúster desechable; no acredita recuperación entre hosts ni aprovisiona roles automáticamente. |
| Restauración Redis | Ensayo nativo RDB/AOF de `scripts/tests/test_deployment_certificate_login_snapshot.py`, 5.924 s: sesiones y desafíos CA eliminados; cuotas, marcas TOTP y sus vencimientos conservados; el reinicio no revive capacidades. | Usa datos sintéticos en Redis real aislado, sin restaurar producción. |
| Archivo de publicación preparada | `scripts/tests/test_ca_publication_backup_roundtrip.py`: tar real, extracción privada, configuración/KEK CA separada y PKI idénticas, mismo UUID/candidato reanudado sin generar otra CRL ni cambiar serie. | SQL y control de servicios simulados; se complementa con restauración SQL nativa, sin afirmar restauración pública. |
| Paquete y servicios | Doce casos de bundle/workspace y ocho del renderer. El paquete admite binario, web, esquema y fuentes CA; conserva compatibilidad con paquetes anteriores. Las unidades CA no dependen de API/web del despacho. | El renderer no instala servicios ni habilita el DNS. |

La restauración SQL comprueba una sesión cerrada antes del respaldo, no afirma
haber invalidado una sesión activa mediante SQL. La invalidación de sesiones
CA y conservación de cuotas/TOTP pertenecen a las pruebas Redis y al
[procedimiento de restauración](restore.md). El archivo de configuración y la
KEK son parte del respaldo privado; la KEK no se regenera al restaurar.

La comprobación focal de publicación descubrió que la guarda de mutaciones
bloqueaba el arranque de su propia operación. Se separó de las guardas de
arranque manteniendo el bloqueo ante otras mutaciones pendientes. También se
incorporaron al backup de esa publicación sus tres archivos públicos de
recuperación: journal preparado, CRL original y candidata. Las pruebas rechazan
faltantes o enlaces antes de declarar el respaldo completo. El respaldo anterior
sin operación de portal conserva su formato y contenido.

La correlación nativa posterior cerró en 37.955 s el recorrido entre el portal
emisor y sus dos consumidores. Los certificados Partner y de firma documental
emitidos por CSR externo conservaron la misma clave pública y huella en el
vínculo Owner y en la operación documental, respectivamente. Después del uso
válido con RSA/MFA y del documento con TSA y ZIP, una publicación CRL nativa
revocó ambas series y avanzó la confianza de la revisión 1 a la 2. Los nuevos
usos fueron rechazados; los recibos previos, la auditoría y el ZIP exacto
permanecieron consultables. La revisión independiente aceptó esa correspondencia
de certificados, publicación y resultados de los consumidores.

El ensayo reutilizó los servicios y ejecutores existentes: HTTP CA y MFA,
OpenSSL, publicación PostgreSQL, `pg_dump`, captura RDB y archivo privado tar.
Capturó los tres payloads de respaldo y registró sus hashes, sin repetir la restauración
ya documentada. Usó identidades sintéticas y una composición local de la revisión
`8011d339c0de785fd3ea11725d1536ef4ba87e13`; la supervisión de procesos fue un
doble controlado, sin systemd ni proxy público. Por ello acredita la correlación
funcional nativa, pero no califica un artefacto distribuible ni un mantenimiento
del host desplegado.

El recorrido `scripts/demo.sh` también terminó correctamente tras los cambios de
CLI. Clippy focal de `despacho-cli` y sus targets aprobó; se corrigieron dos
warnings de módulos importados por tests. La comprobación de registro de tests
aceptó todas las fuentes, tras explicitar dos inclusiones que antes no reconocía.

El cierre CI detectó un archivo omitido en el inventario obligatorio del paquete
y avisos de Clippy en helpers de pruebas. Se incorporó el renderer CA al
inventario y se corrigieron los usos de referencias/iteradores. Los diez casos
focales de paquete aprobaron; después, `cargo +1.98.1 clippy --workspace
--all-targets -- -D warnings` aprobó en 4 min 42 s. Esta última comprobación
abarca también los targets de infraestructura que el primer focal no incluía;
no sustituye los gates remotos de Rust estable y MSRV.

La calificación posterior del paquete reprodujo una carrera en la observación
del timeout: inmediatamente después de recolectar al líder, Linux todavía
mostraba un descendiente en estado ejecutable. La señal se envía al grupo antes
de recolectar al líder; esperar a éste no sincroniza la terminación de los demás.
La prueba ahora observa durante un máximo de un segundo la ausencia o estado
muerto/zombi del descendiente, y falla si permanece vivo. Conserva el límite de
ejecución y la comprobación inmediata del líder recolectado. Los ocho casos de
`crates/bin/tests/ca_executor.rs` aprobaron en 0.99 s con Rust 1.98.1 tras este
ajuste; el ejecutor de producción no cambió. No se activó el paquete cancelado.

El 13 de octubre, la integración de [PR 105](https://github.com/eddndev/TT2026-B136/pull/105)
en `61de34cce57718d0140b43366fdaf0775c569b4a` cerró Documents, CI y Web en éxito.
La corrección de pruebas y documentación de
[PR 106](https://github.com/eddndev/TT2026-B136/pull/106) se integró en
`95a9aacf975e4a9cde5c4f8b60544fab3baff84f`, con árbol igual al HEAD revisado
`8011d339c0de785fd3ea11725d1536ef4ba87e13`. CI y Web aprobaron para ese main
exacto; la evidencia Documents anterior conserva su revisión propia y no se
presenta como otra ejecución en PR 106.

El candidato `v0.1.8` para ese main aprobó la
[corrida 38354975960](https://github.com/eddndev/TT2026-B136/actions/runs/38354975960).
El paquete y sus 239 archivos se verificaron antes de admitirlos; el binario
principal ocupa 24 081 744 bytes y `qadra-ca`, 4 008 104 bytes. La calificación
arrancó el candidato y el retorno sobre bases aisladas previamente restauradas,
con el mismo esquema y datos históricos; no repitió la restauración completa.

La activación de producción del 13 de octubre conservó las 77 tablas del
despacho, material criptográfico y configuración de correo, con respaldo externo
privado comprobado. Los cuatro servicios quedaron activos y la cadena de
778 732 entradas se verificó válida. Versión y página públicas respondieron
`200`; la identidad anónima recibió `401`. Después de preparar esquema y rol CA
se repitió la comprobación pública del despacho; el acceso Owner por certificado
continuó deshabilitado.

El 14 de octubre de 2026 se completó el enrolamiento real de `eddndev@gmail.com`, UUID
`8e76db9c-7ac4-4047-abd6-46e4b53c6ea4`, con credenciales aleatorias y archivos
privados autorizados por el usuario. La configuración operativa conserva ese
UUID y la release `v0.1.8-95a9aacf975e4a9cde5c4f8b60544fab3baff84f`.
Las dos unidades CA quedaron activas y los cuatro servicios del despacho
continuaron activos. El vhost CA se abrió hacia `127.0.0.1:18088`, conservando
el vhost del despacho. En el navegador, `https://ca.qadra.com.mx` aceptó
contraseña y TOTP y mostró la sesión de `eddndev@gmail.com`.

Esta comprobación acredita instalación y acceso público real. No declara
completados el recorrido de mutaciones del portal instalado, la restauración
integral del host ni la aceptación Owner del [manual operativo](operations.md).
El primer factor Owner por certificado permanece deshabilitado; se conservan
su cuenta, contraseña y MFA existentes.

La aceptación pública de emisión del 14 de octubre encontró un CSR de prueba
con CN, pero sin C/O. La revisión lo admitió y la operación
`be148c5f-7ab2-4c56-80cd-dbb157962926` reservó `1002`; OpenSSL rechazó la ausencia
del país obligatorio. Se comprobaron índice y contador sin cambios y ausencia
de certificado. Reanudar el mismo UUID repitió el rechazo. El journal sigue
pendiente en producción; no se borró, sustituyó ni declaró emisión completada.

La reparación local se verificó con cuatro pruebas Rust de política del sujeto,
60 pruebas Python, incluidas inspección y recuperación con ejecutor nativo,
23 pruebas Node y un recorrido de navegador. La prueba Rust inicial reprodujo
la invocación indebida del script con un CSR que sólo tenía CN. La corrección
compara C/O por tipo y valor ASN.1 contra la raíz y exige CN antes de reservar.
Los casos positivos incluyen otra identidad de CA y atributos repetidos; los
negativos mantienen separados incompatibilidad, prueba inválida y raíz inválida.

La recuperación prueba un rechazo terminal exclusivamente con CSR, clave y
raíz originales, captura completa intacta y hojas ausentes. Conserva el journal
y devuelve el mismo rechazo al repetir; cambios de material, fallos genéricos
y recibos alterados no liberan la guarda. La interfaz presenta rechazo sin
emisión ni descarga y exige una acción explícita para otra solicitud. Estas
comprobaciones locales no son una nueva aceptación de producción: faltan
integración, paquete, despliegue y conciliación del incidente con la revisión
aceptada, antes de continuar las mutaciones del portal instalado.
