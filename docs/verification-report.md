# Informe de verificación local

## Integración de navegación desde actividades y recuperación de audiencias

La cabeza `02f33f5` aprobó CI en 7:42, Web en 13:03 y Documents en 8:45.
Se conservaron 3156 pruebas Rust y dos ignoradas, 396 de navegador controlado y
47 reales, sin perder identidades de la campaña integrada anterior. PR48 se
integró por squash como `5020707ebdb65e85ab913f0252bf319ca2f52931`.
La confirmación natural de main está en curso; estos resultados no sustituyen
las aceptaciones pendientes de informes ni del despliegue.

## Adaptación del despliegue a VPS3: 1 de octubre de 2026

La adaptación conserva el controlador por tags y los límites del despliegue
privado. El paquete se compila en el runner de VPS3 para su ABI de Ubuntu 22.04;
validación y transferencia usan VPS1. qpdf 12.4.1 y FFmpeg/ffprobe 9.0.2 quedan
incluidos en el inventario del paquete y seleccionados mediante rutas explícitas.
Las credenciales y el estado persistente permanecen fuera de cada release.

| Comprobación nueva | Resultado registrado |
| --- | --- |
| Suite del controlador de despliegue, con recuperación de confianza | 30/30 aprobadas en 2.069 s; sustituye el corte anterior de 22 en 2.052 s |
| Regresión focal de configuración del host, tras fijar temporales nginx | 5/5 aprobadas; incluidas en la suite del controlador |
| Recuperación de publicación inicial interrumpida, TDD | RED observado y 7/7 aprobadas en 0.009 s; incluidas en la suite del controlador |
| Recuperación de confianza con PostgreSQL y OpenSSL reales | Cinco comprobaciones aprobadas sobre la migración 0009 exacta |
| Sintaxis de todos los workflows con `actionlint` 1.7.12 | PASS |
| Cuatro unidades systemd de usuario en VPS3 | `systemd-analyze --user verify`: PASS |
| Configuración nginx ejecutada como `qadra` | `nginx -t`: PASS tras corregir rutas temporales |

VPS3 tiene la cuenta `qadra` (UID 1001) y raíz privada `/home/qadra/qadra`.
PostgreSQL y Redis propios están activos en `127.0.0.1:15486` y
`127.0.0.1:16386`. La provisión selecciona Python 3.12.14 y el ejecutable Redis
instalado para las unidades, sin depender del Python ni Redis antiguos del
sistema base. La primera comprobación real de nginx falló por intentar usar
`/var/lib/nginx/fastcgi`, inaccesible para la cuenta sin privilegios. Ahora los
cinco directorios temporales (`client-body`, `proxy`, `fastcgi`, `uwsgi`, `scgi`)
son propios bajo `run/`; la repetición sobre el host aprobó.

API y frontend permanecen inactivos: no se ha activado una release ni creado un
Owner. El enrolamiento espera el correo administrativo elegido por el operador.
Estas pruebas no acreditan todavía el workflow completo por tag, la primera
activación, el recorrido autenticado ni la restauración y recuperación real entre
dos releases. Los 17 ensayos originales del 25 de septiembre se conservan abajo
como evidencia histórica independiente.

La colisión de actualizaciones Rust entre runners que compartían usuario y
`RUSTUP_HOME` se corrigió operacionalmente mediante seis homes separados bajo
`/home/tt-runner/.rustup-runners/`, uno por runner nativo. Se verificaron las seis
unidades activas en los tres VPS, con stable 1.99.0, `rustfmt`, `clippy`,
`llvm-tools-preview` y actualización automática de rustup deshabilitada; el
runner nativo de VPS1 dispone además de Rust 1.88. Los caches y targets Cargo
conservaron sus rutas. Véase [operación de runners](ci-runner-operations.md).
La campaña CI/Web posterior se inició nuevamente a las 04:59:33 UTC; su
resultado completo sigue pendiente. La corrección verificada del host no se
presenta como una campaña aprobada.

La recuperación tras una publicación de confianza confirmada y una escritura
interrumpida del marcador de esquema se verificó por separado. La aceptación
PostgreSQL/OpenSSL aplicó `0009_participant_credential_trust.sql` y comprobó:
ausencia inicial, coincidencia exacta del DER, rechazo de CRL diferente, rechazo
de CA local ausente y conservación de una autoridad y una revisión sin nueva
publicación. El primer intento de preparar el ensayo usó una URI incorrecta y
falló antes de crear el esquema; se corrigieron los campos de conexión del
ensayo y las cinco comprobaciones aprobaron. No fue un fallo del producto.
La recuperación solo admite revisión inicial 1 vigente al tiempo de PostgreSQL,
con CA y CRL locales idénticas a las almacenadas, antes de ejecutar la generación
de material; una diferencia o ausencia de esos archivos exige mantenimiento.
No acredita rotación, restauración general ni recuperación entre dos releases.

El corte documental anterior a esta ampliación produjo un PDF de 345 páginas.
Se inspeccionaron visualmente las páginas físicas 206, 207, 247, 344 y 345 con
resultado aprobado. La nueva redacción de recuperación requiere reconstruir e
inspeccionar el PDF final; no se atribuye esa comprobación a fuentes posteriores.

Una regresión adicional retiró, por separado, siete archivos de firma y TSA
sobre una publicación ya confirmada. Primero fallaron los siete casos; tras
exigir el material completo antes de generar archivos, las ocho pruebas focales
de inicialización aprobaron. La recuperación exige restaurar el material faltante
sin rotar claves ni escribir el marcador. El controlador contiene ahora 31 casos;
el corte completo de 30 y el focal posterior de ocho se informan por separado.

## Despliegue privado por tags: 25 de septiembre de 2026

Se implementaron el workflow de tags exactos, empaquetado por versión/commit,
transferencia SSH con llave de host fijada, servicios privados y recuperación
conservando la base actual. Véanse [operación](deployment.md) y
[ADR 0048](adr/0048-private-versioned-deployment.md).

Verificación focal nueva en Windows con Python 3.14: **17 pruebas aprobadas**
mediante `python -B -m unittest discover -s scripts/tests -p 'test_deploy_*.py'`.
Cubren versión canónica, identidad y checksum, rutas/enlaces de archivo inseguros,
configuración privada, activación, recuperación ante error de salud/respaldo,
rechazo de esquema diferente o versión retrasada y respuestas HTTP locales.
Las transiciones usan dobles de servicios/enlaces; la comprobación HTTP usa un
servidor de prueba y sustituye la base de datos. No se acredita recuperación
real de dos versiones ni un recorrido autenticado del producto.

`actionlint` 1.7.12, descargado con verificación SHA-256, aprobó los tres
workflows afectados; se deshabilitaron sus analizadores externos shellcheck y
pyflakes, no instalados. `bash -n` aprobó ambos guiones nuevos. Los archivos de
lógica nuevos son ASCII y menores de 400 líneas. No se modificó código Rust ni
se actualizaron cifras históricas de cobertura. No se inició otra campaña
Cargo: el runner `vps2-tt2026-b136` ya tenía una ejecución de cobertura activa.

En Ubuntu 24.04 x86_64, mediante el acceso existente, se preparó
`/home/hat/qadra` (0700), con configuración privada 0600, servicios systemd de
usuario y lingering habilitado. `systemd-analyze --user verify` y `nginx -t`
aprobaron. PostgreSQL 16 y Redis propios quedaron activos y habilitados, con
listeners exclusivamente `127.0.0.1:15486` y `127.0.0.1:16386`. Los servicios API
y web están preparados pero inactivos hasta recibir una versión verificada.
La consulta `SELECT 1` con el rol runtime y `PING` autenticado de Redis aprobaron.
Se confirmó que la llave Ed25519 proporcionada coincide con la configurada y
ya autorizada. Los dos secretos cifrados y cuatro variables de Actions quedaron
configurados; se verificaron sus nombres por API, sin publicar valores privados.

No se publicó un tag ni se ejecutó el workflow completo de despliegue. Quedan
por acreditar la compilación del paquete en Actions, su primera activación,
el recorrido autenticado y la recuperación entre dos versiones reales. El
workflow exige CI y Web exitosos para el tag; cualquier fallo bloquea el cambio.
La actualización y comprobación académica se registran por separado en
[el informe documental](academic-report-verification.md).

## Reprogramación próxima de audiencias: planes compatibles con el inventario

La aceptación HTTP con servicios reales detectó un fallo al reabrir `serve`
después de una parada SIGTERM con salida cero. El inventario persistido rechazó
una alerta de audiencia con el motivo `notification kind does not apply to
hearing`. El fallo ocurrió antes de generar informes. El planificador creaba
un episodio `DueChangedSoon` al cambiar una fecha próxima sin distinguir
entre audiencia y plazo. Ese plan podía guardarse, aunque la activación y el
inventario de arranque exigen que una audiencia sólo tenga `Upcoming`, como
establece [el contrato de alertas](alerts-api.md).

La reproducción focal del planificador obtuvo **dos fallos y un control positivo
aprobado**. Una segunda reproducción con PostgreSQL aislado obtuvo **dos fallos
y un control positivo aprobado en 14.55 s**: los casos de audiencia fallaron al
activar o reabrir, con el mismo error observado por HTTP. El control de plazo
conservó su aviso de cambio de vencimiento. Las pruebas de reapertura comparan
el estado persistido antes y después de abrir el adaptador; no emplean reparación
de filas para superar el inventario.

La corrección limita la creación del episodio de cambio a sujetos de plazo.
La reprogramación de una audiencia conserva la reconciliación de anticipaciones
con la nueva fecha y su origen. No cambian preferencias, tiempos, validadores,
contrato HTTP ni datos existentes. Las guardas SQL vigentes comprueban tamaño y
digest del contenido, relaciones e inmutabilidad; no comprueban la combinación
semántica audiencia/tipo de aviso dentro de ese contenido. La corrección evita
producirla, sin omitir la validación estricta al activar o iniciar el servidor.

Después de la corrección aprobaron los **tres casos unitarios y los tres casos
PostgreSQL**, estos últimos en **14.96 s**: seis casos distintos en total.
Incluyen planes pendientes y avisos activados de audiencia, reapertura sin
reescritura de filas y conservación de `DueChangedSoon` para plazos. Son los
mismos casos de la reproducción anterior, no seis pruebas adicionales a ella.
Sigue pendiente repetir el reinicio del servidor compuesto y la aceptación HTTP
que detectó el problema. El resultado focal no modifica los totales históricos
de pruebas o cobertura ni acredita ese cierre integrado.

La prevención no sanea registros incompatibles ya persistidos. Un inventario
con ellos debe seguir rechazándose; no se borran planes, se reinterpretan como
anticipaciones ni se relaja el arranque. Cualquier recuperación de datos no
desechables requiere un procedimiento explícito y verificable por separado.

## 2026-10-02: ubicación del recorrido de clasificación documental

La ejecución de cierre del navegador agotó los 60 segundos totales del escenario
de clasificación en VPS2, durante la recarga final del historial. La revisión 5
ya era visible; el único rechazo HTTP registrado fue el conflicto 409 esperado.
Los resultados anteriores del mismo escenario fueron 47.478–50.075 segundos en
las entregas funcionales recientes. No se atribuye la variación a una causa de
CPU concreta sin una medición controlada.

Se asignó únicamente este escenario a la tercera partición, en VPS3. El recorrido,
sus aserciones, sus tiempos y sus datos permanecen iguales; no requiere otra
familia de preparación. La nueva prueba de asignación falló primero y después
aprobó junto con las demás comprobaciones del plan: **11/11**, incluida la unión
exacta de los archivos descubiertos sin duplicados. Esto acredita la asignación;
la estabilidad y el tiempo del recorrido concurrente quedan pendientes del CI.
Rust y documentación habían aprobado en la cabeza anterior; la campaña de
navegador cancelada no acredita la regresión completa.

La actualización académica posterior de estos resultados y la comprobación del
PDF se documentan en [la revisión del reporte](academic-report-verification.md).
Esa revisión documental no constituye una nueva ejecución de la suite Rust.

## Informes propios y arranque: comprobación del 2 de octubre de 2026

La entrega de [informes](case-reports-api.md) conserva una captura autorizada y
cifrada, publica juntos PDF y CSV, y mantiene avisos propios durables. Estas
comprobaciones son locales; no acreditan todavía una cabeza global integrada
ni una activación de VPS3. Los grupos focales son distintos y no se suman como
si fueran una ejecución completa de CI.

La primera campaña global detectó un fallo de preparación en el verificador SQL
del calendario: todavía extraía las migraciones de `postgres.rs`, aunque las
constantes habían pasado a `postgres/migrations.rs`. CI y Web se cancelaron
automáticamente antes de completar la regresión. El helper ahora resuelve las
rutas respecto al módulo que las declara y rechaza un inventario vacío o ajeno
a `migrations/`. La comprobación SQL corregida aprobó sus 21 casos en 5.369 s
con PostgreSQL desechable; el cierre global requiere la nueva cabeza publicada.

La segunda campaña se canceló al rechazar la política de dependencias
`rustybuzz` 0.20.1 y `ttf-parser` 0.25.1, sin mantenimiento según
[RUSTSEC-2026-0206](https://rustsec.org/advisories/RUSTSEC-2026-0206.html) y
[RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html).
Se sustituyeron por HarfRust 0.13.3 y Skrifa 0.46.2, con un único parser
`read-fonts` 0.43.3; no se añadieron excepciones a la política. Los vectores
fijos capturados del motor anterior conservan métricas, glifos, clústeres,
avances, posiciones y líneas de ambas fuentes Noto originales.

| Comprobación posterior del motor tipográfico | Resultado nuevo |
| --- | --- |
| Captura de referencia sobre el motor anterior | 1/1; 0.32 s, generador temporal retirado |
| Equivalencia completa y reutilización del plan | 2/2; 0.38 s |
| Renderizado, paginación, límites y texto original | 15/15; 1.96 s |
| Política de avisos, licencias, fuentes y restricciones | `cargo deny check`: PASS |
| Clippy workspace y todos los targets | PASS; 2 min 17 s |
| PDF representativo mediante el proceso aislado | 1/1; 0.566156 s de renderizado, compilación separada de 59.44 s |

La última comprobación reutilizó la captura exacta de 60 expedientes, 179
asignaciones y 120 filas de carga. Conservó los **1654401 bytes**, las 25 páginas,
el texto extraído, todas las identidades y las continuaciones; su SHA-256 sigue
siendo `32659e0008fefdf7565ba66d60708cc3e117871e11d4affb23bfb26c808a4674`.
Los 0.566156 s nuevos y 1.604057 s anteriores son observaciones individuales
sobre esa captura, con los mismos límites del worker. No se repitió la captura
máxima ni se extrapola esta diferencia al tiempo total de CI. La inspección
visual previa corresponde a esos mismos bytes. La aceptación API y de navegador
registrada abajo precede a esta sustitución; los gates de la nueva cabeza
publicada deben confirmar la regresión global.

| Comprobación de esta entrega | Resultado observado |
| --- | --- |
| Aplicación: contratos, autorización, trabajos y avisos | 26 casos aprobados |
| Protección cifrada, renderizado inicial y persistencia | 12, 12 y 24 casos, respectivamente |
| Esquema de informes | 4 casos aprobados |
| Paginación y renderizado final | 15/15 |
| Selector de litigantes: aplicación, PostgreSQL y HTTP de informes | 6/6, 4/4 y 14/14 |
| Cliente Node y navegador controlado | 19 y 13 casos distintos aprobados |
| Composición y supervisión del servidor | 43/43 |
| Validación PostgreSQL compartida durante construcción | RED: 5 aprobados y 3 fallidos; GREEN: 8/8 en 15.00 s |
| Capacidad de arranque sellada y sin escape | 2/2 ejemplos de rechazo de compilación |
| Reutilización de plan tipográfico con glifos y posiciones exactos | 2/2 |
| Clippy workspace, todos los targets, con `-D warnings` | aprobado; 19.91 s |
| Navegador contra servicios reales, un worker | 2/2; 23.4 s de Playwright y 207.863 s de comando |
| API completa con restauración y sesiones nuevas | salida 0 en 326.299 s; 3136 fuentes sin cambios durante la ejecución |
| Demostración CLI criptográfica y rechazos esperados | `scripts/demo.sh` completo, salida 0 |

El recorrido real de Owner y Litigator conserva bytes, hashes, identidad de la
captura y avisos al iniciar una sesión nueva. Cuatro capturas de 1440 y 390
píxeles son legibles, sin superposiciones ni desbordamiento horizontal. Esta
inspección no sustituye una evaluación de usabilidad con personas.

La aceptación HTTP verificó tres trabajos por cola, replay exacto, publicación
pareada, avisos leídos y no leídos, denegación a otros roles y a otro Owner, y
revocación del informe completo al perder acceso a uno de sus expedientes.
Tras `pg_dump`/`pg_restore` y MFA nuevo, PDF/CSV, identidades y avisos conservaron
sus valores exactos. El proceso recorrió los estados observables sin exigir
que cada consulta alcanzara a ver todas las transiciones intermedias.

Los reinicios con inventario poblado después de SIGTERM y SIGINT estuvieron
listos en **5 s cada uno**, conservando historial y evidencia. Antes de compartir
la validación durante construcción, el primero tardó 59 s y el segundo no
estuvo listo dentro del límite existente de 60 s. Ese límite no se amplió.
[ADR 0062](adr/0062-scoped-postgres-startup-validation.md) conserva la validación
completa para aperturas independientes y reconexiones. Los ensayos previos de
arranque que terminaron antes de informes no se cuentan como su aceptación.

La captura máxima de 1000 expedientes, 10000 asignaciones y 1000 filas de carga
produjo CSV de 1523555 bytes en 0.382 s. Su PDF devolvió `CapacityExceeded`
en 15.197 s: no se declara soportado ese máximo combinado para cualquier texto
ni se atribuye el rechazo a un recurso concreto. Una captura representativa de
60 expedientes produjo 25 páginas y 1654401 bytes. Reutilizar el plan tipográfico
redujo su tiempo de 2.7404 a 1.604057 s, con bytes idénticos y SHA-256
`32659e0008fefdf7565ba66d60708cc3e117871e11d4affb23bfb26c808a4674`.
Las páginas físicas 1, 2, 3, 13 y 25 se inspeccionaron sin recortes ni
solapamientos. Las mediciones usaron el proceso aislado en perfil de desarrollo;
no son una promesa de latencia para el despliegue.

Los filtros representan creación y estado actual. Correo, estimación de entrega,
otros tipos de reporte y evaluación de rendimiento jurídico de CU-16/RF-19
permanecen pendientes; avisos internos y carga no sustituyen esos requisitos.

## Recursos relacionados desde actividades: comprobación del 27 de septiembre de 2026

La consulta inversa autoriza una audiencia o plazo concreto, lee las cabezas
actuales de sus asociaciones y conserva sus capturas históricas. La interfaz
abre el recurso y vínculo exactos y regresa a la actividad original y a la
bandeja filtrada sin marcar alertas como leídas. Su contrato está en
[activity-resource-links-api.md](activity-resource-links-api.md) y la decisión
en [ADR 0059](adr/0059-activity-resource-navigation.md).

| Comprobación nueva | Resultado ejecutado |
| --- | --- |
| Aplicación: ámbito, contrato de página y reautenticación | 5/5 |
| PostgreSQL desechable: lectura, acceso, auditoría y concurrencia | 6 casos distintos aprobados |
| Regresiones PostgreSQL de asociaciones existentes | 12/12 |
| HTTP inverso: rutas, permisos, filtros y contrato | 6/6; diez regresiones previas aprobadas |
| Cliente Node | 8/8 |
| Navegador controlado, un worker | 9 casos distintos aprobados |
| Planificador de fixtures reales | 9/9, incluyendo el archivo nuevo en su familia existente |

Las pruebas fallaron primero ante la ausencia del nuevo comportamiento. El
caso PostgreSQL de revocación necesitó incrementar revisión y generación de
cuenta en su fixture para respetar el guard existente; después aprobó aislado.
El navegador de plazo necesitó representar una revisión histórica con estado
operativo no comprobado; se corrigió el fixture sin debilitar el contrato.
Las repeticiones focales no se cuentan como pruebas adicionales. No se ejecutó
una suite completa local. El recorrido con servicios reales aprobó 1/1 en
11.6 s de escenario y 15.5 s del ejecutor. Se inspeccionaron las capturas a 1440
y 390 píxeles: el panel mantiene el sistema Qadra y no desborda horizontalmente.
El flujo HTTP integrado y su respaldo/restauración aprobaron con salida cero,
incluidas páginas inversas, filtros, capturas y rechazo tras revocar pertenencia.
Las ocho familias documentales también conservaron sus bytes tras restaurar.
Después de corregir sólo tildes visibles, los dos recorridos de navegación
aprobaron en 8.5 s y las 17 regresiones de alertas, plazos y acceso a asociaciones
en 27.7 s, siempre con un worker. Clippy del workspace y todos sus targets aprobó con advertencias denegadas
(1 min 51 s); el registro estático comprobó 216 ejecutables de integración sin
fuentes duplicadas. El PDF final de 344 páginas compiló y se inspeccionaron las
páginas físicas 205, 206, 246 y 247. No se observaron nuevos recortes ni
referencias sin resolver; persisten las sustituciones históricas de versalitas
y el desborde conocido de 0.11754 pt. El cierre global sigue pendiente.

## Admisión documental general: aceptación local del 27 de septiembre de 2026

La nueva admisión valida PDF, DOCX, TXT, JPEG, PNG, MP3, WAV y MP4 en cargas
iniciales y nuevas versiones de hasta 16 MiB. La política inspecciona el contenido
antes de cifrarlo, exige decodificación multimedia completa en procesos acotados
y reautentica al principal antes del commit auditado. Conserva los bytes
originales, la clasificación atómica, los permisos y la política independiente
de soportes procesales PDF/DOCX. Su contrato está en
[document-upload-admission-api.md](document-upload-admission-api.md) y su diseño
en [ADR 0058](adr/0058-bounded-general-document-admission.md).

| Comprobación nueva | Resultado registrado |
| --- | --- |
| Aplicación: tres ingresos, rechazo y reautenticación | 8 aprobadas |
| Infraestructura: estructuras, inventario y supervisor | 29 casos distintos aprobados |
| HTTP: categorías y transporte de errores | 1 prueba unitaria y 1 de rutas aprobadas |
| Adaptador nativo con formatos reales y corrupción comprimida interna | 5 casos distintos aprobados |
| Worker privado: límites, descriptores y protocolo | 5 aprobadas |
| Cliente Node: errores tipados, sesión y envío único | 12 aprobadas |
| Navegador con HTTP controlado: rechazo y borrador conservado | 8 aprobadas |
| Instalador: fuente, configuración y provisión | 15 aprobadas en VPS3 |
| Aceptación API completa, respaldo y restauración | `scripts/api-demo.sh`: PASS completo, salida 0 |
| Navegador con servicios reales | 4/4 en 27.2 s; escenario nuevo de admisión en 8.4 s |

La aceptación API restauró las ocho familias y comprobó sus bytes originales
exactos. El grupo de navegador real contiene un escenario nuevo de admisión y
tres regresiones del flujo de contenido existente; no son cuatro casos nuevos
de formatos. Se inspeccionaron las capturas a 1440 y 390 píxeles sin
desbordamiento horizontal. Estas comprobaciones no sustituyen una evaluación
de usabilidad con participantes reales.

Los conteos de infraestructura y del adaptador nativo corresponden a casos
distintos aprobados; las repeticiones focales no se suman como pruebas nuevas.
Las pruebas nativas incluyen daño interno del contenido comprimido y la
supervisión separa rechazo de formato, exceso de recursos e indisponibilidad.
No se declara una regresión global ni se actualizan porcentajes históricos de
cobertura con estos resultados.

La dependencia quedó verificada con el usuario del runner en los tres VPS.
VPS1, con AlmaLinux, requirió una compilación nativa porque el binario de Ubuntu
22 exige GLIBC 2.35; VPS2 reutiliza el binario compatible preparado en VPS3.
Cada instalación comprobó su manifiesto, configuración y capacidades sin
escribir ni recompilar durante la verificación. Clippy del workspace y todos
los targets aprobó con advertencias denegadas. El PDF de 343 páginas compiló; se inspeccionaron las páginas físicas 171, 172,
223, 224 y 225 sin desbordamiento visible ni referencias sin resolver en las
secciones nuevas. CI general e integración permanecen pendientes.
Esta sección registra únicamente la admisión documental; la evidencia de la
creación contextual de plazos se conserva separada a continuación.

## Creación contextual de plazos: aceptación local del 27 de septiembre de 2026

La entrega añade preparación y confirmación conjuntas de un plazo y su vínculo
con un recurso. Reutiliza perfil, fuente temporal, calendario y políticas
explícitas; el acto seleccionado aporta contexto histórico y no infiere una
regla jurídica. El contrato está en [resource-deadlines-api.md](resource-deadlines-api.md)
y la transacción y el marcador de origen en
[ADR 0057](adr/0057-atomic-resource-deadline-creation.md).

| Comprobación nueva | Resultado local |
| --- | --- |
| Aplicación: preparación, recibos y permisos | 5/5 |
| PostgreSQL desechable: atomicidad, concurrencia, replay y revalidación | 9 casos distintos aprobados |
| HTTP: comando conjunto, padres, autenticación y límites | 3/3 |
| Cliente, comparación temporal y conciliación en Node | 11/11 |
| Navegador con HTTP controlado, un worker | 11/11, 28.8 s |
| Navegador con servicios reales, un worker | 1/1, 13.6 s de escenario y 17.8 s del ejecutor |
| Aceptación HTTP integrada y respaldo/restauración | PASS completo, salida 0 |
| Clippy, workspace y todos los targets, con advertencias denegadas | PASS |
| LuaLaTeX y revisión visual de las secciones modificadas | PASS, PDF de 341 páginas |

El grupo PostgreSQL aprobó ocho casos en la primera ejecución y el noveno en
una repetición focal tras corregir su fixture de roles. Son nueve pruebas
distintas; no se presenta esa repetición como otra campaña completa. Las
comprobaciones incluyen fallo de auditoría dentro de la transacción, ausencia
de escrituras parciales, dos confirmaciones concurrentes, rechazo de una
operación ordinaria previa y nueva comprobación de pertenencia y fuente vigente.
La repetición exacta conserva los recibos originales incluso tras cerrar el
expediente; una nueva creación permanece impedida por el cierre.

La aceptación HTTP comprobó confirmación conjunta, repetición exacta, agenda y
rechazo de una preparación obsoleta. Después de restaurar PostgreSQL y renovar
la sesión, comparó las capturas del plazo y la asociación, sus recibos y su
proyección en agenda. La demostración completa terminó con salida cero. El
rechazo previo por conflicto no sustituye la prueba de atomicidad con fallo
inyectado dentro de la transacción.

Las regresiones se observaron fallar antes de corregirlas. La preparación real
reveló que la administración del recurso exponía el instante como RFC 3339 y la
del plazo como segundos y nanosegundos: el cliente ahora compara el mismo
instante sin perder nanosegundos ni normalizar fechas civiles inválidas. La
conciliación también aceptaba dos recibos ordinarios coincidentes sin acreditar
su origen conjunto. Ahora conserva ese par como no confirmado y exige una
acción explícita con el mismo comando y digest; el servidor comprueba el
marcador auditado antes de devolver éxito. Otro fallo reproducido impedía esa
comprobación histórica tras cerrar el expediente: se permite confirmar el
origen del par existente, pero se mantiene bloqueado reintentar una creación
cuando ambos registros están ausentes y el expediente está cerrado.

El recorrido real creó el plazo desde un acto histórico, confirmó ambos recibos,
repitió la operación y abrió la revisión exacta desde la agenda. Se inspeccionaron
las capturas de escritorio y móvil, sin desbordamiento horizontal. Las suites
se ejecutaron secuencialmente con un compilador y un worker. Estos resultados
son focales y de aceptación; no constituyen una nueva regresión global ni una
medición de usabilidad con personas. El PDF final tiene 341 páginas; se revisaron
visualmente las páginas físicas 203, 205, 206, 243, 244, 245, 335 y 337, sin
desbordamientos ni referencias sin resolver en esas secciones. Persisten avisos
de sustitución de versalitas de la fuente, sin impedir la compilación. El cierre
global y la integración de esta entrega permanecen pendientes. No se consideran completados
el corpus jurídico, la activación automática ni las audiencias propias de recursos.

### Cierre global de la creación contextual

La cabeza `a9a3a331b403547d79494dc3d5b7edb2d7da0db4` aprobó CI en 6 min
48 s, Web en 11 min 39 s y Documents en 7 min 50 s. Se conservaron todas las
identidades de pruebas previas: 3084 Rust, dos ignoradas, 378 de navegador
controlado y 45 con servicios reales. Los gates de cobertura conservaron
97/95/93 % para domain/application/infrastructure. PR46 se integró por squash
como `23970cc516854688da587e878d4304fa52068ecf`; la ejecución natural de main
aprobó también: CI en 6 min 37 s, Web en 13 min 19 s y Documents en 7 min
39 s, conservando el mismo inventario y todos los gates. Esa confirmación
natural es independiente de la campaña de la PR.

La primera campaña se canceló al fallar un helper de navegador: confundía una
lista de hechos históricos con un estado compartido ya inicializado y omitía
preparar la página de login. El fallo se reprodujo localmente; se separaron los
parámetros y aprobaron 15 casos afectados en 34.6 s sin alterar los tiempos,
assertions ni comportamiento del producto. Esa campaña cancelada no acredita
el inventario completo; las cifras anteriores corresponden a la cabeza verde.

## Tablero operativo: verificacion focal del 27 de septiembre de 2026

Esta entrega anade `GET /api/v1/dashboard` y los indicadores de Inicio para
Owner y Litigante. La consulta usa una sola transaccion auditada y verifica
la cuenta, las pertenencias, las cabezas actuales y los limites completos del
agregado. Paralegal y Cliente no reciben estos indicadores. No se consideran
cerrados los pendientes de firma personal, calificacion juridica de plazos,
informes exportables ni otros modulos por incorporar este tablero.

| Comprobacion nueva | Resultado local |
| --- | --- |
| Aplicacion, autorizacion y reautenticacion | 5/5 |
| PostgreSQL desechable: scope, contratos, plazos, limites y revocacion concurrente | 8/8, 25.45 s |
| HTTP: autenticacion, filtros estrictos y contrato | 3/3 |
| Cliente y valores del tablero en Node | 7/7 |
| Navegador con HTTP controlado, un worker | 8/8, 14.5 s |
| Navegador con servidor real y cuentas independientes | 1/1, 6.5 s de escenario |
| Planificador de particiones y autenticacion de fixtures | 11/11 |
| Cancelacion del conjunto CI, Web y Documents | 6/6 |
| Aceptacion API integrada y respaldo/restauracion | PASS completo, salida 0 |
| LuaLaTeX y revision visual de las secciones nuevas | PASS, PDF de 340 paginas |

Las pruebas focales se escribieron antes de la implementacion. Se observaron
fallos por ausencia del modulo, adaptador y panel; se corrigio tambien el
rechazo HTTP de consultas desconocidas para conservar el contrato de 400.
La regresion de cancelacion fallo primero cuando Documents quedaba fuera del
conjunto. Las suites locales se ejecutaron secuencialmente con un compilador
y un worker, sin repetir la regresion Rust completa.

El recorrido real inicia Owner y Litigante, comprueba el despacho completo
frente a un expediente asignado con su contrato, retira la pertenencia y exige
cero indicadores y ninguna carga al actualizar. Para esta comprobacion local
se prepararon exclusivamente sus fixtures; la particion remota conserva el
resto de familias. Se inspeccionaron capturas de 1440 y 390 px, tanto con HTTP
controlado como con servidor real. No hubo desbordamiento horizontal. El PDF se compilo con `make -B -C latex`;
se renderizaron e inspeccionaron las paginas fisicas 205, 244 y 336
(implementacion, pruebas y trazabilidad), con referencias resueltas, texto
legible y transiciones sin recortes. No se afirma una nueva revision visual
integral de las 340 paginas.

El indicador documental significa contrato de la version actual pendiente de
sello interno. La urgencia visual usa vencimiento y proximidad de 48 horas;
no atribuye una calificacion juridica fatal. Las ventanas de siete dias
incluyen la de 48 horas; los plazos retirados o atendidos no cuentan como
trabajo pendiente, y los que requieren revision no reutilizan una fecha
historica como vencimiento operativo.

La regresion integrada detecto una rafaga de navegacion que agotaba los dos
trabajadores y devolvia `503 server_busy` a una peticion ya admitida. La admision
ahora conserva como maximo ocho peticiones y espera asincronamente un trabajador,
sin aumentar los dos trabajadores ni agregar reintentos. La regresion fallo
primero en cuatro comprobaciones y despues aprobo 7/7: dos operaciones activas,
seis en espera, novena rechazada, cancelacion antes/despues de iniciar y liberacion
tras errores. Los permisos de contenido verificado conservan su limite separado. El recorrido
real que abria una segunda sesion y navegaba entre expedientes aprobo en
24.1 s con las mismas assertions y limite total.

La aceptacion HTTP integrada con respaldo/restauracion aprobo: contadores y
carga exactos tras restaurar PostgreSQL, lectura UTC nueva, MFA nuevo, ambito
vacio del litigante revocado y denegacion de Paralegal/Cliente. Tambien
aprobaron los recorridos existentes de la misma demostracion y la igualdad
de evidencia ZIP. La campana completa de CI/Web/Documents queda pendiente. Los
resultados historicos inferiores describen otras revisiones y no sustituyen
la comprobacion de esta entrega. Documents se traslada a los servidores
propios, con herramientas provisionadas y el mismo control de fuentes y PDF;
no se requieren minutos adicionales de maquinas alojadas por GitHub.

## Cierre de rendimiento con PostgreSQL nativo: 27 de septiembre de 2026

La cabeza `793c8de` aprobo [CI 36300194908](https://github.com/eddndev/TT2026-B136/actions/runs/36300194908)
y [Web 36300194936](https://github.com/eddndev/TT2026-B136/actions/runs/36300194936)
con todos sus checks y gates. Los JUnit conservan exactamente las mismas
identidades del checkpoint completo anterior: **3049 Rust**, **359 simuladas**
y **43 reales**, sin duplicados ni fallos. Las dos pruebas Rust ignoradas
siguen declaradas aparte. Cobertura: domain **5512/5629, 97%**, application
**17215/17963, 95%**, infrastructure **30656/32701, 93%**; todos los umbrales de 90%
aprobaron. El chequeo SQL del calendario tambien aprobo antes de Nextest.

| Medicion | Resultado |
| --- | --- |
| CI completo, creacion a ultimo check | **6m23s** |
| Web completo, creacion a ultimo check | **10m26s** |
| Compilacion Rust instrumentada caliente | 0.23s |
| Ejecucion de las 3049 pruebas Rust | 328.404s |
| Generacion del reporte de cobertura | aproximadamente 6.4s |
| Gate agregado de cobertura | 9s |
| Jobs reales completos, particiones 1/2/3 | 10m12s / 10m17s / 6m50s |
| Compilacion real caliente, particiones 1/2/3 | 2.09s / 2.10s / 0.14s |
| Ejecucion navegador real, particiones 1/2/3 | 315.659s / 326.224s / 193.942s |
| Ejecucion navegador simulado, particiones 1/2 | 413.140s / 452.772s |

Frente al checkpoint completo anterior de **13m12s / 11m15s**, CI bajo a
**6m23s** y Web a **10m26s**. El objetivo aproximado de diez minutos queda
reproducido en esta campana. La preparacion de fixtures sigue separada de
la ejecucion del navegador: el comando real de la particion 2 tomo 585s,
con 326.224s de navegador y 2.10s de compilacion; el resto incluye servicios,
criptografia y fixtures. Los logs conservan los tiempos por familia. La
provision inicial de cache fria de VPS3 (1m54s) fue un coste anterior y no
forma parte de esta medicion caliente.

Durante ventanas activas, VPS3 promedio **6.60 CPU** y alcanzo **27.87 GiB**,
con 14 eventos MemoryHigh y sin OOM ni swap. Su PostgreSQL nativo mantuvo el
limite de 6 GiB y alcanzo **4.04 GiB**, sin eventos de memoria en esa unidad;
se observaron hasta 55 conexiones. VPS2 promedio **3.08 CPU**, pico de 5.01 GiB,
sin OOM ni throttling de cuota. VPS1 promedio 1.82 CPU, pico de 4.47 GiB,
1217 eventos MemoryHigh, 113 eventos max y 18.83s de throttling, sin OOM;
su swap pico fue de 0.36 GiB. No se aumentaron recursos para esta campana.

No se requieren nuevos ajustes para perseguir segundos adicionales antes
de integrar. La confirmacion de estabilidad corresponde a la ejecucion
natural de main posterior a la integracion; esta campana no acredita esa
ejecucion posterior.

## PostgreSQL para el chequeo SQL previo: 27 de septiembre de 2026

La cabeza `fe8053d` detuvo [CI 36299431042](https://github.com/eddndev/TT2026-B136/actions/runs/36299431042)
antes de Nextest: el chequeo SQL independiente del calendario requeria
`DOCUMENT_TEST_DATABASE_URL`, pero el servicio nativo se iniciaba solamente
al ejecutar la cobertura. Las tres clases fallaron en preparacion; no se
ejecutaron sus pruebas ni la suite Rust. La cancelacion automatica detuvo CI
y Web; el ultimo job termino 18 segundos despues del error. No hay artefactos
de resultados ni una medicion valida de mejora para esta campana.

El paso previo ahora invoca el mismo supervisor para ejecutar el chequeo SQL
con su propio cluster desechable. La cobertura sigue obteniendo otro cluster
nuevo; el modo no dedicado conserva su servicio anterior. El comando corregido
aprobo **21/21 pruebas SQL en 3.477s** en VPS3 con el usuario del runner,
PostgreSQL16, SCRAM y limite6GiB. El supervisor termino correctamente y elimino
su servicio y cluster. Formato YAML, ASCII, limite de lineas y diff aprobaron.
No se modificaron assertions, limites de tiempo, producto ni recursos.
La nueva campana completa sigue pendiente.

## PostgreSQL nativo para CI: 27 de septiembre de 2026

Una comparacion focal secuencial uso el mismo ejecutable instrumentado, una
base nueva por variante y las mismas opciones desechables de durabilidad.
La prueba de alteraciones del catalogo aprobo **1/1** en cada variante:

| Servicio PostgreSQL 16 | Prueba | Llamadas SQL | Ejecucion SQL | Trabajo JIT |
| --- | --- | --- | --- | --- |
| Contenedor rootless Alpine | 40.747s | 53718 | 10.010s | 0 |
| Paquete nativo Ubuntu | 21.103s | 53718 | 4.925s | 0 |

La reduccion focal fue **48.2%**. Cambian transporte, distribucion y ubicacion
de datos; la comparacion no atribuye toda la diferencia a la red. Tampoco
acredita una reduccion equivalente del CI completo bajo concurrencia.
La optimizacion temporal de compilacion anterior no mostro una ganancia:
82.59s de compilacion y 40.86s en la prueba; no se aplico al repositorio.

El nuevo supervisor conserva PostgreSQL16, SCRAM, loopback, limite6GiB y
bases independientes. Las cinco pruebas de control se escribieron primero,
fallaron por ausencia del helper y luego aprobaron **5/5**. Cubren fallo de
arranque, propagacion del resultado, cancelacion, rechazo de limpieza ajena
y estado de salida por SIGTERM.

En VPS3, el supervisor real verifico rechazo de una contrasena incorrecta,
aislamiento entre sus tres bases, SCRAM, loopback sin socket Unix y el limite
systemd de 6GiB. La prueba de catalogo aprobo **1/1**; supervisor mas prueba y
limpieza tardaron **21.45s**. Otros dos recorridos focales confirmaron que un
comando fallido conserva salida **7** y SIGTERM conserva salida **143**.
En los tres casos se cerraron el puerto y el servicio, se retiro el marcador
y se elimino solo el cluster propio. La campana completa con Rust y navegador
simultaneos queda pendiente; no se aplican nuevas cifras de cobertura aun.

## Campana completa en servidores propios: 27 de septiembre de 2026

La cabeza `5e1bc9b` aprobo [CI 36295738154](https://github.com/eddndev/TT2026-B136/actions/runs/36295738154)
y [Web 36295738149](https://github.com/eddndev/TT2026-B136/actions/runs/36295738149),
incluidos sus checks agregados. Los JUnit contienen **3049 Rust**, **359 de
navegador simulado** y **43 reales** aprobados, sin fallos ni duplicados.
Nextest conserva las dos pruebas ignoradas previamente declaradas; no forman
parte de las 3049 ejecutadas. La cobertura por lineas aprobo los umbrales de
90%: domain **97%**, application **95%**, infrastructure **93%**.

| Medicion | Tiempo |
| --- | --- |
| CI completo, desde creacion hasta ultimo check | 13m12s |
| Web completo, desde creacion hasta ultimo check | 11m15s |
| Ejecucion Rust instrumentada | 738.57s |
| Compilacion instrumentada caliente | 0.19s |
| Generacion del reporte de cobertura | aproximadamente 6s |
| Gate agregado de cobertura | 8s |
| Navegador real en VPS3, 13 casos, job completo | 6m46s |

El coste dominante sigue siendo ejecutar Rust, no compilar ni generar el
reporte. Los slots acumulan 11653.64 segundos de pruebas; divididos entre
16 dan 728.35s, cercanos a los 738.57s medidos. Reordenar las mismas pruebas
sin reducir su coste dificilmente eliminaria los tres minutos restantes.
El objetivo aproximado de diez minutos sigue pendiente; esta campana es el
checkpoint completo reproducido para comparar cambios posteriores.

Durante sus ventanas de jobs, VPS3 promedio **6.59 CPU**, alcanzo **27.73 GiB**
y registro ocho eventos MemoryHigh, sin OOM ni swap. VPS2 promedio **3.12 CPU**,
alcanzo **5.78 GiB**, sin MemoryHigh u OOM. VPS1 alcanzo **4.49 GiB**, registro
918 eventos MemoryHigh y un evento max, sin OOM; su swap pico fue 0.29 GiB.
No se aumentaron recursos. La cache y los artefactos permanecen conservados.

Una comprobacion focal posterior ejecuto exactamente una prueba costosa de
integridad del catalogo con PostgreSQL desechable y pg_stat_statements:
**1/1 PASS en 40.80s**, 53718 llamadas y cero funciones compiladas mediante
JIT. No acredita el conjunto ni reproduce la concurrencia de CI; no aporta
evidencia para desactivar JIT y ese ajuste no se aplico.

## Sincronizacion de respuesta MFA: 27 de septiembre de 2026

[Web 36294895911](https://github.com/eddndev/TT2026-B136/actions/runs/36294895911)
aprobo los **13 escenarios** de VPS3 simultaneamente con Rust en **6m42s**.
La compilacion caliente tardo 0.15s y el navegador 224.96s; administracion
aprobo en 37.18s y participantes en 29.44s, ambos dentro de sus 60s originales.
VPS3 promedio 6.61 CPU y alcanzo 27.21 GiB sin MemoryHigh, OOM ni swap. Esto
confirma esa particion, no el conjunto: la cancelacion posterior interrumpio
los demas gates.

La segunda particion en VPS2 aprobo nueve escenarios y fallo al iniciar
`hearing-sentencing.spec.mjs`. El helper comprobo el encabezado de inicio
durante cinco segundos mientras la pagina seguia mostrando MFA en estado
`Verificando...`; no se habia recibido una respuesta de autenticacion fallida.
La captura no permite afirmar que esa solicitud finalmente habria aprobado.
Una regresion controlada usando el helper real reprodujo el fallo con una
respuesta MFA valida demorada seis segundos.

El helper ahora espera la respuesta POST de recuperacion, exige HTTP 200 y
despues comprueba el mismo encabezado. La espera HTTP usa el presupuesto
existente de Playwright; la assertion visual conserva cinco segundos y cada
escenario real conserva su limite total de 60s. No reintenta solicitudes ni
cambia producto, criptografia o recursos. La regresion fallo antes del cambio
y luego aprobo **1/1 en 13.0s**, incluido el arranque local, con un worker.
Formato, ASCII, limite de lineas y diff aprobaron. El listado conserva los
358 casos simulados anteriores y agrega solo esta regresion: **359** en total.
Los 43 escenarios reales no cambian; la validacion real conjunta sigue pendiente.

## Reequilibrio de familias reales: 27 de septiembre de 2026

[Web 36293965586](https://github.com/eddndev/TT2026-B136/actions/runs/36293965586)
confirmo el checkout corregido y aprobo los **12 escenarios** de la tercera
particion en VPS3, simultaneamente con Rust. Su job completo tardo **6m52s**:
47.43s de compilacion y 191.58s de navegador, mas preparacion, servicios y
publicacion de resultados. Participantes aprobo en **35.23s**, dentro de sus
60s originales. Esto acredita esa particion, no la campana completa.

La primera particion en VPS2 agoto los 60s de administracion de expedientes
tras recorrer conflictos y cierre; sus respuestas fallidas fueron solo los
409 esperados. La cancelacion detuvo el resto. La preparacion administrativa
habia tardado 50s. VPS2 promedio **2.94 CPU**, alcanzo **5.32 GiB**, sin eventos
MemoryHigh, OOM ni throttling de cuota; registro 53.99s de presion CPU parcial
y 15.95s completa durante la ventana de jobs. En VPS3 el conjunto promedio
**6.74 CPU**, alcanzo **26.96 GiB**, sin MemoryHigh, OOM ni swap. No se aumentan
recursos ni slots a partir de esta campana parcial.

Se mueve solo la familia administrativa, con su preparacion, a la tercera
particion. Etapas/plazos permanecen en la primera. No se agrega otro runner
ni se modifican escenarios, timeouts, assertions, producto o criptografia.
La comprobacion nueva del plan fallo antes del cambio y despues aprobaron
**11/11** comprobaciones focales de seleccion y autenticacion de fixtures.
Los listados reales de Playwright preservan las mismas **43 identidades** de
la referencia completa, ahora **13/17/13**, sin duplicados. Formato, ASCII,
limite de lineas y diff aprobaron. El escenario administrativo sin cambios
aprobo **1/1 en 22.3s** en VPS3, usando la nueva preparacion de su particion y
el limite original de 60s. Esta comprobacion focal fue aislada de Rust;
la campana conjunta sigue pendiente.

## Permisos del checkout del runner: 27 de septiembre de 2026

[Web 36293132814](https://github.com/eddndev/TT2026-B136/actions/runs/36293132814)
fallo en ocho segundos durante el checkout del nuevo runner de VPS3, antes
de ejecutar pruebas. La preparacion manual habia dejado `output/` con
propietario root y modo 0755, aunque su hijo `output/tmp` pertenecia al runner.
Una comprobacion como `tt-runner` reprodujo `PermissionError` al crear un
directorio dentro de ese padre. Se corrigio exclusivamente su propietario;
despues aprobaron la creacion y eliminacion recursiva de un directorio de
prueba y la comprobacion de escritura/recorrido de todos los directorios.
El target persistente de Cargo permanece intacto.

La cancelacion automatica detuvo los demas jobs; el ultimo termino 38 segundos
despues del job fallido. Los diagnosticos de navegador subidos por ese job
eran restos de la comprobacion focal anterior, no resultados de esta campana;
se retiraron del checkout. Esta ejecucion no acredita pruebas ni rendimiento.
No se cambiaron producto, pruebas, timeouts o recursos. El checkout completo
y la ejecucion concurrente quedan pendientes de la siguiente campana.

## Reparto de navegador entre servidores: 27 de septiembre de 2026

[Web 36291943072](https://github.com/eddndev/TT2026-B136/actions/runs/36291943072)
repitio el timeout total de participantes, ahora al final de los controles de
permisos. La cancelacion automatica detuvo ambos workflows. En la ventana
activa muestreada, VPS2 obtuvo **4.15 CPU** y alcanzo **6.33 GiB**, con cero
eventos nuevos MemoryHigh u OOM y 0.04 GiB de swap. La prioridad elimino la
presion de memoria observada antes, pero no resolvio el tiempo del escenario.
No se acredita una campana completa ni una mejora del total.

La matriz ahora asigna las particiones reales 1 y 2 a VPS2 y la 3 a un runner
independiente de VPS3. Conserva los mismos 43 escenarios, fixtures, un worker
por job y todos los gates. VPS3 mantiene sus 16 slots Rust y ocho CPU; el
presupuesto conjunto pasa a MemoryHigh 28 GiB y MemoryMax 30 GiB para incluir
el navegador, dejando 2 GiB fuera del limite para el sistema. La ventana Rust
anterior alcanzo 24 GiB, sin OOM ni swap. Se medira la contencion conjunta en
la siguiente campana antes de acreditar el cambio.

Se prepara una sola vez el target independiente del nuevo runner antes de
medir ejecuciones calientes. Ese coste de instalacion y compilacion se
registra aparte; no se presenta como parte de una mejora del tiempo frio.
La compilacion inicial del target nuevo termino en **1m54s** usando un
compilador. Chromium abrio una pagina y PostgreSQL/Redis desechables pasaron
la comprobacion con el usuario real del runner. La matriz paso Actionlint;
no se repitio una suite local. La comprobacion focal inicial detecto Redis
6.0 del sistema sin `GETDEL` y fallo en MFA antes del navegador; se actualiza
la dependencia a **Redis 7.4.11** desde el repositorio oficial, sin alterar
el adaptador de identidad. El mismo escenario completo de participantes
aprobo **1/1 en 21.7s** con su presupuesto original de 60s. Las familias
participantes, hechos, recursos y actividades se prepararon en 35s, 16.00s,
16.26s y 17.40s respectivamente; hechos/recursos compartieron un lote. Esta
comprobacion fue aislada, sin la suite Rust simultanea, por lo que no demuestra
todavia el tiempo del reparto completo.

## Sincronizacion de navegacion del cliente: 27 de septiembre de 2026

[Web 36291240886](https://github.com/eddndev/TT2026-B136/actions/runs/36291240886)
se detuvo en `deadline-navigation.spec.mjs`: el test forzaba el hash de plazos
inmediatamente despues del clic en un expediente. Comprobar que no existia
un enlace privado podia cumplirse antes de terminar `Cases.open`, que espera
la respuesta de detalle y despues navega al resumen. Esa respuesta pendiente
podia sobrescribir la redireccion a Inicio del hash denegado.

La prueba ahora espera el encabezado del resumen antes de forzar el hash.
Mantiene el enlace privado ausente, Inicio seleccionado y cero solicitudes
privadas; no se cambio el producto, el timeout ni la cantidad de pruebas.
La ejecucion focal con un worker aprobo **1/1** (19.9s incluyendo arranque);
formato, ASCII y diff tambien pasaron. No se repitio una suite completa.
La cancelacion cruzada detuvo Web y CI; esta campana no permite concluir si
las prioridades nuevas resuelven el timeout previo del navegador real.

## Preparacion reducida y contencion del host: 27 de septiembre de 2026

En [Web 36290455092](https://github.com/eddndev/TT2026-B136/actions/runs/36290455092)
las tres particiones superaron la autenticacion corregida y llegaron al
navegador. La tercera preparo sus familias y servicios en aproximadamente
cinco minutos, frente a los quince anteriores; la primera llego al navegador
6m46s despues del inicio del workflow. No es todavia una medicion de Web
completo: participantes agoto su presupuesto total de 60s al volver a iniciar
sesion despues de verificar ediciones, archivo, historial y evidencia. Los
unicos errores HTTP registrados fueron los conflictos 409 esperados.
El mismo escenario habia aprobado en 30.113s en la campana completa previa.

Durante la ventana muestreada del fallo, el grupo de CI en VPS2 obtuvo
**4.05 CPU** de seis, sin throttling de cuota. El host tenia **8.5%** de CPU
ociosa y el grupo acumulo **21.45s** de presion parcial de CPU. Alcanzar
MemoryHigh de 6 GiB produjo **440** eventos y **0.60s** de presion completa
de memoria, con un pico de **0.14 GiB** de swap y sin OOM. La presion de CPU
es mayor que la de memoria. El runner original conservaba Nice=5; los dos
adicionales usaban Nice=0. Esto no prueba por si solo la causa de todo el
retraso, pero justifica medir prioridad consistente en el host compartido.

La siguiente configuracion iguala Nice=0, aplica CPUWeight=1000 al slice de
CI y mueve MemoryHigh a 6.5 GiB. Conserva seis CPU de cuota, MemoryMax=7 GiB,
los tres workers, las pruebas y sus timeouts. Se comprobara su efecto en la
siguiente ejecucion; no se acredita aun una reduccion del total ni un pase.
Ambos workflows anteriores se cancelaron automaticamente tras el fallo.
No se repitieron suites locales por este cambio operativo.

## Recuperacion del provisionador y cancelacion cruzada: 27 de septiembre de 2026

La preparacion selectiva publicada en `3ccf617` fallo antes del navegador:
solicitaba el indice 8 de un conjunto de ocho codigos (indices 0 a 7), por
lo que la peticion MFA omitia el campo y recibia HTTP 422. La correccion usa
el codigo reservado del Owner de etapas cuando ese fixture existe; cuando
no existe, usa el codigo de bootstrap que etapas no consumio.

Cuatro pruebas focales importan el script real con transporte HTTP simulado,
una por particion y otra sin particion. Las cuatro reprodujeron primero el
codigo inexistente; tras la correccion pasaron **4/4**. Comprueban pertenencia
al conjunto emitido, ausencia de colision con otros consumidores, uso de la
sesion y cierre al fallar la preparacion. No ejecutan PostgreSQL ni acreditan
la regresion completa remota, que sigue pendiente.

El hook de [Web 36289758863](https://github.com/eddndev/TT2026-B136/actions/runs/36289758863)
solicito cancelar ambos workflows a las 02:53:50 UTC. Web y
[CI 36289758856](https://github.com/eddndev/TT2026-B136/actions/runs/36289758856)
terminaron cancelados a las 02:54:07. Esta ejecucion confirma la cancelacion
cruzada con CI todavia activo; no representa suites completas aprobadas.

## Cache caliente y preparacion selectiva del navegador: 27 de septiembre de 2026

[CI 36287708210](https://github.com/eddndev/TT2026-B136/actions/runs/36287708210)
aprobo en **11m47s**; las **3049** pruebas Rust y los gates pasaron.
Las **358** pruebas simuladas tambien aprobaron. En
[Web 36287708222](https://github.com/eddndev/TT2026-B136/actions/runs/36287708222),
el escenario real completo de administracion agoto sus 60s al llegar a la
comprobacion de permisos. No se observo una respuesta de acceso indebido:
el diagnostico registra los dos conflictos 409 esperados. La cancelacion
automatica se solicito a las 02:27:53 UTC y los jobs restantes terminaron
a las 02:28:12. CI ya habia concluido; se observo la cancelacion del workflow
actual, no una cancelacion cruzada de otro workflow activo.

La primera particion compilo en **2.93s**, pero luego preparo datos durante
aproximadamente **15m17s**. En esa ventana VPS2 promedio **4.03 CPU**, con
solo **0.01s** de throttling, 79.57s de presion completa de CPU y 0.55% de
espera de E/S del host. Memoria maxima 5.95 GiB, swap 0.13 GiB y cero OOM.
La cuota mayor elimino la limitacion anterior; no elimino la preparacion
duplicada de todas las familias en los tres jobs.

La nueva seleccion prepara solo las familias de los archivos asignados.
Diez comprobaciones focales de planificacion y lotes aprobaron; se verifico
primero el fallo de los nuevos casos. Playwright enumero **14 + 17 + 12 = 43**
pruebas con las mismas identidades de la ultima campana real completa, sin
duplicados. La extraccion de los comandos de preparacion de audiencias
conserva su contenido. No se ejecuto una regresion completa local ni se
cambiaron timeouts, aserciones o costes de credenciales. La ejecucion real y
la mejora de tiempo con esta preparacion selectiva quedan pendientes.

## Primera medicion en los tres VPS y cancelacion: 27 de septiembre de 2026

[CI 36286314852](https://github.com/eddndev/TT2026-B136/actions/runs/36286314852)
aprobo en **13m13s**. El job Rust duro **11m42s**; Coverage espero al runner
de soporte. La compilacion release fria tomo **7m10s**, seguida por los
checks restantes en ese mismo runner. No fue una regresion Rust de veinte
minutos.

[Web 36286314906](https://github.com/eddndev/TT2026-B136/actions/runs/36286314906)
tuvo el fallo de sincronizacion descrito abajo y se cancelo. Los jobs reales
seguian preparando servicios y fixtures. En el primero, compilar Rust desde
cero tomo **6m57s**. Las tres particiones repetian esa preparacion bajo una
cuota conjunta de cuatro CPU. Durante la ventana de fixtures muestreada,
VPS2 promedio **3.62 CPU**, acumulo **276.35s** de throttling y **107.98s** de
presion completa de CPU; la espera de E/S del host fue **0.45%** y no hubo
OOM. El muestreo completo registro 4141 eventos MemoryHigh y hasta 0.31 GiB
de swap en el grupo del runner. CPU y compilacion fria explican el retraso;
no hay evidencia de disco como cuello dominante.

La siguiente campana conserva los targets ya compilados y eleva el presupuesto
conjunto de VPS2 a **6 CPU**, MemoryHigh **6 GiB** y MemoryMax **7 GiB**,
dentro de sus seis vCPU y aproximadamente doce GiB fisicos. El efecto sobre
tiempo, presion y servicios compartidos sigue pendiente de medicion.

Se configuro cancelacion de CI/Web de la misma revision al fallar un check,
fail-fast de matrices/Nextest y un fallo maximo de Playwright. La suite focal de cinco casos
del helper de cancelacion fallo antes de implementarlo y aprobo despues, cubriendo alcance y errores de API; no cancelan runs reales. La
cancelacion automatica completa aun debe observarse en Actions cuando haya
un fallo. No se redujo ningun test, asercion ni umbral de cobertura.

## Espera de preparacion de actividades: 27 de septiembre de 2026

La primera campana completamente alojada en los VPS encontro un fallo en
`resource-activities-conflict.spec.mjs`: esperaba revision 4 pero leyo 3 de
la peticion anterior. El test inspeccionaba inmediatamente el registro de
peticiones despues del click de preparacion asincrona. La interfaz solo
muestra `Confirmar vinculo` cuando recibe y valida el nuevo borrador.

La correccion espera ese boton antes de inspeccionar el comando, igual que
la prueba existente de recuperacion del mismo flujo. Mantiene todas las
aserciones sobre revision actual, fuentes historicas y confirmacion, sin
cambiar producto, timeouts ni reintentos. La prueba exacta aprobo **1/1 en
9.4s**, con un worker local y temporales en disco. Prettier, ASCII, tamano y
diff checks aprobaron. La regresion remota de esta correccion queda pendiente;
la campana previa continua para conservar sus resultados y caches frias.

## Regresion completa y bloqueo de jobs finales: 27 de septiembre de 2026

En la cabeza `14dcf1d`, [CI 36284539265](https://github.com/eddndev/TT2026-B136/actions/runs/36284539265)
ejecuto correctamente las **3049** pruebas Rust en **652.355s**, con dos
ignoradas y compilacion instrumentada reutilizada de **0.16s**, y
[Web 36284539264](https://github.com/eddndev/TT2026-B136/actions/runs/36284539264)
ejecuto correctamente las **358** simuladas y **43** reales. La comparacion
de JUnit confirma las mismas identidades, sin duplicados ni fallos, respecto
a la campana completa anterior. La correccion de sincronizacion de
`stage-adoption.spec.mjs` queda asi verificada en el navegador real.

Ambos workflows terminaron con fallo porque **Coverage** y el agregado
**Browser with real services** no iniciaron. GitHub atribuyo el rechazo a
pagos recientes fallidos o al limite de gasto; ambos jobs carecen de pasos
ejecutados. No es un fallo de las pruebas. Como comprobacion adicional se
ejecuto el gate local sobre el LCOV descargado de esa misma cabeza:
domain **97%**, application **95%**, infrastructure **93%**, todos sobre
el umbral de 90%. Esa comprobacion no sustituye los gates de Actions.

La siguiente configuracion traslada CI y Web a los servidores propios, segun
[ADR 0052](adr/0052-owned-ci-runners.md). Los siete runners se registraron
en linea. Chromium abrio una pagina tanto en el contenedor de VPS1 como
nativamente en VPS2. Actionlint y la sintaxis del nuevo helper de cache
aprobaron; el helper tambien creo caches y temporales accesibles con las
cuentas reales de los runners. No se repitieron suites completas locales.
Quedan pendientes la
campana remota con esta topologia y la comparacion de tiempos frios/calientes;
no se atribuye una mejora de rendimiento a la migracion sin medirla.

## Dieciseis slots y sincronizacion de navegacion: 27 de septiembre de 2026

[CI 36283718718](https://github.com/eddndev/TT2026-B136/actions/runs/36283718718)
aprobo en **11m54s**. Las **3049** pruebas Rust pasaron, con dos ignoradas,
en **665.174s** y compilacion instrumentada reutilizada de **0.15s**. Todos
los gates de CI aprobaron. La mejora frente a doce slots fue de veinte
segundos; no justifica seguir aumentando paralelismo sin otra causa medida.

CPU promedio **6.56**, mayor promedio de muestra **7.14**, memoria cargada
maxima **21.05 GiB**, PostgreSQL **3.79 GiB** y conexiones maximas **56**.
No hubo OOM, swap ni eventos de limite de memoria. El throttling acumulado
fue 0.0015s. PSI de memoria registro menos de un milisegundo de presion; el
muestreo no muestra saturacion de memoria o E/S. Estos datos no equivalen a
que todas las pruebas usen ocho cores de forma continua.

[Web 36283718728](https://github.com/eddndev/TT2026-B136/actions/runs/36283718728)
termino con fallo: las **358** pruebas simuladas y **42 de 43** reales pasaron.
La identidad exacta de las suites se conserva. `stage-adoption.spec.mjs`
esperaba el inicio tras forzar una ruta de personal para un cliente, pero la
captura mostro el resumen del expediente. El test cambiaba el hash justo
despues del click que inicia una consulta asincrona; comprobar que el enlace
no existe no esperaba a que concluyera esa apertura. Su finalizacion podia
sobrescribir la navegacion que la prueba intentaba comprobar.

La correccion espera el encabezado del resumen antes de comprobar los enlaces
y forzar la ruta. Conserva el retorno esperado al inicio y la asercion de cero
consultas a etapas, sin ampliar timeouts ni agregar reintentos. La prueba
focal existente `Client neither navigates nor requests staff stages` aprobo
con un worker local; ya empleaba esa misma espera. Prettier y diff checks
aprobaron. La prueba real corregida y la regresion de la nueva cabeza quedan
pendientes en CI; no se repitio una suite completa local.

## Doce slots y cache Web caliente: 27 de septiembre de 2026

[CI 36282244911](https://github.com/eddndev/TT2026-B136/actions/runs/36282244911)
aprobo en **12m14s** desde creacion hasta actualizacion final. Las **3049**
pruebas Rust pasaron, con dos ignoradas, en **683.441s**; la compilacion
instrumentada reutilizada tomo **0.17s**. La generacion de cobertura tomo
unos seis segundos y los gates continuaron aprobados. La identidad exacta
de todas las pruebas coincide con la campana anterior.

[Web 36282244873](https://github.com/eddndev/TT2026-B136/actions/runs/36282244873)
aprobo en **10m03s**, con las mismas **358** pruebas simuladas y **43** reales.
La cache caliente redujo la compilacion real a **22.43-50.48s**. El job real
mas lento compilo en 43.32s, preparo fixtures en aproximadamente 4m44s y
corrio el navegador en 3.4 minutos. Web alcanzo el objetivo aproximado una
vez; falta confirmar estabilidad tras integrar. Rust todavia supera ese tiempo.

El muestreo de pruebas registro CPU promedio **6.19**, maximo promedio de
muestra **6.69**, y throttling acumulado **11.42s**. PostgreSQL alcanzo
**46** conexiones: predominan estados de espera del cliente y trabajo activo,
con solo dos observaciones de espera por bloqueo consultivo. Los intervalos
son muestras de 15s, no un registro exhaustivo de cada espera.

La memoria cargada al padre alcanzo **23.93 GiB**: aproximadamente 21.02 GiB
de archivos, 2.44 GiB de memoria de kernel recuperable y 0.44 GiB anonima.
Hubo nueve eventos MemoryHigh y ninguno de limite maximo u OOM, sin swap.
PostgreSQL alcanzo 3.64 GiB dentro de sus 6 GiB. El host registro 0.032% de
espera por E/S y 0.003% de steal; no se atribuye el retraso a disco saturado.

La siguiente medicion usa **16 slots** y una cuota de **8 CPU**, manteniendo
26 GiB de memoria maxima y 24 GiB para MemoryHigh. Las cinco comprobaciones
focales de aislamiento y rechazo de indices invalidos fallaron primero con
el limite anterior y luego aprobaron. Actionlint aprobo. La regresion remota
y las mediciones con esta configuracion siguen pendientes; no se afirma que
16 slots cumplan ya el objetivo ni se repitio una suite completa local.

## Memoria PostgreSQL y tres particiones Web: 27 de septiembre de 2026

La ejecucion [CI 36281044569](https://github.com/eddndev/TT2026-B136/actions/runs/36281044569)
aprobo en **13m52s**, con **3049** pruebas aprobadas y dos ignoradas. Las
pruebas instrumentadas tomaron **781.391s**, compilacion reutilizada **0.25s**
y generacion del informe unos seis segundos. Los gates conservaron el 90%
por crate; dominio, aplicacion e infraestructura mostraron **97/95/93%**.
No hay mejora material de Rust respecto de la ejecucion anterior.

[Web 36281044607](https://github.com/eddndev/TT2026-B136/actions/runs/36281044607)
aprobo en **11m33s**, frente a 13m25s. La union JUnit conserva **358** pruebas
simuladas y **43** reales, sin duplicados ni fallos. El reparto real fue
**15+16+12**. Los jobs reales compilaron en 2m10s a 2m15s; el mas lento preparo
fixtures en unos cinco minutos y ejecuto navegador en 3.3 minutos. La cache
con clave nueva estaba fria y quedo guardada; su mejora con cache caliente
sigue pendiente de medir.

La memoria cargada al cgroup padre alcanzo **21.35 GiB** y PostgreSQL
**3.44 GiB**, con limite de 6 GiB. No hubo eventos de limite, OOM ni swap.
CPU promedio **5.44 cores**, mayor promedio de muestra de 15s **5.97**, y
throttling acumulado **1.06s**. La ampliacion elimino la presion del limite
previo, pero no redujo el tiempo de Rust. Docker/rootless uso 1.06 cores.

Una microprueba desechable de ocho clientes consultando `SELECT 1` comparo
TCP con socket Unix, alternando dos muestras de diez segundos por transporte.
TCP obtuvo 40.3-41.3 mil consultas/s y socket 42.1-43.2 mil consultas/s. La
pequena diferencia no demuestra un cuello de botella de transporte en la
suite real; no se cambia su conexion basandose solo en esa microprueba.

Siguiente medicion: doce slots, mismos limites de 7 CPU y 26 GiB, PostgreSQL
6 GiB. Cinco comprobaciones focales de URLs independientes, indices Redis y
rechazo de configuraciones invalidas pasaron tras comprobar primero el fallo
con doce slots. Actionlint aprobo. La regresion remota y el muestreo de esperas
PostgreSQL quedan pendientes; no se repitio una suite completa local.

## Ocho slots y limites por servicio: 26 de septiembre de 2026

La ejecucion [36279736231](https://github.com/eddndev/TT2026-B136/actions/runs/36279736231)
aprobo todas las pruebas y cobertura, en aproximadamente **13m50s** desde la
creacion hasta finalizar Coverage. Los **3049** casos pasaron, con dos
ignorados, en **782.456s**; el build instrumentado reutilizado tomo **0.17s**.
La union de reportes Web conserva **358** pruebas simuladas y **43** reales,
todas aprobadas. El [workflow Web](https://github.com/eddndev/TT2026-B136/actions/runs/36279736239)
tardo aproximadamente **13m25s**: no mejoro materialmente frente a 13m22s.
En el shard real mas lento, la compilacion tomo 2m16s, la preparacion posterior
5m04s y las pruebas cinco minutos. La nueva concurrencia de fixtures paso la
regresion, pero no se acredita como mejora de tiempo global.

El muestreo de ocho slots registro **5.39 CPU** de promedio, maximo promedio
de muestra **6.16**, y **0.82s** acumulados de throttling durante el intervalo.
La memoria cargada al grupo llego a **18.18 GiB**, sin OOM ni swap. Los contadores
locales identificaron al contenedor PostgreSQL: llego a su limite de **2 GiB**
y registro **9175** eventos `memory.events.local:max`. El cgroup padre no
registro eventos max locales. El contenedor PostgreSQL uso unas 3.23 CPU y el
servicio Docker/rootless alrededor de 1.03 CPU. Son observaciones; el efecto
de ampliar el limite aun requiere una nueva medicion.

El siguiente ajuste aumenta PostgreSQL a **6 GiB**, dentro del presupuesto
conjunto de 26 GiB, y conserva ocho slots. Web real pasa a tres shards con
cache de crates del workspace ademas de dependencias; siempre ejecuta Cargo
sobre la revision seleccionada. Actionlint y la enumeracion disjunta de todas
las pruebas reales validan el reparto; la regresion remota queda pendiente.
No se cambiaron tests, umbrales de cobertura ni limites de produccion.

## Seis slots y navegador dividido: 26 de septiembre de 2026

La ejecucion [36278172116](https://github.com/eddndev/TT2026-B136/actions/runs/36278172116)
aprobo en **14m47s**, con **3049/3049** pruebas y dos ignoradas. La ejecucion
instrumentada tomo **839.417s** y reutilizo compilacion (0.16s). La mejora
sobre los 16m45s calientes anteriores fue de 1m58s; no alcanzo diez minutos.

El [workflow Web](https://github.com/eddndev/TT2026-B136/actions/runs/36278172087)
aprobo en **13m22s**. Los reportes JUnit confirman las **358** pruebas simuladas
y **43** reales sin fallos. El gate de las simuladas termino en 6m46s. En la
particion real mas lenta, la compilacion tomo 2m27s, la preparacion posterior
unos 5m12s y el navegador 4.6 minutos. La preparacion domina ahora ese camino.

Durante el tramo de pruebas, el cgroup conjunto de VPS3 uso **4.79 CPU** de
promedio y **5.24 CPU** como mayor promedio de muestra de 15 segundos. No hubo
throttling de CPU, OOM ni swap. El maximo de memoria cargada al cgroup fue
**16.60 GiB**, incluyendo hasta 14.80 GiB de cache de archivos; no equivale a
RSS. Aumentaron eventos jerarquicos de limite de memoria sin alcanzar el
limite agregado: la siguiente medicion recoge tambien los subgrupos para
identificar su origen. No se atribuyen esos eventos a un servicio sin evidencia.

Siguiente ajuste: ocho slots Rust conservando limites del host, preparacion
remota de fixtures independientes en lotes de dos y debug de lineas para el
build del navegador. Cuatro tests focales de concurrencia, orden local,
rechazo de limites y drenaje ante fallo fallaron antes de agregar el helper y
aprobaron despues. Actionlint y diff checks aprobaron. La siguiente regresion
remota queda pendiente; no se repitio una suite completa local.

## Medicion completa y siguiente ajuste de CI: 26 de septiembre de 2026

La campana [36275446647](https://github.com/eddndev/TT2026-B136/actions/runs/36275446647)
aprobo con cuatro slots: **19m55s** con compilacion fria, **3049/3049** pruebas
ordinarias aprobadas y dos ignoradas, **953.784s** de ejecucion instrumentada.
Los gates por crate conservaron el 90% exigido; los porcentajes mostrados
fueron 97% dominio, 95% aplicacion y 93% infraestructura.

La siguiente ejecucion de main,
[36276646491](https://github.com/eddndev/TT2026-B136/actions/runs/36276646491),
aprobo en **16m45s** con cache reutilizada. Su
[workflow Web](https://github.com/eddndev/TT2026-B136/actions/runs/36276646536)
aprobo en **14m01s**: 358 pruebas con API simulada tomaron 12 minutos; 43
pruebas con servicios reales tomaron 7.5 minutos, mas preparacion de fixtures.
Estas mediciones sustituyen el estado pendiente de las secciones historicas.

El siguiente ajuste propone seis slots Rust y dos particiones por suite de
navegador, manteniendo los gates y un worker por particion. La validacion
estatica de workflows y los **19/19** tests de helpers CI aprobaron localmente.
La enumeracion de Playwright comprueba que las particiones son disjuntas y
que su union conserva las 358 y 43 pruebas originales. Esto comprueba la
seleccion, no sustituye la ejecucion remota. El tiempo con seis slots sigue
pendiente; no se afirma todavia un CI de diez minutos.

## Validacion del nuevo host principal de CI: 26 de septiembre de 2026

El nuevo host Ubuntu 22.04, con 8 vCPU y 32 GB, cuenta con un runner exclusivo
del repositorio y Docker rootless bajo la misma slice de systemd: 7 CPU,
`MemoryHigh=24G`, `MemoryMax=26G`. Una cuenta independiente queda disponible
para futuros servicios. Los runners anteriores se conservan como respaldo.

Se instalaron CPython 3.12.14 separado del interprete del sistema, clientes
PostgreSQL 16.15, Rust 1.98.1, cargo-llvm-cov 0.9.1, Nextest 0.9.146 y qpdf
12.4.1. El interprete satisface la comprobacion con entorno vacio. Los
archivos de Python, runner, herramientas Cargo y qpdf se verificaron contra
los hashes de sus publicaciones antes de utilizarlos.

`scripts/tests/check_nextest_pipeline.py --slots 4` aprobo **6/6 pruebas en
0.604 s**, con **0.32 s** de compilacion del workspace pequeno. Cuatro casos
de backend se sincronizaron para ejecutar simultaneamente y tomaron la misma
clave consultiva contra bases distintas, sin superar el limite de bloqueo.
PostgreSQL y Redis fueron contenedores desechables; ambos se retiraron al
finalizar. Tambien se verificaron LCOV fresco y seis resultados JUnit.
Es una comprobacion del aislamiento y del entorno, no un tiempo de CI del
proyecto. La regresion completa y el objetivo de 10-20 minutos siguen pendientes.

## Preparacion del runner dedicado: 26 de septiembre de 2026

El modo opcional de [ADR 0050](adr/0050-isolated-test-slots.md) programa pruebas
individuales con Nextest 0.9.146 y bases PostgreSQL e indices Redis por slot.
La configuracion inicial usa cuatro slots, conserva todos los tests ordinarios
y la cobertura requerida, y deja activos los dos runners anteriores mientras
la variable `TT_CI_DEDICATED` no sea `true`. La provision del nuevo host y su
regresion completa siguen pendientes; no hay una mejora de tiempo global
medida ni se declara alcanzado el objetivo de 10-20 minutos.

Verificacion fresca, con una sola suite local a la vez y `output/tmp` sobre
el filesystem de disco del checkout:

- Los nuevos casos de aislamiento y conteo de reportes fallaron antes de su
  implementacion; tambien fallo inicialmente el caso que permite cuatro
  compiladores conservando el limite de memoria. Despues aprobaron **19/19**
  tests de los helpers CI.
- `scripts/tests/check_nextest_pipeline.py`, ejecutado mediante
  `scripts/test-backends.sh`, compilo un workspace pequeno y aprobo **3/3**
  pruebas reales con un slot, PostgreSQL, Redis y cargo-llvm-cov. Verifico los
  URLs aislados, el reporte LCOV y tres resultados JUnit. La primera prueba
  detecto una ruta equivocada para JUnit; se fijo explicitamente el directorio
  de Nextest y la comprobacion completa posterior aprobo.
- La prueba real de infraestructura
  `deadline_worker_guards::completion_audit_failure_rolls_back_revision_and_result_but_records_retry`
  aprobo **1/1 en 6.641 s** bajo Nextest, con un slot y los servicios
  desechables. Las otras 34 pruebas del ejecutable se filtraron para esta
  comprobacion focal. La compilacion reutilizada tomo 0.16 s. Es evidencia
  local del caso, no una medida de coverage ni de la campana remota completa.
- actionlint, sintaxis Python/TOML, ASCII de scripts y `git diff --check`
  aprobaron. No se repitio la suite completa ni el navegador por este cambio
  de infraestructura de pruebas.

## Sincronizacion de la navegacion del Cliente: 26 de septiembre de 2026

El job de navegador de la ejecucion
[36264358697](https://github.com/eddndev/TT2026-B136/actions/runs/36264358697)
aprobo 42 de 43 pruebas. La comprobacion de permisos de participantes
cambiaba el hash inmediatamente despues de pulsar un expediente, sin esperar
la consulta asincrona de su detalle. La captura del fallo mostro el resumen
del expediente donde se esperaba el inicio: la apertura pendiente podia
competir con la navegacion de prueba hacia una ruta no permitida.

El helper espera ahora que aparezca `Resumen del expediente` antes de
intentar esa ruta como Cliente. Conserva la exigencia de volver al inicio,
la ausencia de solicitudes de participantes y los limites de espera. No
cambia la navegacion ni la autorizacion de la aplicacion.

La prueba integrada `case-participants.spec.mjs` aprobo **1/1**, con un
worker de navegador y servicios Rust, PostgreSQL y Redis desechables. El caso
tomo 24.6 s y Playwright completo su ejecucion en 29.8 s; esos tiempos no
incluyen la compilacion ni la preparacion de los servicios y fixtures.
Prettier, la comprobacion sintactica de Node y `git diff --check` aprobaron.
La campana completa del navegador sigue pendiente para esta correccion.

## Contencion entre fixtures de infraestructura: 26 de septiembre de 2026

En la ejecucion [36262186578](https://github.com/eddndev/TT2026-B136/actions/runs/36262186578),
`Test (1/2)` y ambos jobs de Web aprobaron. `Test (2/2)` fallo en
`deadline_worker_guards::completion_audit_failure_rolls_back_revision_and_result_but_records_retry`:
la preparacion de su despacho encontro `ClassifiedPort::Busy` antes de
inyectar el fallo de auditoria. PostgreSQL registro `lock_timeout` en
`pg_advisory_xact_lock`; las otras 34 pruebas del ejecutable aprobaron.
Coverage se omitio por la dependencia fallida.

Los esquemas de prueba comparten una base de datos y el candado global de
mutaciones auditadas. Dos pruebas independientes pueden retenerlo el tiempo
suficiente para agotar el limite de un segundo del adaptador de despacho.
El reparto de CI ahora pasa `--test-threads=1` a los ejecutables de
infraestructura; conserva el paralelismo entre runners con bases separadas,
los hilos propios de las pruebas concurrentes y los dos hilos del harness
en los otros crates. No se cambia el protocolo de bloqueo de la aplicacion.

La regresion del comando fallo antes del cambio y los 12 tests de helpers
de CI aprobaron despues. La prueba Rust que habia fallado aprobo 1/1 en
5.75 s con PostgreSQL y Redis locales desechables, un hilo de harness y
directorio temporal sobre disco btrfs. Esta ejecucion focal no reproduce
la carga completa del VPS. La suite completa y la union de cobertura deben
aprobar en la nueva revision antes de integrar.

## Python de los workers aislados: 26 de septiembre de 2026

En la primera campaña distribuida,
[Test (1/2)](https://github.com/eddndev/TT2026-B136/actions/runs/36258835813/job/108450635744)
falló tras unos 38 minutos en
`document_formats::isolated::tests::continuously_readable_input_does_not_pay_a_delay_per_pipe_chunk`.
El worker de prueba encontró Python 3.9.25 después de limpiar su entorno;
esa versión no expone `fcntl.F_SETPIPE_SZ`. Python 3.12 ya estaba instalado,
pero solo se seleccionaba mediante el PATH de la sesión del runner.

La prueba original reprodujo el fallo aisladamente en VPS1 en 0.03 s. Se
seleccionó Python 3.12.14 mediante `/usr/local/bin/python3`, conservando el
intérprete de la distribución en `/usr/bin/python3`. Después, el mismo binario
instrumentado original aprobó las cinco pruebas de aislamiento en **0.31 s**,
con dos hilos, directorio temporal en disco y el usuario acotado del runner.
No se modificó el código Rust ni se amplió el límite de la prueba. VPS2
también aprobó la comprobación con entorno vacío usando Python 3.12.3.

El workflow comprueba ahora Python 3.12 o posterior y la constante de Linux
antes de compilar. La preparación de nuevos hosts está documentada en
[las operaciones del runner](ci-runner-operations.md). `actionlint` y
`git diff --check` aprobaron. Estos resultados focales no sustituyen la
suite completa ni el umbral de cobertura, todavía pendientes.

## Distribucion del CI entre dos runners: 26 de septiembre de 2026

El repositorio es privado y cuenta con dos runners dedicados en linea. El
runner de VPS1 tiene un limite de 2 CPU y 4 GiB para su usuario y Docker
rootless; el de VPS2 tiene 4 CPU y 6 GiB. El workflow propuesto asigna grupos
de pruebas Rust disjuntos a `Test (1/2)` y `Test (2/2)`, cada uno con PostgreSQL
y Redis desechables. El job Coverage solo inicia cuando ambos han aprobado y
une la cobertura por linea antes de aplicar los umbrales actuales. La campaña
extendida de despacho sigue siendo manual. Todavia no hay una medicion de
duracion ni una aprobacion de CI para esta distribucion.
La asignacion usa una proporcion de capacidad 2:3: VPS1 recibe los ejecutables
rapidos y 27 grupos de infraestructura; VPS2 recibe los otros 39. Esa
proporcion es inicial y debera contrastarse con la duracion real de ambos jobs.

Antes de la primera ejecucion distribuida se detecto que VPS1 tenia las
herramientas cliente de PostgreSQL 17, mientras el servicio de pruebas usa
PostgreSQL 16. Se instalaron las herramientas 16 junto a las existentes y el
workflow ahora exige esa version antes de iniciar la suite. La reproduccion
local con cliente 18 y servidor 16 fallo en `pg_restore` por
`transaction_timeout`; la misma prueba aprobo 1/1 en 5.97 s con cliente 16.
La primera suite local se detuvo al encontrar esa incompatibilidad. La nueva
ejecucion con herramientas compatibles aprobo la restauracion y la prueba
concurrente corregida, pero fue interrumpida por el apagado del equipo; no es
una aprobacion completa. Se publica con la campana completa pendiente en CI.
Los 12 tests de los helpers de CI aprobaron. Una prueba minima con dos crates
y dos ejecutables verifico que `cargo llvm-cov --no-report` acumula sus perfiles
y que un unico reporte LCOV final incluye las seis lineas cubiertas de ambos.
La generacion del reporte queda fuera del bucle de ejecutables para evitar
repetir la agregacion de perfiles durante toda la campana.

## Sincronizacion de la prueba concurrente: 26 de septiembre de 2026

La ejecucion [36217187390](https://github.com/eddndev/TT2026-B136/actions/runs/36217187390)
fallo despues de 79 minutos en
`deadline_backend_concurrency::simultaneous_connections_commit_one_successor_and_one_audit_event`.
El primer hilo esperaba al segundo con un limite de cinco segundos, pero la
prueba abria y validaba la segunda conexion despues de iniciar el primero.
Ambos participantes agotaron ese limite antes de probar la escritura
concurrente. Las otras 38 pruebas del ejecutable aprobaron; Coverage se omitio
al depender de Test. Los otros cinco checks de Rust y ambos de Web aprobaron
para la misma cabeza. El fallo no demuestra un defecto en la escritura de
revisiones ni una mejora de tiempo de CI.

La prueba prepara ambas conexiones antes de iniciar los hilos y permite hasta
30 segundos para que el sistema programe a ambos participantes. Ese limite no
agrega espera cuando llegan normalmente. La prueba focal aprobo 1/1 en 9.90 s
con PostgreSQL 16 desechable. La campana instrumentada completa sigue
pendiente.

## Bloqueo de migraciones por esquema: 26 de septiembre de 2026

La apertura PostgreSQL tomaba un candado de migracion comun a todos los
esquemas de una base de datos. Las pruebas de integracion crean esquemas
independientes, por lo que dos aperturas se serializaban aunque sus tablas no
se compartieran. La prueba nueva retuvo el candado anterior y el candado de un
primer esquema: con el codigo previo, abrir un segundo esquema fallo por
`lock_timeout` en 3.11 s. Con la clave basada en el OID del esquema, esa misma
prueba aprobo en 0.43 s. Las dos pruebas del ejecutable
`postgres_startup` aprobaron en 2.26 s con PostgreSQL desechable. El build
ordinario del workspace tambien aprobo. El alcance y la limitacion de
despliegues que mezclen binarios antiguos y nuevos estan en
[ADR 0048](adr/0048-schema-scoped-migration-lock.md).

La suite completa local con PostgreSQL y Redis desechables aprobo **3049
pruebas**, con **2 ignoradas**, en **212 ejecutables**. Despues del ultimo
ajuste de claridad en la prueba de arranque, su ejecutable aprobo de nuevo
las 2 pruebas. `cargo fmt --all`, `cargo build --workspace --locked` y
`cargo clippy --workspace --all-targets --locked -- -D warnings` aprobaron.
Los tiempos de CI se mediran antes de atribuir una mejora global.

## Diagnostico de la PR42: 25 de septiembre de 2026

La ejecucion [35572138875](https://github.com/eddndev/TT2026-B136/actions/runs/35572138875)
termino cancelada exactamente al limite de 90 minutos del job Test. Format,
Lint, MSRV, Dependency policy, Release binary size y los dos jobs Web aprobaron;
Coverage se omitio porque dependia de Test. No se observo una asercion fallida.
La compilacion instrumentada de la primera campana tomo 13 min 47 s. Los
primeros 121 ejecutables completos sumaron 75.39 min de pruebas, y el job se
cancelo al empezar otro. Los grupos mas costosos fueron `deadline_suite_1`
(989.84 s), `deadline_suite_2` (637.44 s), `deadline_suite_3` (529.28 s),
`alert_suite_1` (527.04 s) y los dos grupos `case_suite` (350.38 y 340.10 s).
Esta es evidencia historica de una campana incompleta, no un resultado global.

La siguiente campana, con configuracion desechable de PostgreSQL y tres hilos,
fallo en `alert_suite_1`: 24 pruebas aprobaron y
`activation_audit_failure_rolls_back_inbox_outbox_and_plan_together` recibio
un error `Busy` al adquirir el candado de auditoria antes de inyectar su fallo.
El job termino en 15 min 35 s; Coverage se omitio. El grupo de alertas tardo
538.23 s, frente a 527.04 s con dos hilos en la campana anterior. El candado
de auditoria es comun a todos los esquemas de esa base de datos y el adaptador
de alertas limita su espera a un segundo. Por eso se retiran los tres hilos;
no hay evidencia de que hayan acelerado esa prueba. Web aprobo ambos jobs en
[la ejecucion 36205760409](https://github.com/eddndev/TT2026-B136/actions/runs/36205760409)
para la cabeza `9fd5110`. El resultado completo de Rust sigue pendiente.

El build ordinario reutilizo su cache y registro 0 MiB como pico medido, por lo
que la seleccion conservadora mantuvo un compilador. El grupo de memoria del
runner registro cero eventos OOM, aunque supero el umbral MemoryHigh. Los logs
de PostgreSQL muestran checkpoints frecuentes de miles de archivos. Para la
siguiente campana se configuran solo sus bases desechables sin durabilidad de
caida, se conservan dos hilos de pruebas dentro del mismo tope 3 CPU/5 GiB y se
amplia temporalmente el tiempo maximo a 180 minutos para obtener el resultado
completo. La razon y el riesgo estan en [ADR 0047](adr/0047-disposable-postgres-ci.md).
Se medira el tiempo completo antes de afirmar una aceleracion o reducir el
limite de tiempo. En una base PostgreSQL local desechable, la comprobacion
previa de configuracion fallo con `on|on|on` y aprobo con `off|off|off` despues
de aplicar el ajuste. Con esa configuracion, 17 pruebas focales de despacho
aprobaron en **100.22 s de pruebas** (105.49 s incluyendo preparacion). Esta
medicion local no reemplaza la regresion instrumentada del VPS.
Tras volver a dos hilos, la suite local de alertas con PostgreSQL y Redis
desechables y la misma configuracion no durable aprobo **25 pruebas en 79.78 s**,
incluida la que habia fallado en CI. La diferencia de equipo e instrumentacion
impide usar ese tiempo para estimar la duracion del VPS.

La campana [36207460783](https://github.com/eddndev/TT2026-B136/actions/runs/36207460783),
con dos hilos y PostgreSQL desechable, fue cancelada exactamente al limite de
180 minutos. El ultimo grupo completo, `typed_suite_1`, aprobo 31 pruebas a
las 04:09:21 UTC; el job entro en `typed_suite_2` y se detuvo 25 segundos
despues. No aparece una asercion fallida. Los otros cinco checks de Rust y los
dos de Web aprobaron para la misma cabeza `4649d43`; Coverage se omitio al
depender de Test. El limite se amplia a 210 minutos para completar la campana
y producir el reporte de cobertura. El candado de migracion pasa a ser por
esquema segun [ADR 0048](adr/0048-schema-scoped-migration-lock.md), con la
regresion local completa descrita arriba. El tiempo total nuevo sigue sin
medirse: el aumento de limite no cuenta como una aceleracion.

## Reduccion del costo de CI: 21 de septiembre de 2026

Se sustituyen 537 ejecutables de integración por 203, conservando los archivos
originales y sus casos. El inventario automatizado comprueba que cada archivo
está registrado exactamente una vez. La revisión del cambio conserva los
nombres de funciones y atributos de prueba e ignorado existentes.

La caché instrumentada se conserva por configuración compatible; cada campaña
elimina perfiles y reportes previos. La compilación posterior usa uno o dos
trabajadores según la memoria medida durante el build ordinario, dentro del
mismo límite de 5 GiB. Los fixtures de despacho reutilizan un repositorio durante
la siembra sin compartir esquemas entre pruebas ni retirar validación de
producción. Véase [ADR 0046](adr/0046-cached-ci-test-suites.md).

La campaña anterior fue cancelada para aplicar esta corrección. No se cuenta
como aprobada ni se promete un tiempo final hasta medir la nueva ejecución.
El navegador permanece sin cambios en esta corrección.

Comprobaciones ejecutadas en esta corrección:

- Resolución y compilación de comprobación de todos los targets Rust mediante
  `cargo fix --workspace --tests --locked --allow-dirty`, seguida de la misma
  comprobación para la agrupación final de infraestructura. Solo se aplicaron
  sugerencias de imports sobrantes introducidos al compartir fixtures.
- 17 escenarios de despacho agrupados, con PostgreSQL/Redis desechables, un
  compilador y un hilo: **17 aprobados en 108.23 s de pruebas**. Incluyen
  rollback, concurrencia, permisos, corrupción del catálogo y reapertura.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: aprobado
  después de resolver atributos duplicados y módulos anidados compartidos.
- Siete pruebas Python de limpieza/invalidez de caché y límites de memoria.
- Dos ejecuciones de un proyecto mínimo: el binario conserva su fecha de
  compilación y la cobertura baja de 11 a 8 líneas cuando la segunda campaña
  deja de ejecutar una función. La prueba local usó cargo-llvm-cov 0.8.7;
  CI fija 0.9.1 y valida allí el workspace completo.
- Inventario de 203 ejecutables, `cargo fmt --all -- --check`, `git diff
  --check` y actionlint. La regresión global y el umbral de cobertura quedan
  para la nueva campaña de CI; no se presentan como aprobados todavía.

## Preparacion de la prueba de despacho y CI: 21 de septiembre de 2026

La prueba `deadline_dispatch_measurement` reutiliza un repositorio abierto al
sembrar los plazos. Se mantienen una muestra obligatoria de 21 registros y una
campaña manual de 240, ambas con límites 1, 20 y 100, igualdad de identificadores,
cantidad de páginas y total de trabajos. La campaña extensa queda ignorada en
la ejecución normal y se puede activar explícitamente; no se retiraron las
pruebas de corrupción, permisos, reapertura o restauración.

Comprobación focal local, PostgreSQL y Redis desechables, `CARGO_BUILD_JOBS=1`,
`RUST_TEST_THREADS=1` y temporales privados en disco Btrfs:

- RED: el target no compiló al solicitar el helper de repositorio reutilizable
  antes de incorporarlo (`E0425`).
- GREEN: `scripts/test-backends.sh cargo test -p infrastructure --test
  deadline_dispatch_measurement -- --include-ignored --nocapture` aprobó los
  dos casos en **25.32 s de ejecución de pruebas**, sin incluir compilación.
  La campaña de 240 creó los 720 trabajos esperados y la de 21 creó 63.
- Los logs locales quedaron en `output/ci-runner-verification/`. Este resultado
  es una ejecución focal; no acredita todavía el tiempo de la suite completa
  en el VPS ni permite comparar directamente velocidades de equipos distintos.

El flujo de CI de [ADR 0045](adr/0045-single-pass-ci-and-dispatch-measurements.md)
ejecuta la batería instrumentada una vez y entrega su reporte al control de
cobertura. La campaña manual en vps2, con un hilo y el límite compartido de tres CPU y
5 GiB, aprobó los 240 registros y 720 trabajos en **149.01 s de pruebas**.
El job completo, incluida la primera compilación, tomó **7 min 34 s**.
El control posterior falló antes de las pruebas porque la limpieza de caché
había eliminado `rustup`. Se retiró esa acción del runner persistente y se
conservan los artefactos de Cargo fuera del checkout. El cierre de la suite
completa con esta corrección aún está pendiente. La primera compilación
instrumentada terminó y la ejecución avanzó a infraestructura, pero se canceló
la campaña que forzaba un solo hilo por su duración. Se conserva un solo
compilador y una sola suite, con dos hilos de pruebas dentro de los mismos
límites de CPU y memoria. No se contabiliza la campaña cancelada como aprobada.
El recorrido de navegador real de la corrección del selector de responsables
aprobó sus 43 escenarios en una campaña separada; no sustituye el cierre Rust.

## Costo de contrasenas: calibracion del 19 de septiembre de 2026

El adaptador genera hashes Argon2id con tres pasadas, conservando 262144 KiB
(256 MiB), una via, sal aleatoria de 16 bytes y salida de 32 bytes. La razon y
los limites estan en [ADR 0044](adr/0044-reference-password-hashing-cost.md).
No se migran PHCs existentes ni se altera la concurrencia de la API.

La medicion se ejecuto en AMD Ryzen 7 7730U, 16 CPU logicas, Linux
7.1.13-100.fc43.x86_64. El binario dev optimiza `argon2` y `blake2` a nivel 3.
Solo habia un coordinador de verificacion; no se afirma aislamiento de todo
el sistema operativo. La carga media previa era 1.9004/2.6924/2.7114.

- Base de dos pasadas: **400.3851816 ms**, cinco hashes, veredicto `below`.
  El binario construido por la aceptacion de miembros tiene SHA-256
  `e62a9cccc90deea528c35ea092e320434b75a626ea020b8b8d10dea765027427`.
- Tres pasadas: **659.7596048 ms**, cinco hashes, veredicto `inside` en la
  banda inclusiva de 500-1000 ms. Se comprobaron el JSON y sus limites;
  el exit code no acredita por si solo la banda. SHA-256 del binario:
  `cf688da4940d70825b85c9d9d7ce26e3efc03888a0b206d7819e92268bdd5486`.
  Compilacion y calibracion juntas tomaron 79.514 s, con 2553 fuentes estables.
- TDD: la expectativa de tres pasadas fallo con el adaptador anterior;
  dos casos aprobaron y uno fallo en 34.548 s. Despues aprobaron **7 casos**
  en 21.602 s: tres de integracion y cuatro internos, incluidos PHC anterior
  con contrasena correcta/incorrecta, sales distintas y formatos malformados.
- CLI: **6 pruebas aprobadas**, 20.916 s. `scripts/demo.sh` aprobo en
  **8.240 s**, incluida una medicion secundaria de **563.2 ms** sobre cinco
  hashes. Esta segunda observacion no reemplaza la principal. Cada comando
  conservo sus 2553 fuentes estables y uso temporales privados en disco.

Los registros estan en `output/password-calibration-verification/`. La banda
corresponde a un hash nuevo en ese binario y equipo, no al login completo,
enrolamiento, recuperacion ni comportamiento bajo carga. Los hashes historicos
siguen verificandose con sus parametros codificados. CI, integracion y revision
del PDF de estas fuentes siguen pendientes; las cifras anteriores conservan
su propio contexto y no se recalculan con este cambio.

## Directorio, acceso y asignaciones: verificacion del 19 de septiembre de 2026

El incremento local implementa directorio Owner, cambios de rol/actividad con
revision esperada y selectores de asignacion. Una generacion persistida revoca
sesiones y desafios anteriores incluso despues de reactivar la cuenta. Las
asignaciones se conservan al desactivar; se protege el ultimo Owner activo.
El [contrato de miembros](members-api.md) y
[ADR 0043](adr/0043-member-access-and-authentication-generation.md) fijan limites.
Invitaciones, recuperacion de credenciales y autenticacion por certificado
permanecen pendientes. Este corte no acredita integracion ni regresion global.

Las campanas locales corrieron en serie, con un coordinador, jobs Rust y
hilos de prueba en uno, y Playwright con un trabajador. Los temporales privados
estuvieron en disco, bajo `output/tmp`. Se conservaron resultados y hashes de
fuentes por comando en `output/member-lifecycle-verification/`.

- Node: dieciseis fallos iniciales por metodos ausentes; despues **16 aprobadas**
  en 0.655 s, con 2521 fuentes estables. Cubren revision decimal sin perdida,
  contrato estricto y directorios filtrados de cuentas/asignaciones.
- Aplicacion/HTTP: **41 aprobadas** en 42.282 s, con 2553 fuentes estables:
  veinte de identidad, cuatro de consultas, diez de servicio y siete HTTP.
  La campana inicial habia aprobado dieciseis de identidad y fallado las cuatro
  nuevas, cuatro consultas, diez servicios y siete rutas. La repeticion cubre
  generacion, reautenticacion, limites, conflictos y respuestas sin secretos.
- Navegador con HTTP controlado: **11 aprobadas** en 28.848 s de comando y
  27.5 s de Playwright, con 2553 fuentes estables. Incluye escritorio/movil,
  filtros, asignacion con expediente cerrado, conflicto, ultimo Owner,
  cambio propio, respuesta incierta y cambios de contexto. El intento inicial
  paro ante la region de directorio ausente: uno fallo y diez no se ejecutaron.
- PostgreSQL/Redis: diez casos reprodujeron el comportamiento ausente.
  La siguiente campana no ejecuto tests: fallo compilacion por conversion
  `OffsetDateTime`/PostgreSQL en dos puntos, en 10.773 s. Se corrigieron las
  conversiones sin ampliar dependencias. La repeticion aprobo **12 casos** en
  41.896 s, con 2553 fuentes estables y PostgreSQL/Redis desechables: atomicidad,
  ultimo Owner concurrente, filtros, inventario/catalogo, guardas y restauracion.

El navegador con servicios reales aprobo **3 escenarios**, 418.426 s de comando
y 40.1 s de Playwright, con 2553 fuentes estables. En escritorio de 1440 pixeles
y movil de 390 se verificaron baja/reactivacion, sesion anterior rechazada tras
reactivar, login nuevo y asignaciones conservadas; el expediente movil estaba
cerrado. Otro recorrido confirmo cambio propio antes de cerrar sesion y
restriccion de Client. Las cuatro capturas reales resultaron legibles, sin
recortes ni desbordamiento, conservando Qadra. Se encuentran bajo
`output/member-lifecycle-verification/browser-live-first/`.

La aceptacion HTTP completa con restauracion aprobo en **453.174 s**, con
2553 fuentes estables. Reprodujo directorio paginado, asignaciones, cambios con
revision esperada, baja/reactivacion, desafios y sesiones anteriores rechazados,
nuevo MFA, cambio propio y rol actualizado. Tras dump/restore, las proyecciones
de cuentas y asignaciones coinciden, las sesiones anteriores siguen denegadas
y los accesos renovados usan MFA sin cambiar revisiones. El inventario conserva
19 raices documentales, 25 versiones y un incidente; la importacion conserva
cuatro documentos, 70 eventos historicos y ZIP de evidencia identico.
El registro es `members-api-live-first.json` y su log correspondiente.

Clippy, formato y MSRV aprobaron en CI de `41f914b`. La regresion global y
cobertura siguen pendientes; la compilacion e inspeccion de las fuentes
academicas actualizadas requieren su propia comprobacion. Estos resultados
no equivalen a integracion en main.

## Contenido original e incidentes: verificación del 19 de septiembre de 2026

El incremento añade descarga exacta de versiones pendientes o selladas y buzón
interno Owner. Las focales PostgreSQL y el navegador con servicios reales
aprobaron. La aceptación API completa con restauración también aprobó; la
inspección del PDF y CI permanecen pendientes. Este corte no acredita integración.
La [API](document-content-api.md) conserva la separación entre comprobar bytes
y verificar firma/sello, así como la denegación de Client.

La construcción comenzó por pruebas fallidas: diez casos Node por método
ausente, trece de aplicación y siete HTTP frente a puertos/rutas sin implementar,
y seis de PostgreSQL. Las focales siguientes son independientes:

- Aplicación y HTTP: **20 aprobadas**, 41.997 s; trece de aplicación y siete de
  transporte. Cubren AAD/digest, exactitud histórica, permisos, reautenticación
  del principal completo, fallo de auditoría o incidente y ausencia de bytes
  rechazados. También distinguen 409 de indisponibilidad y los límites de ruta.
  Las fuentes de aplicación y HTTP permanecieron estables; ocho archivos de
  infraestructura no compilados por este comando cambiaron en paralelo.
- Cliente Node: **19 aprobadas**, 0.563 s, con 2485 fuentes estables. Comprueban
  identidad/versión/cabeceras binarias, limpieza de contexto y parseo estricto
  de incidentes, tiempos y paginación.
- Navegador con HTTP controlado: **13 de 14 aprobadas**, 58.274 s; seis de
  descarga y siete del buzón. El escenario de descarga durante logout agotó
  su espera y sigue pendiente de corrección; este comando no se declara
  aprobado. Sólo cambió un archivo Rust durante el navegador; las fuentes
  JavaScript/Svelte ejecutadas permanecieron estables.

- PostgreSQL: **8 aprobadas**, 42.427 s, con 2485 fuentes estables y servicios
  desechables. Comprueban exactitud histórica, cota previa a materializar vault,
  revocación, rollback de auditoría, repetición exacta, permisos Owner, catálogo,
  inventario y restauración completa de la captura.
- Capacidad HTTP: dos casos nuevos reprodujeron la entrega sin límite de cuerpos
  vivos; ambos fallaron al observar 200 donde se esperaba 503. Después aprobaron
  junto con las cuatro regresiones de contenido en 24.454 s, con 2486 fuentes
  estables. El permiso acompaña al buffer ceroizable hasta soltar el último
  chunk o copia, incluso después de consumir el cuerpo HTTP.

La primera campaña API integrada se detuvo en 92.508 s antes de recibir la
primera petición funcional: el guion agotó diez segundos de arranque sin que el
proceso publicara dirección. No acredita aceptación ni un fallo documental.
La espera acotada se amplió a sesenta segundos y se inició una nueva campaña.

El caso pendiente de logout controlado aprobó por separado en **9.783 s**,
con 2490 fuentes estables. La espera ahora observa la resolución/cancelación
del fetch iniciada antes del cambio, sin exigir consumir un cuerpo que el cliente
ya descartó. Conserva las comprobaciones de cero descargas y cero éxito tardío.
Son **14 escenarios distintos aprobados** por las dos ejecuciones, sin atribuir
éxito a la campaña anterior completa.

La segunda campaña API se detuvo en 53.584 s al agotar la misma espera corta
en el segundo servidor de concurrencia; también se amplió a sesenta segundos.
La tercera duró 333.427 s y alcanzó el nuevo fixture tras aprobar los recorridos
anteriores. Falló su helper SQL: pasar una URI como `PGDATABASE` sin `-d` no
la expandía y elegía la conexión predeterminada. La reproducción sobre la base
desechable retenida confirmó el fallo y la corrección con `-d`; ambos helpers,
API y navegador, ahora especifican esa conexión. Estas ejecuciones no acreditan
el cierre integrado. Las fuentes HTTP/Rust/Python ejecutadas en la tercera se
conservaron; cambiaron únicamente fuentes web ajenas a ese comando.

### Navegador con servicios reales

La campaña `content-browser-live-first` aprobó **3 escenarios**: 285.120 s del
comando y 24.6 s de Playwright, con **2490 fuentes estables**. Comprueba descarga
binaria exacta de V1 pendiente de sello después de registrar V2, cabeceras y
digest ligados a esa versión, aviso Owner, buzón persistente, apertura exacta
y retorno. Los recorridos Owner se ejecutaron a **1440 y 390 píxeles**; el
expediente del recorrido móvil estaba cerrado administrativamente. El tercero
comprueba lectura del Paralegal asignado, denegación de Client y del buzón a
otros roles, y limpieza de la captura después de revocar la asignación.

El fixture alteró dos vaults AES aislados de la base desechable para provocar
rechazos reales y sus incidentes persistidos. Restauró los bytes originales en
`finally` antes de abrir el navegador. Esta preparación del ensayo no es una
función de reparación del producto ni demuestra un ataque. Las denegaciones
por permisos no añadieron incidentes.

Se inspeccionaron **cuatro capturas** de contenido exacto y buzón en escritorio
y móvil, además de dos ampliaciones del detalle: Qadra permanece legible, sin
recortes ni desbordamiento. Esta inspección de interfaz no acredita usabilidad
con personas ni sustituye la compilación e inspección del PDF. La aceptación
API completa sigue pendiente después de los tres fallos descritos; el navegador
aprobado no cierra esa campaña ni los controles de CI.

La cuarta campaña API completa **aprobó en 372.048 s**, con 2490 fuentes
estables, tras especificar la conexión SQL del fixture. Comprueba descarga V1/V2
pendientes de sello, bytes/cabeceras exactos, cuatro roles, cierre, revocación,
rechazo AES sin contenido ni autorización exitosa, y buzón exclusivo de Owner.
La corrupción desechable se revierte byte a byte antes del respaldo. Después de
restaurar, los archivos y respuestas históricas del incidente coinciden; el
inventario conserva 19 raíces documentales, 25 versiones y un incidente inmutable.
También preserva el prefijo importado de cuatro documentos y 63 eventos con ZIP
idéntico. No se atribuye éxito a los tres intentos anteriores.

El primer CI de esta entrega detectó una conversión `as_str` redundante en el
codec del incidente; se elimina sin cambiar sus bytes ni el contrato. El cierre
remoto y su comprobación de Clippy conservan resultados separados.

Se usa un solo runner local, Cargo y Rust en un hilo, Chromium con un trabajador
y temporales privados en disco. No se repitió una regresión global local.

## Asociaciones de recursos con actividades: verificación del 19 de septiembre de 2026

El incremento implementa vínculos organizativos a audiencias y plazos
existentes, con recurso/acto y actividad históricos verificados. Detalle y lista
separan esas capturas del estado actual observado; desvincular no modifica las
actividades ni duplica sus alertas. PostgreSQL, HTTP y Qadra están conectados
localmente. El [contrato](resource-activities-api.md) delimita el incremento;
no incluye creación contextual de audiencias, activación automática ni corpus
jurídico nuevo. La aceptación API/restauración integrada y los tres recorridos con navegador
real aprobaron. CI e integración permanecen pendientes en este corte.

El navegador remoto de `f81f0cb` aprobó 36 escenarios y falló el de
asociaciones de escritorio: esperaba que el aviso de 48 horas siempre tuviera
origen R2. El trabajador puede activarlo en R1 antes del cambio de sede, sin
cambiar su fecha ni abrir otra ocurrencia. La prueba ahora exige origen R1 o R2
con su digest exacto, mismo sujeto y ámbito, ventana de 48 horas y estado activo;
conserva la comparación íntegra del aviso antes y después de vincular/desvincular.
Dos casos Node reprodujeron el rechazo incorrecto; los tres casos de origen
aprobaron en 0.330 s, incluido el rechazo de digest, ámbito, revisión o ventana
ajenos. La repetición con servidor real y el cierre remoto siguen pendientes.

Resultados focales ejecutados en serie, sin sumarlos como una regresión global:

| Frontera | Casos distintos aprobados | Alcance y límite |
| --- | ---: | --- |
| Dominio | 4 | Identidades, referencias tipificadas, acto opcional y selección canónica. |
| Aplicación | 18 | Seis de flujo, ocho de límites y cuatro de tiempo; permisos, reautenticación, fuentes, recibos e historia por puertos. |
| HTTP Rust | 10 | Dos de cuerpos y ocho de workflow, incluido el acto no nulo; alcance de URL, límites, errores y proyecciones. |
| Cliente Node | 12 | Comandos, selección, respuestas, recibos y conservación separada de historia/vigencia. |
| Navegador con HTTP controlado | 12 | Diez casos anteriores y dos adicionales de recuperación; dos repeticiones visuales a 1440/390 píxeles no añaden casos distintos. |
| PostgreSQL | 12 | Once de backend, permisos, atomicidad, inventario, catálogo y restauración; una adicional del reloj bajo bloqueo. |

La primera campaña HTTP observó dos fallos de cuerpos y seis de workflow frente
al stub. Los nueve casos aprobaron después de implementar el transporte en
29.639 s, con 2406 fuentes estables. La prueba adicional del acto no nulo aprobó
después en 27.228 s, con 2426 fuentes estables: conserva recurso R2, acto dentro
de recurso R3 y soporte exacto, y rechaza una captura discordante. Esas focales
usan puertos controlados y no demuestran comportamiento con PostgreSQL real.

La revisión identificó que capturar `checked_at` antes de esperar el bloqueo
podía rechazar una cabeza legítima confirmada durante la espera. El RED de
aplicación produjo dos fallos positivos y dos rechazos aprobados en 4.565 s.
La corrección acepta un corte UTC entre inicio y retorno, exige uno común por
página y conserva su vínculo con la proyección operativa. Los 18 casos de
aplicación aprobaron juntos en 6.737 s, con 2426 fuentes estables.

La campaña PostgreSQL de 88.999 s aprobó once casos y reprodujo el fallo del
reloj; ese comando tuvo un fallo y no se declara aprobado. La prueba focal del
reloj aprobó después en 15.186 s, con 2426 fuentes estables: verifica que la
consulta tome su instante mientras posee el bloqueo, tanto para detalle como
para lista. Los doce casos distintos quedan respaldados por esas ejecuciones
separadas. La restauración focal usa `pg_dump`/`pg_restore`, preserva capturas y
autores históricos y permite crear otra asociación. El recorrido HTTP completo
de restauración se ejecutó después, con el alcance descrito a continuación.

Dos casos posteriores de navegador cubren el reenvío explícito del mismo
comando después de un resultado incierto no confirmado y la recuperación del
selector tras un conflicto al confirmar. El primero aprobó dentro de una
campaña de 15.408 s que aún falló en el selector; ésta no se presenta como
completamente aprobada. La repetición focal del selector aprobó en 12.847 s,
con 2426 fuentes estables. Los dos casos se añaden una sola vez a los diez
anteriores. Dos repeticiones visuales duraron 12.326 s. Se inspeccionaron cuatro
capturas de las secciones nuevas a 1440/390 píxeles, sin desbordamiento; esa
inspección no constituye un ensayo de usabilidad con personas.

### Aceptación API con restauración

La campaña completa `scripts/api-demo.sh` aprobó en 434.402 s, con 2426 fuentes
sin cambios durante la ejecución. El guion de asociaciones verificó fuentes
exactas y cabezas actuales separadas, acto opcional, desvinculación, repetición
exacta, roles, revocación, cierre, archivo y aislamiento entre expedientes.

Después de restaurar se compararon 26 respuestas HTTP. Sus instantes de lectura
se validaron por separado, incluido el corte común de página y su vínculo con
la proyección operativa; no se exigió que un instante de consulta nueva igualara
al anterior al respaldo. Las capturas históricas y los demás datos comparados se
conservaron. El inventario confirmó tres raíces y cuatro revisiones de asociación
idénticas después de restaurar, junto con las demás tablas del recorrido.

Este resultado acredita la API integrada y su restauración. No se ha ejecutado
otra suite global local.

### Navegador real y correcciones de cierre

La primera campaña real duró 345.356 s: aprobó los recorridos Owner a 1440 y
Litigator a 390 píxeles, y falló en el guion de Client porque utilizaba el
buscador exclusivo del personal. Ese comando completo no se declara aprobado.
La corrección usa la lista básica autorizada de Client y limita el control de
solicitudes a las rutas API, sin confundir módulos JavaScript con acceso a datos.
La repetición del único recorrido pendiente aprobó en 274.304 s, incluidos
preparativos; Playwright tardó 18.0 s. Ambas ejecuciones conservaron 2426 fuentes
sin cambios. Son tres escenarios distintos aprobados: vínculo/desvínculo e
historia en escritorio/móvil, y permisos/revocación con Paralegal y Client.
Las comprobaciones conservan las actividades y sus alertas sin duplicarlas.

Un caso controlado ampliado reprodujo fecha sin formato y autor técnico vacío
en la actividad de plazo: falló en 25.044 s y aprobó en 8.445 s al reutilizar los
formatos existentes. Es una ampliación del caso ya contabilizado, no un caso
adicional. CI detectó carga duplicada de fixtures Rust; la corrección reutiliza
el módulo compartido y aprobó Clippy focal de los tres objetivos de aplicación
en 13.996 s. El CI inicial fue cancelado después de ese fallo, no aprobado.
Una segunda carga duplicada del mismo catálogo en fixtures HTTP fue corregida
al reutilizar el módulo ya importado. Clippy del workspace completo, incluidos
todos los objetivos, aprobó en 315.721 s con 2426 fuentes estables. El cierre
global remoto permanece pendiente; el PDF de 331 páginas tiene inspección
registrada en `docs/academic-report-verification.md`.

## Integración de recursos procesales en main

La [PR 37](https://github.com/eddndev/TT2026-B136/pull/37) se integró mediante
squash el 19 de septiembre de 2026 a las 19:16:57 UTC como `eeba869`. Su cabeza
`40ed0c7` aprobó los controles remotos de formato, Clippy, MSRV, dependencias,
binario, web, navegador real y documentos. El
[CI de cierre](https://github.com/eddndev/TT2026-B136/actions/runs/35459822870)
registró **2936 pruebas Rust aprobadas, cero fallidas y una TSA externa ignorada**,
con PostgreSQL/Redis aislados. La campaña instrumentada reprodujo esos conteos;
no se suman como casos adicionales.

La cobertura medida fue dominio 5572/5694 líneas (97 %), aplicación
16078/16860 (95 %) e infraestructura 28261/30660 (92 %), todas por encima del
umbral del 90 %. El binario obtuvo 1822/2302 (79 %), sin umbral propio.
Estos valores pertenecen a esa revisión remota; no constituyen una ejecución
global del incremento de asociaciones ni del contenido documental posterior.

## Recursos procesales: checkpoint local del 19 de septiembre de 2026

El dominio, servicio, adaptador PostgreSQL, rutas HTTP y formularios Qadra
implementan registro y corrección de recursos, actos con revisiones propias,
archivo/reactivación e historia. Cada recurso conserva resolución, personas y
soportes exactos. El [contrato](procedural-resources-api.md) distingue esta
captura del enlace posterior con audiencias, términos y alertas, aún pendiente.
La aceptación HTTP con restauración completa y los tres recorridos de navegador
con servidor real aprobaron después de las pruebas focales. El cierre de CI y
la integración permanecen pendientes; el checkpoint está publicado en la PR 37.

Se ejecutó una sola verificación local a la vez, con Cargo y pruebas Rust en un
hilo, navegador con un trabajador y temporales privados en disco. Evidencias
por frontera, sin sumar campañas repetidas como pruebas distintas:

- Dominio: 14 pruebas aprobadas sobre identidades, valores, canones y compromisos.
- Aplicación: 13 aprobadas por puertos, con permisos, fuentes exactas,
  reautenticación, recibos, actos y estados organizativos. El primer intento
  detectó una variable del fixture que ocultaba una función; se corrigió antes
  de ejecutar los casos.
- HTTP Rust: nueve aprobadas. La primera campaña detectó un nombre documental
  inválido en el fixture; se conservó como rechazo y se corrigió el positivo.
- Cliente: cuatro pruebas de valores y cinco de API/recibos aprobadas, después
  de sus fallos iniciales por módulos aún ausentes.
  Nueve pruebas adicionales verificaron el vínculo entre administración y etapa
  capturadas: tres positivos pasaron y seis rechazos fallaron antes del arreglo;
  las nueve aprobaron en 0.371 s con 2329 fuentes estables después del guard.
- PostgreSQL: tres pruebas de almacenamiento, tres de atomicidad/revocación y
  una de inventario aprobaron con una base desechable. La restauración falló
  por la función SQL de alertas anterior al arreglo ya publicado; se incorporó
  esa corrección. La campaña final aprobó los doce casos PostgreSQL en
  48.921 s, con 2328 fuentes estables: los siete anteriores, restauración,
  dos de catálogo y dos de raíces de actos. Estos últimos fallaron primero
  al aceptar una raíz sustituida después de abrir el almacén; el JOIN que
  verifica identidad, primera revisión y acción del acto corrigió ambos.
  Restaurar preservó recibos y autoría y permitió anexar otra revisión.
- Qadra con HTTP controlado: cinco recorridos aprobaron en 16.1 s de Playwright
  (17.338 s de comando), incluidos resolución R1 con cabeza R2, acto oral,
  archivo/reactivación, historia exacta y respuesta incierta sin reenvío.
  La campaña siguiente aprobó siete casos adicionales de roles, revocación,
  cierre y conflicto explícito, junto con dos repeticiones para capturas
  visuales: nueve recorridos en 19.2 s (20.398 s de comando).
- Se inspeccionaron las capturas de historia a 1440 y 390 píxeles: tarjetas,
  referencias y controles conservan Qadra sin desbordamiento horizontal. Una
  repetición de dos recorridos (9.6 s de navegador, 10.785 s de comando) tomó
  las imágenes desde el inicio de página para evitar un artefacto de elementos
  fijos fuera del viewport en la captura completa. No añade casos funcionales.
- `npm run build` aprobó en 3.655 s. Esta compilación no acredita la aceptación
  contra el backend real.
- Clippy focal detectó un `format!` innecesario en un fixture HTTP, corregido
  después. La ejecución global posterior se interrumpió a los 242.790 s para
  publicar el checkpoint y reservar el cierre global para CI; no se contabiliza
  como aprobada ni sustituye el control requerido antes de integrar.

### Aceptación integrada de recursos

`scripts/api-demo.sh` aprobó en 383.388 s, con 2329 fuentes estables y PostgreSQL,
Redis, qpdf y TSA local desechables. Ejercitó revocación y apelación, actos orales
y escritos, soportes históricos PDF/DOCX, corrección de actos, archivo/reactivación,
cuatro roles, aislamiento, cierre/reapertura, conflictos y repetición exacta.
La captura previa al respaldo conservó veinte respuestas HTTP de recursos;
después de restaurar se compararon completas, incluidos autores y recibos.
Los estados de recursos, alertas y auditoría entraron en la misma comparación
de tablas con el servidor detenido. No se envió correo externo.

El primer navegador real aprobó escritorio y móvil y se detuvo en el caso de
permisos: el guion confundió la descarga de un módulo JavaScript con una consulta
a la API. Se restringió ese filtro a rutas API. La repetición aprobó los tres
casos en 25.3 s de Playwright y 264.627 s totales, con 2329 fuentes estables.
Owner a 1440 píxeles y Litigator a 390 registraron y corrigieron un acto oral,
conservando R1 y R2 frente a R3. Paralegal consultó sin mutaciones, Client no
obtuvo navegación ni acceso API y la revocación de membresía retiró el detalle.
Se inspeccionaron cuatro capturas actuales e históricas: sin desbordamiento ni
solapamiento. Se tomaron desde el inicio de página para evitar artefactos de
elementos fijos fuera del viewport en capturas completas.

La CI de `616b908` aprobó formato, MSRV, política de dependencias, tamaño,
verificación web y compilación del reporte. Clippy señaló `chunks_exact(2)` en
un fixture canónico; se sustituyó por `as_chunks::<2>()` como en los vectores
versionados existentes. Clippy focal del fixture corregido aprobó en 3.921 s,
con 2329 fuentes estables. Test y Coverage seguían en curso al preparar este corte.
La cabeza corregida requiere sus propios controles de cierre.

Las ejecuciones conservaron fuentes y logs en `output/resources-verification/`.
Los cambios concurrentes registrados durante algunos comandos pertenecen a
fronteras distintas de las ejercitadas: fixtures PostgreSQL o SQL de alertas
durante navegador, y JavaScript durante HTTP/PostgreSQL. No se presenta el
conjunto como una regresión global de una única cabeza publicada.

## Alertas personales: checkpoint funcional del 19 de septiembre de 2026

Están implementados temporización configurable, puertos de preferencias y
bandeja personal, servicio autorizado, cinco rutas HTTP y Qadra. La bandeja
conserva el origen exacto, lectura independiente de atención, estados del correo,
filtros, continuación, conflictos y respuestas inciertas. El contrato está en
[alerts-api.md](alerts-api.md). La persistencia, generación durable y composición
del servidor están implementadas; el navegador real aprobó sus recorridos
focales. La aceptación API/restauración también aprobó. PR 36 integró esta
ampliación en `main` como `8261c51`. Ese cierre no acredita la CI ni la integración
de recursos de PR 37 o del incremento posterior de asociaciones. No se afirma
entrega a un destinatario externo.

Se ejecutó una sola verificación local a la vez, con pruebas focales después del
RED y sin otra regresión global. Resultados de fronteras independientes:

- Ocho pruebas de temporización y 16 de aplicación aprobadas.
- Tres pruebas de vigencia distinguen revisión humana de recálculo ordinario.
- Seis pruebas HTTP aprobadas tras corregir las rutas parametrizadas.
- 26 pruebas del cliente aprobadas; una prueba adicional del cursor futuro falló
  primero y aprobó después de corregir la comparación con el instante consultado.
- Nueve recorridos de navegador con HTTP controlado aprobaron en 18.891 s,
  incluidos escritorio y móvil, lectura incierta, conflicto, apertura exacta,
  retorno, continuación y revocación. Sus 2173 fuentes mantuvieron sus huellas.
  Se inspeccionaron las dos capturas de bandeja sin recortes ni solapamientos.

El adaptador HTTP de correo genérico tiene seis pruebas aprobadas contra un
servidor loopback: solicitud e idempotencia, aceptación explícita, respuestas
inciertas, rechazo y reintentos. No se contactó a un destinatario externo. Ocho
pruebas focales adicionales aprobaron configuración completa o deshabilitada,
límites de ciclo, parada y supervisión de ambos consumidores; esta composición
por puertos se comprobó antes de conectar el servidor. El adaptador PostgreSQL
se excluyó de aquella compilación; su comprobación posterior se distingue abajo.

La comprobación posterior de PostgreSQL real usó bases desechables, directorio
temporal privado en disco y un solo proceso de compilación y pruebas. La primera
pasada de siete objetivos terminó en 148.091 s: 20 casos aprobaron y dos pruebas
nuevas reprodujeron defectos de episodios y preferencias. Se conservaron las
fuentes Rust y SQL durante esa pasada; sólo cambiaron guiones de aceptación
independientes, que no ejecutó esa campaña.

Los 20 casos cubren preferencias y reintentos, aislamiento antes del límite de
bandeja, reinicio y lectura independiente de atención, origen histórico corrupto,
rollback de escaneo/activación/correo, fuente cambiada antes del trabajador,
arrendamiento y confirmación de intentos, payload congelado e incertidumbre más
allá de la ventana del proveedor, inventario de arranque y permisos mínimos.
Las dos pruebas fallidas exigieron resolver el aviso anterior al registrar
atención aunque se reabra antes del escaneo, y reactivar una ocurrencia nunca
emitida al habilitar sus canales. Tras corregirlo, sólo ese objetivo se repitió:
ambas aprobaron en 18.977 s, con 2219 fuentes sin cambios. Los avisos ya activados
conservan su identidad y no se reenvían por esa reactivación.

Una prueba adicional reprodujo la misma pérdida de episodio cuando se acepta
una revisión humana y otra fuente cambia antes del siguiente escaneo (RED en
11.519 s). Se conserva ahora esa aceptación en el estado y se verifica su
revisión histórica al resolver el aviso anterior. El caso nuevo y los dos
anteriores aprobaron juntos en 26.463 s, con las fuentes
sin cambios durante la campaña.

Dos pruebas adicionales de composición del servidor aprobaron en 25.417 s,
incluida compilación: fallo de bind sin iniciar consumidores y parada de ambos
con liberación final de los adaptadores fuera de Tokio. La implementación Rust
se mantuvo estable; continuó la preparación independiente de guiones HTTP y web.

Cuatro pruebas de ayuda CLI y límites de argumentos aprobaron en 6.011 s,
con fuentes sin cambios.

La primera aceptación HTTP integrada terminó con salida 1 en 84.430 s:
el escaneo sin raíces escribía progreso y auditoría y alteraba el oráculo
exacto de operaciones rechazadas del expediente cerrado. Se corrigió el
escaneo vacío para que permanezca sin escrituras y se conservó el oráculo
original. La repetición terminó en 239.232 s: aprobaron los recorridos previos
de documentos, participantes, expedientes, audiencias, calendarios, hechos,
perfiles, plazos, agenda y reinicios. La restauración detectó que una función
CHECK nueva dependía del `search_path` de la sesión; `pg_restore` lo deja vacío.
Se hizo autocontenido el predicado y se añadió una regresión específica. Esa
campaña conserva salida 1 y no acredita la aceptación final de alertas.

El CI de `1971c81` reprodujo el mismo defecto en Test y Coverage. Formato,
Clippy, MSRV, dependencias, release y verificación web aprobaron. El navegador
remoto aprobó 29 escenarios y falló en los dos nuevos de alertas porque su
fixture leía título y referencia fuera del objeto `administration`. Se corrigió
el fixture conforme al contrato existente, sin cambiar la respuesta del producto.

El recorrido local con servicios reales aprobó tres escenarios: alertas a
1440/390 píxeles y permisos de agenda con revocación. Tardó 233.822 s con
preparación; Playwright informó 26.7 s. Comprueba generación por el consumidor,
origen exacto, lectura persistente, preferencias y correo deshabilitado. Se
inspeccionaron ambas capturas, sin desbordamientos ni solapamientos. El fixture
de título/referencia se corrigió antes de cargarlo; durante la preparación se
editaron también el predicado SQL, su prueba y el guion API independiente. El
servidor ya compilado no incorporaba todavía la corrección SQL de restauración;
por ello este recorrido acredita la interfaz real, no esa corrección posterior.

Las dos regresiones nuevas de `search_path` vacío y escaneo sin raíces aprobaron
en una campaña focal de 18.053 s, con PostgreSQL real y 2220 fuentes sin cambios.
Se ejecutaron secuencialmente, sin repetir los otros objetivos PostgreSQL.

La aceptación API final aprobó en 273.199 s, con 2220 fuentes sin cambios.
Ejercitó el consumidor real, aviso de 48 horas, bandeja personal, preferencias,
lectura idempotente, origen exacto y denegaciones. El respaldo/restauración
comparó exactamente las ocho tablas de alertas con el servidor detenido y, al
reabrirlo, conservó preferencias, lectura, destinatario, origen e historia de
audiencia sin una segunda ocurrencia. También aprobaron los recorridos previos
y sus comparaciones de documentos, hechos, plazos, agenda, señales y reinicios.

Los resultados no se suman a campañas históricas ni acreditan recepción externa
de correo o cierre global. El CI ahora verifica las PR y los pushes a main,
sin ejecutar dos campañas equivalentes por cada actualización de una rama.

## Agenda combinada: checkpoint funcional del 19 de septiembre de 2026

`GET /api/v1/agenda` reúne audiencias y vencimientos operativos bajo una lectura
PostgreSQL autorizada y auditada. Comprueba dependencias antes de incluir un
plazo y conserva un cursor de candidatos examinados, incluso en páginas vacías
parciales. Qadra añade día, semana, mes y rango personalizado; acumula actividades
y abre su revisión exacta tras revalidar el expediente. El contrato está en
[agenda-api.md](agenda-api.md) y la decisión en [ADR-0038](adr/0038-authorized-combined-agenda.md).

El desarrollo usó TDD focal y un solo runner local. Las comprobaciones siguientes
son grupos distintos, no una nueva campaña global ni un porcentaje de avance:

| Frontera | Evidencia ejecutada |
| --- | --- |
| Aplicación y HTTP | 19 pruebas de aplicación y seis HTTP aprobaron después del RED. La ejecución conjunta conservó cinco fallos esperados del adaptador PostgreSQL aún sin implementar; no se declara verde esa ejecución completa. |
| PostgreSQL real | Siete pruebas aprobadas en 59.339 s, con bases desechables: mezcla, membresías, cambio de fuente sin despacho, 100 candidatos omitidos, corrupción, fallo de auditoría y revocación concurrente. |
| Cliente y periodos civiles | Ocho pruebas del cliente y 14 de periodos e instantes aprobadas, con RED previo; conservan nanosegundos y límites civiles. |
| Navegador con transporte controlado | 20 de 21 escenarios aprobaron; el restante detectó la falta de regreso desde el plazo a Agenda. Se añadió el botón y sólo ese escenario se repitió, aprobado en una campaña de 8.268 s. La primera pasada había detectado selectores de prueba que confundían texto de etiquetas envolventes con nombres accesibles; se corrigieron a combobox. |
| Navegador con servicios reales | Tres escenarios aprobados en 269.684 s, incluidos preparación y build; Playwright informó 28.1 s. Dos recorren la agenda a 1440/390 píxeles; el tercero comprueba asignaciones, cuatro roles, cierre y revocación. |

La aceptación real consulta los tres periodos, comprueba la respuesta mezclada,
abre audiencia y plazo exactos, modifica la fuente y observa el plazo pendiente
fuera de Agenda, con cálculo e historia intactos y auditoría válida. No enumera
expedientes desde el navegador. Sus 2118 fuentes conservaron huellas durante
la campaña; el formato posterior de los archivos de prueba no cambió su lógica.
Un revisor independiente no encontró defectos concretos por lectura en permisos,
cursores, vigencia, atomicidad ni proyección HTTP; esto no sustituye las pruebas.

Se inspeccionaron cuatro capturas reales. Conservan los componentes y colores
Qadra y no desbordan la página; el mes usa desplazamiento horizontal contenido
en móvil. Esa revisión detectó tarjetas mensuales demasiado altas. La presentación
se compactó después a familia, revisión, hora exacta, título y referencia, con
descripción accesible completa y detalle al abrir. Omite sólo fracciones nulas.
La comprobación focal de los tres periodos a 1440/390 píxeles detectó un desborde
móvil causado por la descripción oculta fuera de la tarjeta. Al contenerla dentro
del botón, ambos escenarios aprobaron en 9.805 s, sin cambios de fuentes durante
la campaña. Se inspeccionaron las capturas del mes en escritorio y del encuadre
móvil; este último conserva desplazamiento horizontal para consultar los días
fuera del área inicial. No se repitió la campaña real por este ajuste visual.

El guion API contrastó la consulta mixta, filtros, continuación, permisos y
vigencia antes y después de restaurar, reutilizando los registros existentes.
La campaña completa terminó con salida 0 en 235.844 s y conservó las huellas de
sus 2118 fuentes; también verificó reinicios del consumidor y la historia exacta.
La aceptación transversal posterior reunió cinco expedientes asignados al mismo
Litigante (tres audiencias y dos plazos vigentes) y excluyó un sexto expediente
sin asignación. Las vistas de día, semana y mes usaron una consulta de agenda,
sin enumerar expedientes, y abrieron las cinco revisiones exactas. Ambos recorridos
reales, a 1440/390 píxeles, aprobaron en 214.055 s con preparación; Playwright
informó 16.4 s. Las 2119 fuentes conservaron sus huellas. Se reutilizó el operador
y calendario, con políticas fijas y datos independientes de la prueba Follow.
Se inspeccionaron sus dos capturas: las cinco tarjetas conservan legibilidad
en escritorio y el calendario mantiene desplazamiento contenido en móvil.
La agenda se integró en `main` mediante squash de la
[PR 35](https://github.com/eddndev/TT2026-B136/pull/35), commit `c16b820`,
el 19 de septiembre de 2026. El [CI de cierre](https://github.com/eddndev/TT2026-B136/actions/runs/35434470861)
aprobó formato, Clippy, MSRV, pruebas, cobertura, dependencias y binario de
release para `79a618b`; Web aprobó sus dos trabajos. El PDF de esa revisión
pasó compilación y revisión visual según el informe académico. Estos controles
remotos no se repitieron como otra campaña completa local.
La aceptación de esta agenda no acredita alertas, activación automática ni
perfiles jurídicos calificados.

## Seguimiento humano V2 y composicion local: 19 de septiembre de 2026

La entrega se integró en `main` mediante squash de la
[PR 34](https://github.com/eddndev/TT2026-B136/pull/34), commit `e2e6758`,
el 19 de septiembre de 2026. La campaña de
[CI de la PR](https://github.com/eddndev/TT2026-B136/actions/runs/35431913733)
aprobó formato, Clippy, MSRV, pruebas, cobertura, política de dependencias y
binario de release para `24da17d`. También aprobaron los workflows Web
y Documents de esa revisión. Esta evidencia remota cierra la regresión de
reevaluación; no se repitió la campaña completa localmente ni se atribuye
a la agenda posterior.

El servicio humano prepara y confirma seguimiento V2 con politicas explicitas,
autor autenticado completo y continuidad administrativa verificada. Detalle y
listado comparan cabezas comprobadas dentro de la transaccion autorizada y
auditada; el resultado operativo no se deduce de la cola. Qadra separa ese
resultado del calculo conservado y muestra la historia humana y tecnica.
`serve` compone un consumidor serial con pausa, presupuesto por ciclo y cierre
supervisado de HTTP y operaciones bloqueantes.

### Verificacion focal terminada

Las siguientes ejecuciones son grupos separados de esta ampliacion. No se suman
como una suite global ni sustituyen las campanas completas de la entrega anterior.
Cada suite se ejecuta de forma exclusiva: Cargo con un proceso de compilacion y
un hilo de pruebas, Node con concurrencia 1 y Playwright con un worker. Los
colaboradores no ejecutan compilaciones ni suites paralelas.

| Grupo | Resultado confirmado |
| --- | --- |
| Aplicacion: vigencia, enlace de resumen, consultas y servicio actual | 50 pruebas; siete targets. |
| HTTP Rust: proyecciones, contexto, solicitudes y limites V1/V2 | 160 pruebas focales. |
| PostgreSQL: detalle, guardas, listado y regresion de registros | 17 pruebas en bases desechables. |
| Lectura actual y escritor concurrente | Una prueba PostgreSQL: espera observada en el bloqueo de auditoria, historia y orden de eventos conservados. |
| Dispatcher: reconexion, atomicidad y presupuestos | 11 pruebas PostgreSQL; incluye cuatro de reconexion con inventario y recuperacion. |
| Cliente y presentacion de plazos | 88 pruebas Node. |
| Navegador de plazos con HTTP controlado | 36 escenarios, incluidos ocho nuevos de V2; escritorio 1440 y movil 390. |
| Navegador completo con servicios reales | 25 escenarios, incluidos dos nuevos de reevaluacion Follow en escritorio y movil. |
| Binario compuesto y opciones de serve | 31 pruebas unitarias y cuatro de ayuda/configuracion. |
| Clippy de aplicacion, infraestructura y web | Todos sus targets, warnings denegados; comprobacion anterior a la composicion de serve. |
| Clippy del binario compuesto | Todos sus targets, warnings denegados. |

Las 26 pruebas del ciclo y supervisor estan incluidas en las 31 unitarias del
binario: 13 del ciclo serial, cuatro de parada y nueve de supervision. Otras
dos pruebas ejercitan la composicion real y observan que los propietarios
finales del router y adaptadores se liberan fuera de Tokio, tanto al fallar
el bind como al detenerse el consumidor. Antes de
implementar el ciclo fallaron 12 de sus 13 escenarios y la nueva prueba de
opciones; antes de implementar parada y supervision fallaron sus 13 escenarios.
La repeticion confirmo limites, alternancia, pausa, errores recuperables,
parada entre llamadas, conservacion del join y drenaje de ambos lados. Los
canales y barreras observan operaciones en curso; las esperas de vigilancia
no constituyen la evidencia de exclusion.

La primera pasada de los ocho escenarios nuevos de navegador detecto una
asercion que contaba consultas de revisiones del perfil como si fueran del plazo.
Se restringio a las rutas de plazos, conservando la exigencia de una sola
conciliacion y ningun reenvio automatico. La repeticion y la regresion de 36
escenarios aprobaron. Las capturas inspeccionadas conservan el diseno Qadra y
la lectura movil sin desbordamiento horizontal.

### Aceptacion integrada ejecutada y cierre pendiente

La primera pasada del navegador real se interrumpio tras un escenario aprobado,
uno fallido, uno interrumpido y 22 sin ejecutar. La recarga posterior al cierre de
sesion no podia importar la interfaz. Una regresion reducida sin PostgreSQL ni
Redis reprodujo 52 solicitudes de scripts con `ERR_INSUFFICIENT_RESOURCES`.
`/tmp` era un tmpfs con presion de espacio y la swap estaba practicamente llena.
Se probaron importaciones diferidas, pero no eliminaron el fallo y se retiraron.
La misma interfaz original completo las tres recargas del caso en 7.588 s al
ubicar `TMPDIR` en disco. Esta comparacion identifica una condicion del entorno
local; no establece la causa de un OOM anterior del equipo. Las campanas
posteriores usan temporales privados en disco y mantienen la ejecucion serial.
La regresion se conserva en `web/tests/browser/app-loading.spec.mjs`.

La repeticion completa con temporales en disco termino con salida 0 en
437.344 s: **25 escenarios aprobados**, incluidos los dos nuevos de seguimiento
Follow en escritorio 1440 y movil 390. Playwright informo 4.4 minutos para los
recorridos, ademas de preparar servicios y datos. Se inspeccionaron seis capturas
de calendario actualizado, revision pendiente y confirmacion humana; conservan
el diseno Qadra y no presentan recortes ni desbordamiento horizontal. Las 2074
fuentes comprobadas conservaron sus huellas durante la ejecucion. La campana
uso Node 22.22.2, PostgreSQL 18.6, Redis desechable y un unico worker.

Los datos de ambos recorridos se crearon mediante HTTP en expedientes distintos.
Cada uno contrasta la revision tecnica por calendario, la fuente cambiada con
calculo conservado y fecha operativa nula, y la aceptacion explicita de Litigator
desde Qadra. Consulta R1 a R4 exactas, autor y causa, historial y auditoria.
Los perfiles son sinteticos; la aprobacion no califica una regla juridica.

Los calendarios sinteticos de los dos escenarios nuevos llevan titulos distintos
del calendario de la regresion previa, para evitar selectores ambiguos. El nuevo
guion de datos tambien figura en los filtros de cambios del workflow Web.

La primera campana HTTP con servicios reales completo el cambio de calendario,
la revision pendiente por cambio de fuente, la correccion humana, la historia
exacta y un cierre SIGTERM con salida 0. Tras reiniciar, el consumidor proceso
un nuevo evento testigo. El guion fallo despues al solicitar 100 revisiones de
historia, fuera del maximo 20 del contrato; la API rechazo correctamente con
`invalid_query`. Se corrigio solo esa peticion en los guiones API y navegador.
La repeticion completo ambos reinicios con salida 0 (SIGTERM y SIGINT),
retomo el procesamiento sin duplicar revisiones y conservo R1 a R5. Durante
la restauracion detecto otra precondicion antigua del guion: la coleccion de
calendarios esperada se habia capturado antes de publicar el calendario del
nuevo escenario. Se anadio una captura de las colecciones mutables justo antes
del respaldo; se conservan las expectativas originales de revisiones y dias
exactos. La tercera ejecucion completo el guion con salida 0 en 215.613 s,
incluidos ambos reinicios, el evento testigo sin duplicados y el cotejo de R1
a R5 despues de `pg_restore`. La huella de sus 2073 archivos de fuente no cambio
durante la ejecucion. Tambien conservo las 14 respuestas historicas del flujo
previo de plazos y las comparaciones de los demas modulos.

Esta tercera pasada emitio una advertencia en otra comprobacion del guion: un
token de prueba que empezaba por guion se interpreto como opcion de `rg` al
buscarlo entre claves Redis. Se corrigio a `rg -F --` y una comprobacion focal
confirmo la coincidencia literal y el rechazo de una clave distinta. El recorrido
de reevaluacion y restauracion termino. La repeticion final del guion completo
con esa correccion aprobo en 215.301 s: incluyo la asercion de sesiones, los
reinicios y la restauracion. Sus 2074 fuentes conservaron las huellas.

### Ajuste focal detectado en CI

Clippy 1.98.1 rechazo la implementacion manual de un despertador sin efecto en
el auxiliar de pruebas del supervisor (`manual_noop_waker`). Se sustituyo por
`Waker::noop()`, conservando la conduccion manual del futuro y sus aserciones.
Las nueve pruebas del supervisor aprobaron de nuevo en 4.918 s, sin repetir
la suite global local. Clippy 1.98.1 del binario y todos sus targets aprobo
en 28.386 s con warnings denegados. Esta correccion no cambia el consumidor
de produccion.

### Checkpoint funcional y regresion de cierre

La comprobacion final del cliente completo aprobo 284 pruebas Node en 9.390 s,
formato en 6.969 s y build en 3.898 s, sin cambios de las fuentes durante las
comprobaciones. Son ejecuciones posteriores al grupo focal de 88 pruebas;
no se suman ambos conteos.

Se detuvo deliberadamente la regresion amplia de navegador con salida 130
tras 175.837 s, antes de completar todos los escenarios. Esa ejecucion no
acredita una suite aprobada. La serie se detuvo ahi: no ejecuto los controles
Rust, MSRV, CLI y API que estaban encadenados despues. No quedaron suites
locales activas. La evidencia focal y los 25 recorridos reales anteriores
sostienen el checkpoint funcional; la regresion de cierre se realiza sobre
la revision publicada antes de integrar, aprovechando los controles de CI.

CI cubre Rust con backends reales, formato, Clippy, MSRV, cobertura, politicas
de dependencias, build release y las suites web completas. No ejecuta los
guiones CLI ni API; la aceptacion API final con la asercion corregida se
ejecuto localmente y no se deduce del navegador real.

El guion CLI completo tambien aprobo en 6.355 s, incluido el ciclo de evidencia,
rechazo de alteraciones, revocacion y autenticacion. No se repitio la campana
global local. La guia de Qadra se actualizo para explicar las politicas y la
revision humana, eliminando la afirmacion anterior de que no habia seguimiento.

Las fuentes academicas afectadas estan actualizadas. El PDF de CI con el arbol
`f743c5e` tiene 315 paginas; su revision visual dirigida termino despues de
corregir un desborde en el anexo. La procedencia, huella y 22 paginas comprobadas
se documentan en [la revision academica](academic-report-verification.md).

La [regresion Web remota de f467624](https://github.com/eddndev/TT2026-B136/actions/runs/35430535047)
aprobo formato, build, 284 pruebas Node, 292 escenarios con HTTP controlado y
25 con servicios reales. Verifico el merge de revision `3f3b06d`; las dos
correcciones posteriores hasta `f743c5e` solo cambiaron el anexo LaTeX. Node
informo 2277.685713 ms; las campanas de navegador, 4.6 y 2.6 minutos. El navegador
controlado uso dos workers remotos y el real uno; localmente se conserva un
solo runner. Estos resultados no se suman a sus focales locales.

El [workflow Rust de ese corte](https://github.com/eddndev/TT2026-B136/actions/runs/35430535048)
aprobo formato, Clippy, MSRV 1.88, dependencias y build release de 16894024 bytes.
Test y Coverage seguian en curso al registrar este corte: no se atribuye aun
un nuevo conteo global Rust ni una nueva cobertura. La integracion requiere
los controles de cierre aprobados en la revision que finalmente se publique.

Estos resultados locales no acreditan por si mismos una campana global
Rust/MSRV, cobertura nueva ni integracion en main. CI comprueba esos controles
sobre la revision publicada. Los resultados siguientes conservan
sus revisiones y alcance historicos.

## Consumidor durable de plazos: 18 de septiembre de 2026

El [consumidor](deadline-worker.md) confirma una revisión técnica, un resultado
sin cambios o un intento fallido verificable por cada trabajo elegible. La
revisión, el resultado y su auditoría comparten transacción; un fallo revierte
esa transacción antes de registrar el intento. Las migraciones `0020_` conservan
ambos historiales, la procedencia del trabajo y sus restricciones de escritura.
Al cierre de esa campaña el servidor todavía no componía el despachador y el
consumidor; el servicio humano, HTTP y Qadra mantenían V1. Las verificaciones de este incremento no
acreditan ese recorrido operativo V2, agenda conjunta ni alertas.

### Regresiones reproducidas y correcciones

El primer ensayo de restauración detectó que PostgreSQL reconstruía con otra
agrupación algunas expresiones `BETWEEN` combinadas con `AND`. Se sustituyeron
por comparaciones explícitas equivalentes en las nuevas tablas y se conservaron
las comprobaciones exactas del catálogo. La repetición aprobó cinco casos:
tres de catálogo/permisos y dos de recuperación, incluido un `pg_dump` y
`pg_restore` real sin migración reparadora. El ensayo compara las once
expresiones `CHECK` antes y después de restaurar, además de datos y recibos.

Tres regresiones focales comprobaron que un perfil corrupto se clasificaba como
fallo transitorio y que se perdía la categoría nativa de dos errores SQL.
Tras conservar categorías tipadas a través de los puertos, los tres casos
aprobaron: evidencia inconsistente con espera de una hora, bloqueo `55P03` e
interrupción `57014`. Los diagnósticos no se interpretan como códigos ni se
exponen en HTTP. Esto no constituye una campaña de desconexiones físicas o TLS.

La primera campaña global iniciada quedó interrumpida durante compilación y no
se contabiliza como aprobada. Después de comprobar que sus procesos habían
terminado, se cerró exclusivamente su PostgreSQL desechable identificado.
La siguiente campaña, ya serializada, se detuvo en una preparación de prueba:
las nuevas claves foráneas impedían eliminar la clave primaria del trabajo.
La alteración deliberada del catálogo ahora elimina también sus dependencias;
el validador de producción conserva sus comprobaciones estrictas.

Una regresión dirigida confirmó también que el truncado aislado se rechaza por
la clave foránea antes del guard de inmutabilidad. La prueba exige ese rechazo
y comprueba además el SQLSTATE y mensaje exactos del guard para truncado en
cascada y conjunto. Las dos suites corregidas aprobaron tres casos cada una,
con conservación del estado después de los rechazos. No se relajaron las
restricciones de producción.

La siguiente campaña se detuvo en una expectativa antigua de timeout del
despachador: el puerto de auditoría ya conservaba la categoría tipada de bloqueo
pero la prueba esperaba un error genérico. La repetición dirigida aprobó los
dos escenarios exigiendo `Busy` e `Interrupted`, respectivamente, con rollback
y recuperación de la misma conexión. El rechazo de la segunda prueba en la
campaña previa correspondió al mutex envenenado por la primera aserción.

### Condiciones de ejecución

Todas las verificaciones locales de cierre se coordinan de forma secuencial,
con `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1` y un bloqueo exclusivo del
coordinador. No se superponen Cargo, otras suites, cobertura ni compilación
documental. Se usa PostgreSQL 18.6 y Valkey 8.1.9, este último mediante los
comandos y el protocolo Redis del entorno de pruebas. Sus instancias son
desechables. qpdf corresponde a la versión 12.4.1 exigida por el repositorio;
la compilación ordinaria usa Rust 1.94.0.

La campaña completa terminó con **2630 pruebas Rust aprobadas,
cero fallidas y una TSA externa ignorada**, en 459 resúmenes.
Los manifiestos registran 1581 fuentes y conservan las huellas de cada
ejecución. La lógica de producción no cambió entre los seis controles.
La suite se ejecutó con `cargo test --workspace --no-fail-fast` dentro de
`scripts/test-backends.sh`; recopilar todos los fallos no introduce paralelismo.

| Control | Resultado confirmado |
| --- | --- |
| Formato del workspace | Salida 0; 3.025 s. |
| Compilación del workspace | Salida 0; 0.347 s. |
| Suite Rust con servicios aislados | Salida 0; 1482.506 s. |
| Clippy 1.98.1, warnings denegados | Salida 0; 15.586 s. |
| MSRV 1.88, todos los targets | Salida 0; 166.722 s. |
| API y restauración V1 | Salida 0; 151.915 s. |

El primer Clippy detectó un atributo `allow(dead_code)` duplicado al importar
un helper en dos archivos de pruebas. Se eliminó solamente la anotación
redundante; no cambiaron lógica ni aserciones. Los cuatro casos de esos dos
targets se repitieron con servicios aislados y aprobaron antes de completar
Clippy, MSRV y la demo. Esa repetición no se suma al total global. El manifiesto
contrasta exactamente las dos eliminaciones con las fuentes de la suite completa.

Los diez targets PostgreSQL del consumidor aprobaron **27 pruebas** dentro de
esa suite, incluidas familias, historia, inventario, permisos, atomicidad,
concurrencia, espera, clasificación y restauración. Las once pruebas focales
iniciales, cinco de catálogo/recuperación y tres de clasificación conservan sus
ejecuciones propias; no se suman otra vez al total global. También se ejecutaron
las pruebas de categorías nativas SQL, su conservación en lectores y la
respuesta HTTP opaca.

La demo HTTP confirma compatibilidad y restauración del flujo V1 bajo el
esquema ampliado. No compone ni acredita HTTP/Qadra V2 o ejecución del trabajador
desde `serve`. No se midió cobertura, rendimiento ni navegador en esta campaña.
La CI del incremento publicado debe comprobarse sobre su nueva cabeza.

## Despacho persistente de plazos: 18 de septiembre de 2026

El [despachador](deadline-dispatch.md) confirma trabajos únicos, cursores y
auditoría en una transacción. Consume las cinco familias de eventos y conserva
un recorrido recurrente del legado. Las migraciones `0019_` añaden restricciones,
reservas de operación y comprobaciones de esquema, permisos e inventario.
Al cierre de esta campaña estaban pendientes el consumidor y la confirmación
de revisiones técnicas; servicio/HTTP y Qadra conservaban V1 y el despachador
no se invocaba desde `serve`. El corte posterior del consumidor se registra arriba.

La campaña global con PostgreSQL/Redis aislados y qpdf 12.4.1 terminó con
**2584 pruebas Rust aprobadas, 0 fallidas y 1 ignorada**,
en 446 resúmenes. La prueba ignorada corresponde al proveedor TSA
externo. Las fuentes conservaron sus huellas durante los seis controles.

| Control | Resultado confirmado |
| --- | --- |
| fmt | Salida 0; 2.936 s. |
| build | Salida 0; 0.360 s. |
| workspace | Salida 0; 793.321 s. |
| clippy | Salida 0; 1.816 s. |
| msrv | Salida 0; 1.337 s. |
| api | Salida 0; 168.757 s. |

Las 26 pruebas propias del despachador están incluidas en esa suite, no se
suman otra vez. Cubren paginación, UUID nulo y máximo, huecos de secuencia,
eventos vacíos, selección de cabeza y ámbito, legado añadido bajo un cursor
anterior y dos despachadores reales. Comprueban además colisiones de operación
en ambos sentidos, fallo de auditoría después de escribir trabajos/cursor,
respuesta perdida, migración repetida y pérdida del singleton sin reparación.

Las comprobaciones directas rechazan cursores que saltan candidatos y cambios
en tablas, restricciones, índices, triggers, funciones o permisos, incluidos
PUBLIC y roles heredables o asumibles. Un trabajo histórico sigue siendo válido
si una corrección posterior cambia la dependencia o retira el plazo.

Las pruebas de timeout mantienen un bloqueo ajeno o una sentencia lenta activa
y exigen que el adaptador salga sin cancelación externa. Verifican rollback y
recuperación de la misma conexión. Los dos escenarios se serializan desde la
preparación hasta la limpieza porque sus esquemas comparten el advisory lock
de la base de datos. El watchdog sólo evita una prueba infinita;
si actúa, el caso falla. La restauración usa un dump real con trabajos y ambos
cursores parciales, conexión con `search_path` vacío y reapertura sin migrar.
La continuación conserva filas, operaciones y auditoría anteriores y completa
ambos recorridos sin duplicados.

### Medición de páginas y límites

Una campaña independiente creó 240 plazos y procesó tres eventos con límites
1, 20 y 100, comprobando 720 trabajos. Antes del cambio, el helper reunía todos
los candidatos antes de aplicar el filtro y límite exteriores. Ahora el
intervalo, filtro de trabajos pendientes y límite se aplican dentro del helper:
materializa hasta 101 filas por página y una por consulta de guard.

| Límite | Mediana anterior por página (ms) | Mediana con límites internos (ms) |
| --- | --- | --- |
| 1 | 8.911 | 8.324 |
| 20 | 116.289 | 118.248 |
| 100 | 552.286 | 529.158 |

Son observaciones de dos campañas locales, no una prueba estadística de mejora
general ni una garantía de producción. El plan de la consulta exterior confirma
la reducción de filas materializadas; no mide por sí solo todas las raíces
examinadas dentro de PL/pgSQL. Las coincidencias dispersas aún pueden exigir
recorrer muchas raíces. Los presupuestos de un segundo para bloqueos y cinco
por sentencia no limitan el tiempo total de una página ni de la apertura.

La demo HTTP posterior comprueba compatibilidad y restauración del flujo V1
bajo el esquema ampliado; la prueba específica de dump/restore anterior cubre
los trabajos y cursores. Ninguna acredita ejecución del consumidor ni Qadra V2.
Los 18 controles remotos de `179115d` aprobaron y corresponden al almacenamiento
humano anterior. La CI de este incremento debe comprobarse sobre su nuevo commit.

## Persistencia humana V1/V2: 18 de septiembre de 2026

El adaptador PostgreSQL incorpora capturas y observaciones V2 sin modificar
los bytes históricos V1. Revalida las cuatro acciones humanas con el usuario
real, conserva ambas huellas del predecesor y reconstruye material histórico
exacto. Las migraciones `0018_` añaden seguimiento separado del cálculo,
parsers estrictos y un guard compatible con selección histórica `Fixed`.
La lectura no recalcula fechas ni completa padres que el legado no observó.

La campaña completa ejecutó `scripts/test-backends.sh` con PostgreSQL de
identidad, expedientes y documentos, Redis desechable y qpdf 12.4.1.
Terminó con **2558 pruebas aprobadas, cero fallidas y una TSA externa
ignorada**, en 436 resúmenes. Los casos comprobados en campañas focales
se ejecutan también en esta suite; las repeticiones de aquellas campañas no se
suman a este total.

| Control | Resultado confirmado |
| --- | --- |
| Formato del workspace | Salida 0, 3.832 s. |
| Compilación del workspace | Salida 0, 0.343 s. |
| Suite Rust con servicios aislados | Salida 0, 663.489 s. |
| Clippy 1.98.1 con warnings denegados | Salida 0, 29.008 s. |
| MSRV 1.88, todos los targets | Salida 0, 26.607 s. |
| API y restauración con servicios reales | Salida 0, 152.118 s. |

Las 1483 fuentes de la campaña conservaron sus huellas. Se reprodujeron
los fallos de codec, parsers, esquema y persistencia antes de implementar sus
fronteras. Los controles SQL anteriores rechazaban perfiles fijos históricos
y aceptaban enlaces de predecesor, cambios de política en atención y recibos
de observación incoherentes. Sus pruebas directas verifican los rechazos y
la ausencia de escrituras parciales tras la corrección.

La primera campaña global de almacenamiento se detuvo en la prueba de
inmutabilidad de eventos: `TRUNCATE` devolvió `0A000` porque la nueva
clave foránea impide truncar la tabla referenciada antes de ejecutar su
guard. Se reprodujo el error completo y se ajustó la expectativa por
sentencia. La prueba exige además `23514` y el mensaje exacto del guard
para `TRUNCATE CASCADE` y truncado conjunto, manteniendo el snapshot
después de cada rechazo. Las diez pruebas del emisor aprobaron antes de
repetir esta campaña completa. No se alteraron las restricciones de datos.

La aceptación incluye actualización desde un esquema 0017 real con cuatro
registros V1, migración repetida, reapertura, autor humano y auditoría,
notificaciones con padre histórico y observado distintos, administración
histórica y detección de falsificaciones con hashes coherentes. Los controles
del inventario reconstruyen evidencia, además de comprobar sus huellas.

Los cinco controles Rust terminaron con salida cero. El primer intento HTTP
quedó interrumpido por una terminación del runner con señal 15, sin un
resultado completo de la demo. Tras comprobar que sus procesos habían
terminado y detener su PostgreSQL desechable, se repitió solamente ese
control con servicios nuevos. La tabla recoge la repetición terminada;
las huellas confirman que no cambiaron las fuentes entre ambos intentos.

El recorrido HTTP repetido sigue produciendo V1: comprueba la compatibilidad
del flujo existente y su restauración bajo el esquema ampliado. No acredita
una API V2 ni restauración de trabajos automáticos. El guard humano sigue
rechazando autoría técnica hasta disponer del servicio y trabajo durable.
Al cierre de esta campaña, la PR 34 permanecía en borrador y estaban pendientes
despacho, trabajador, HTTP V2 y Qadra. Los cortes posteriores de despacho y
consumo se registran arriba.

La CI de `8c838cb` terminó con sus 18 controles aprobados. Corresponde al
preparador técnico publicado antes de este cambio de almacenamiento; la CI
del incremento actual debe comprobarse sobre su propio commit.

## Preparación técnica de seguimiento: 18 de septiembre de 2026

Este incremento añade el preparador técnico puro de revisiones y resultados
sin cambios. Procesa evidencia verificada, conserva decisiones humanas y
admite eventos anteriores a la cabeza actual sin sustituir su causa original.
La base de contratos V2 se publicó en
[PR 34](https://github.com/eddndev/TT2026-B136/pull/34), commits `2680455` y
`b185534`; los resultados de esta sección corresponden al incremento posterior.
Al cierre de este incremento, la PR seguía en borrador: persistencia V2,
trabajos durables, HTTP y Qadra de reevaluación todavía no estaban conectados.
El estado posterior y sus verificaciones se describen en los cortes siguientes.

La campaña completa ejecutó `scripts/test-backends.sh` con PostgreSQL de
identidad, expedientes y documentos, Redis desechable, Rust 1.94 y qpdf 12.4.1.
Terminó con **2442 pruebas aprobadas, cero fallidas y una TSA externa ignorada**.
Los **415 resúmenes de resultados** no son 415 pruebas. Las 48 pruebas añadidas
desde la base publicada están incluidas en ese total; no se suman las
repeticiones focales.

| Control | Resultado confirmado |
| --- | --- |
| Formato del workspace | Salida 0, **2.502 s**. |
| Compilación del workspace | Salida 0, **18.436 s**. |
| Suite Rust con servicios aislados | **2442 aprobadas, 0 fallidas, 1 ignorada**, salida 0, **761.287 s**. |
| Clippy 1.98.1, workspace y todos los targets | Warnings denegados, salida 0, **39.597 s**. |
| Rust 1.88, workspace y todos los targets | Salida 0, **31.655 s**. |

Las **1436 fuentes** capturadas conservaron sus huellas durante la campaña.
Los **21 archivos Rust** modificados en este incremento son ASCII y menores
de 400 líneas. Los adaptadores se ejercitaron con sus servicios configurados;
la TSA externa omitida no representa una campaña con un proveedor real.

Las pruebas cubren preparación técnica y conservación de políticas, atención,
responsable y cálculo histórico; calendario seguido; motivos pendientes;
bootstrap del legado y resultados tipificados sin cambios. Los casos nuevos
de continuidad cubren eventos atrasados y avances simultáneos. La revisión
detectó y reprodujo antes de corregir dos brechas: la transición V1/V2 debía
reconstruir las observaciones históricas para comprobar causa y avance, y los
resultados sin cambios debían rechazar regresión o alteración administrativa.
La última corrida focal aprobó **71 casos**, incluidos casos anteriores.
La corrida previa de aplicación con 915 aprobadas precede a las últimas
correcciones; no sustituye esta campaña final.

No cambió el recorrido HTTP integrado en este incremento. Su última ejecución
con restauración fue la campaña de la base publicada, registrada abajo:
14 respuestas exactas de plazos y evidencia ZIP idéntica. Comprueba V1; no
acredita un almacenamiento V2. Tampoco se repitieron los recorridos locales
de navegador sobre este cambio puro de aplicación.

Se consultaron los controles remotos de `b185534`: los **18 controles** de
push y pull request terminaron aprobados, incluidos Test, Coverage y navegador
con servicios reales. Son evidencia de esa cabeza publicada; no se atribuyen
a los cambios posteriores hasta que su propia campaña remota termine.

## Núcleo local V2 de plazos: 18 de septiembre de 2026

Este corte amplía la base integrada por
[PR 33](https://github.com/eddndev/TT2026-B136/pull/33), commit `214202a`.
Corresponde al núcleo local de aplicación; no atribuye V2 a esa PR ni acredita
su conexión a persistencia, trabajador durable, HTTP o Qadra. El contrato se
describe en [los recibos de seguimiento](deadline-tracking-receipts.md).

Las corridas focales comprobaron los grupos siguientes. Todos los resultados
de esta tabla terminaron sin fallos ni casos ignorados; las repeticiones y los
casos compartidos con cortes anteriores no se suman como pruebas nuevas.

| Grupo | Resultado comprobado | Alcance |
| --- | --- | --- |
| Observaciones completas | **31 aprobadas**: 10 de construcción, 7 de evidencia, 4 de desplazamientos temporales y 10 de validación. | DLOE1, recibos exactos, ámbitos, pares selección/cabeza, padre de notificación, alteración de metadata e igualdad de revisión. |
| Reconstrucción del legado | **7 aprobadas**, más **1 vector V1** repetido. | Sólo material capturado; sin padre ni cabeza actual inventados; conservación de DLRV1, DLST1 y DLTX1. |
| Registro V2 | **8 aprobadas**. | Versiones explícitas, autoría humana/técnica, políticas, observaciones, revisión y recibos. |
| Preparador humano | **13 aprobadas**. | Calificación explícita, selección fija, cabezas disponibles, atención/retiro, transición desde legado y administración capturada. |
| Consultas V2 de aplicación | **4 aprobadas**. | Fecha operativa sólo con revisión aceptada y plazo activo; historia con ambas huellas del predecesor y rechazo de V2 a V1. |
| Continuidad de revisiones | **34 aprobadas**: 17 generales, 5 de calendario, 7 de dependencias y 5 de administración. | Preservación por acción, calendario seguido, motivos para todas las dependencias avanzadas y administración sin retrocesos ni reescritura. |
| Frontera V1 de infraestructura | **2 aprobadas**. | Autor humano conservado y rechazo tipificado de autor técnico en el adaptador V1. |
| Frontera HTTP V1 | **5 aprobadas**. | JSON humano conservado y rechazo de autor, recibo, acción o estado V2 que la proyección V1 no puede representar completos. |

Los grupos de preparación, consultas y registro terminaron aprobados en la
integración de aplicación posterior a sus fallos iniciales. La revisión de
continuidad detectó además regresión administrativa y ausencia de motivo al
avanzar otra dependencia seguida; se reprodujeron y corrigieron antes de la
última corrida focal y de la campaña completa registrada abajo. El padre de
notificación se comprueba en el ámbito fuente. Estas pruebas comparan evidencia
capturada sin recalcular la aritmética histórica.

### Campaña global y controles completados

La repetición completa ejecutó `cargo test --workspace` en el entorno preparado
por `scripts/test-backends.sh`, con PostgreSQL de identidad, expedientes y
documentos, Redis desechable, Rust **1.94.0** y qpdf **12.4.1**. Terminó con
código de salida cero:
**2394 aprobadas, 0 fallidas y 1 TSA externa ignorada**, en **600.183 s**.
El registro contiene **408 resúmenes de resultados**, que no equivalen a 408
pruebas. Los casos focales anteriores están incluidos en el total y no se
suman nuevamente. Los adaptadores se ejecutaron con sus variables de servicios;
no se atribuye éxito a retornos por ausencia de configuración.

| Control de este corte | Resultado confirmado |
| --- | --- |
| Formato del workspace | Aprobado, salida 0, **2.145 s**. |
| Compilación del workspace | Aprobada, salida 0, **6.510 s**. |
| Suite Rust con servicios aislados | **2394 aprobadas, 0 fallidas, 1 ignorada**, salida 0, **600.183 s**. |
| Clippy 1.98.1, workspace y todos los targets | Aprobado con warnings denegados, salida 0, **14.740 s**. |
| Rust 1.88, `check --workspace --all-targets` | Aprobado, salida 0, **43.570 s**. |
| `scripts/api-demo.sh`, servicios y restauración reales | Aprobado, salida 0, **138.696 s**. |

La campaña global anterior se interrumpió deliberadamente antes de terminar
para corregir los defectos de continuidad; no se contabiliza como aprobada.
Las duraciones registradas corresponden a comandos de verificación, incluida
la preparación cuando procede; no miden latencia de producto. La advertencia
de incompatibilidad futura de `redis 0.25.4` se conserva como diagnóstico de
dependencia y no se oculta.

### Restauración y límites del cierre local

El recorrido API terminó completo y restauró **14 respuestas exactas de plazos**,
incluidos resultados y recibos capturados, junto con los módulos persistidos
anteriores. La comparación conservó el ZIP de evidencia idéntico. Este recorrido
comprueba la regresión del almacenamiento y del HTTP V1; no acredita persistencia
ni restauración V2.

Las **1424 fuentes** del manifiesto de verificación conservaron todas sus huellas
al terminar la campaña. Los **72 archivos Rust modificados** son ASCII y menores
de 400 líneas. Las actualizaciones del informe y del estado del producto no
modifican esas fuentes. Las campañas históricas de navegador y CI de PR 33
permanecen separadas; la publicación e integración de este núcleo local requieren
sus propios controles remotos.

La suite global verifica el código local existente, incluidos los controles
provisionales V1; no crea persistencia V2 ni demuestra un trabajador que aún
no está implementado. Tampoco acredita HTTP/Qadra V2, alertas, un corpus
jurídico aprobado o aceptación integral del producto.

## Corte focal previo: políticas y codecs, 18 de septiembre de 2026

Sobre la base integrada `214202a` se ejecutaron **52 pruebas focales de
aplicación: 52 aprobadas, 0 fallidas y 0 ignoradas**. Incluyen 14 decisiones
ante cambios de dependencias, 15 comprobaciones de motivos y políticas de
revisión, 11 del recibo DLTX2, 11 de observaciones DLOB1 y un vector de
compatibilidad de los bytes DLRV1, DLST1 y DLTX1. Los fallos iniciales por las
APIs ausentes precedieron a cada implementación nueva.

Los codecs se contrastaron con vectores independientes, límites máximos,
truncados, versiones, textos canónicos y referencias de ámbito incorrecto.
El vector legado conserva el formato anterior con un hasher determinista de
prueba; no constituye una nueva prueba del algoritmo SHA-256. Las dos pruebas
de codecs compilaron en 2.76 s. El formato del workspace y Clippy 1.98.1 sobre
los cinco targets aprobaron. Clippy detectó inicialmente una copia redundante
en una prueba: se eliminó y se repitieron el control y los 11 casos de
observaciones, todos aprobados; esa repetición no aumenta los 52 casos únicos.
Los 13 archivos nuevos de código y pruebas son ASCII y menores de 400 líneas.

Este corte previo comprendía primitivas locales sin conexión a persistencia,
trabajador, HTTP o Qadra. En ese momento no se había repetido la suite completa
sobre el código nuevo. La campaña del núcleo V2 se registra por separado arriba;
los resultados históricos de las entregas integradas que siguen no se le atribuyen.

## Interfaz de plazos y responsables: 18 de septiembre de 2026

La [PR 32](https://github.com/eddndev/TT2026-B136/pull/32) se integró mediante
squash a las **13:27:15 de Ciudad de México**, commit
`ae205d2e5cbe7bcbb9e47b25248fdeb0f9239e52`. Se comprobaron **19 controles
aprobados y un release omitido** conforme al evento. El árbol integrado coincide
con el de la cabeza verificada. La cobertura remota de ese corte fue dominio
**5070/5192 (97 %)**, aplicación **11807/12268 (96 %)** e infraestructura
**20210/21782 (92 %)**; los tres umbrales aprobaron. Estas cifras corresponden
al backend integrado, no a la interfaz añadida después ni al push posterior a
`main`.

La ampliación actual incorpora el selector autorizado de responsables y
[Qadra de plazos](../web/README.md#plazos-del-expediente). Se ejecutaron los
controles siguientes sobre el nuevo código, con Rust 1.94, Clippy 1.98.1,
Node 22, qpdf 12.4.1 y PostgreSQL/Redis desechables. Ningún adaptador se omitió
por falta de variables.

| Comprobación | Resultado fresco |
| --- | --- |
| Formato y compilación Rust | Aprobados; compilación 21.134 s con caché. |
| Suite Rust completa | **2238 aprobadas, 0 fallidas, 1 TSA externa ignorada**, 697.753 s. |
| Clippy, todos los targets | Aprobado con warnings denegados, 28.920 s. |
| Rust 1.88, todos los targets | Aprobado, 28.967 s. |
| Pruebas unitarias web | **232 aprobadas**, 2.495 s del comando. |
| Formato y compilación web | Aprobados; compilación inicial 3.988 s, repetida tras actualizar la guía. |
| API y restauración reales | Aprobadas, 170.800 s. |

Los **15 casos nuevos Rust** están incluidos en el total: seis del servicio,
cuatro HTTP con puertos controlados, cuatro PostgreSQL y uno concurrente que
recorre cuatro formas de revocación. Comprueban Owner sin membresía, personal
asignado, exclusión de Client/cuentas inactivas, paginación filtrada, cursores
estrictos, cierre, reautenticación y auditoría antes de devolver candidatos.
El caso concurrente comprueba también una conexión cuyo aislamiento por defecto
es más fuerte que el de las transacciones auditadas.

El recorrido API agregó la selección real de responsables, ausencia de
credenciales en su proyección y desaparición después de revocar membresía.
Repitió los flujos de días, meses y horas y conservó **14 respuestas exactas**
de plazos después de restaurar, junto con la comparación de estado y auditoría.
La CLI no cambió; su última demostración permanece identificada en el corte
histórico siguiente y no se registra como una ejecución nueva.

La primera regresión web completa aprobó **281 de 283 escenarios**; dos
agotaron cinco segundos al esperar la pantalla inicial durante la compilación
en frío. Ambas capturas ya mostraban el acceso al finalizar el diagnóstico.
Se acotó a veinte segundos únicamente la espera de hidratación inicial del
arnés, sin aumentar tiempos de operaciones ni añadir reintentos. La repetición
completa aprobó **283 de 283 escenarios** en **246.123 s**; incluye los 28 nuevos
de plazos. El formato final aprobó en 6.743 s.

`scripts/web-demo.sh` aprobó **23 escenarios con servicios reales** en
**438.490 s**, incluida la preparación; Playwright informó 4.2 minutos.
Los tres nuevos escenarios recorren días, meses y horas con fuentes/calendario
seleccionados R1 y cabezas R2; mes sin homólogo; cantidad ausente corregida a
24 sin adoptar el máximo de 72; atención, retiro e historia exacta. Comprueban
los cuatro roles, expedientes ajenos, cierre y revocación. Una corrección
competidora conserva el borrador; un 201 confirmado en el servidor cuya respuesta
se pierde se concilia por revisión y recibo, con una sola escritura.
Los veinte escenarios anteriores también aprobaron. Tras ajustar únicamente el
restablecimiento de foco y desplazamiento previo a las dos capturas de plazos,
se repitió ese escenario contra servicios aislados: **1/1 aprobado**, 153.958 s
incluida la preparación y 12.7 s informados por Playwright. Esa repetición no
incrementa los 23 casos únicos. Las capturas finales de escritorio (1440 px)
y móvil (390 px) se inspeccionaron sin recortes ni desbordamiento horizontal de
la página; el enlace de salto permanece fuera del área visible mientras no
recibe foco.

Las comprobaciones focales detectaron y corrigieron columnas comprimidas en
móvil y la ausencia del calendario/referencias de los ejemplos de un perfil.
La interfaz conserva la evaluación capturada y sus revisiones exactas; no
implementa un segundo evaluador. Las capturas de escritorio y móvil permiten
revisar legibilidad, sin sustituir una prueba formal de usabilidad.

El reporte de **306 páginas** tiene compilación y revisión visual propias en
[la verificación académica](academic-report-verification.md). La reevaluación
durable, activación automática, alertas, agenda conjunta y corpus jurídico
aplicable siguen pendientes. Las duraciones anteriores miden comandos de
verificación, no latencia ni rendimiento del producto.

La [PR 33](https://github.com/eddndev/TT2026-B136/pull/33) se integró mediante
squash el mismo día a las **15:09:02 de Ciudad de México**, commit
`214202a1ddfc37ffc2bbf47706ab88b164d091c3`, después de **19 comprobaciones
aprobadas y una publicación de release omitida** conforme al evento. El árbol
`09ee772ce4e03e5cdb86ed6ac1e38322468ae5fe` coincide exactamente con el de la
cabeza probada `eac8d0b42ca69988e5bc93a26a1e35df181a46f4`.
La [medición remota](https://github.com/eddndev/TT2026-B136/actions/runs/35392792002/job/105754739607)
registró dominio **5070/5192 (97 %)**, aplicación **11862/12323 (96 %)** e
infraestructura **20273/21845 (92 %)**; los tres umbrales de 90 % aprobaron.
El ejecutable registró **913/1286 (70 %)**, informado sin umbral en esa campaña.
Se sincronizaron las referencias locales de `main` y se conservaron los cambios
académicos ajenos. Estas comprobaciones corresponden a la PR; las ejecuciones
posteriores al push de `main` tienen resultados independientes.

## Integración de la base publicada

El 16 de septiembre de 2026 a las 19:42 (`America/Mexico_City`) se confirmó
la integración en `main` del código publicado hasta `c51b49c`. El commit
resultante, `2d552fd`, conserva exactamente su árbol de fuentes. Se verificaron
19 controles de CI aprobados y un paso de publicación de release omitido
por el tipo de evento antes de integrar. Incluyen pruebas Rust, cobertura,
verificación web, navegador real y compilación del reporte.

Esta comprobación corresponde a la base previa a los insumos autorizados.
El cambio adicional se trasladó sobre esa base sin alterar su árbol; su
campaña local se detalla a continuación y requiere su propia CI. La integración
no cierra los flujos pendientes de perfiles, plazos operativos ni alertas.
Los cortes históricos conservan sus fechas, métricas y límites originales.

El mismo día a las 20:07 (`America/Mexico_City`) se integró también el corte de
insumos temporales mediante el commit `1e75b41`. Sus 19 controles de CI aprobaron
y el paso de release fue omitido conforme al evento. La consulta posterior a
GitHub y la sincronización de `origin/main` confirmaron ese commit. El trabajo
nuevo de perfiles descrito en el contrato conserva verificación e integración
propias; esas comprobaciones remotas anteriores no lo cubren.

La consulta remota del 17 de septiembre confirmó también la integración del
catálogo mediante la [PR 31](https://github.com/eddndev/TT2026-B136/pull/31):
`84af1be3717df4d3257e4d4e6b9dda14084aa713`, integrado el 16 de septiembre a las
22:40:14 (`America/Mexico_City`). Ese commit sigue siendo la punta remota al
iniciar la comprobación del registro persistente. Las modificaciones posteriores
se verifican por separado y no forman parte de esa integración.

## Corte reproducido: registro persistente de plazos

Fecha local: 17 de septiembre de 2026. El bloque incorpora
[registro, atención e historia](deadline-records.md), persistencia PostgreSQL
0017 y [contrato HTTP](deadlines-api.md). Conserva los resultados almacenados y
sus fuentes exactas; no reevalúa una revisión al leerla.

La suite completa de **2159 aprobadas, cero fallidas y una TSA externa ignorada**
corresponde al corte previo al adaptador PostgreSQL y a la API de este bloque.
No acredita esas incorporaciones. La nueva campaña completa aprobó **2223 pruebas,
cero fallidas y una TSA externa ignorada**, en **663.95 s**, mediante
`scripts/test-backends.sh cargo test --workspace` con PostgreSQL/Redis aislados y
qpdf 12.4.1. Rust local fue 1.94.0. Los adaptadores no se omitieron por ausencia de
variables. Las cifras focales siguientes están incluidas en el total y no se
suman nuevamente:

- 31 casos del adaptador, revalidación, concurrencia, almacenamiento, proyección
  SQL, esquema e importación aprobados después de corregir sus primeros fallos.
- Cuatro casos adicionales de persistencia mensual y horaria, parentesco exacto
  de notificaciones, acuerdos y calendarios históricos aprobados.
- Restauración real con `pg_dump`/`pg_restore` aprobada, seguida de siete casos de
  esquema repetidos. Las repeticiones no aumentan el conteo de casos únicos.
- 17 pruebas HTTP con puertos controlados, seis proyecciones del resultado y
  dos comprobaciones de traducción de errores aprobadas.

La primera restauración detectó que PostgreSQL aplanaba cuatro conjunciones
originadas en `BETWEEN`, causando un rechazo incorrecto de un esquema equivalente.
La migración expresa ahora los límites con comparaciones explícitas. La
comprobación sigue contrastando expresiones exactas; el test compara el catálogo
antes y después de restaurar y conserva los rechazos de modificaciones reales.
La importación inicial rechaza también una raíz parcial de plazo sin auditoría.

`scripts/api-demo.sh` aprobó en **163.95 s**, incluida la compilación del workspace
en **17.35 s** con caché. El recorrido registra plazos diarios, mensuales y horarios,
contrasta cuatro roles, aislamiento, preparación, conflictos, atención, retiro y
revocación. Después de `pg_dump`/`pg_restore` recupera **14 respuestas exactas** de
plazos, además de repetir los módulos anteriores. La comparación completa de
estado incluye las dos tablas nuevas, auditoría y recibo de importación; la
verificación de evidencia documental y el ZIP histórico permanecen válidos.

Clippy **1.98.1** para todos los targets aprobó con warnings denegados en **5.61 s**.
Los primeros intentos detectaron cuatro patrones de estilo en fixtures y un
bloqueo conservado a través de `await`; se corrigieron sin suprimir advertencias.
Estas duraciones son de comandos de verificación, no latencias del producto.
El PDF de **305 páginas** tiene compilación y revisión visual aprobadas,
documentadas en [la verificación académica](academic-report-verification.md).
El formato del workspace y la compilación con Rust **1.88**, todos los targets,
aprobaron; esta última tomó **28.45 s**. `scripts/demo.sh` aprobó en **6.45 s**.
`scripts/web-demo.sh` aprobó **20 pruebas de navegador con servicios reales**
en **362.00 s**, incluida la preparación de datos. Esta campaña comprueba la
regresión de los flujos Qadra existentes; todavía no existe una interfaz de
plazos que pueda acreditarse con ella. La integración remota requiere sus
propios controles. Qadra de plazos, agenda conjunta,
reevaluación durable y alertas permanecen pendientes. Ninguna de estas cifras
acredita un corpus normativo aprobado ni el cierre de todos los casos de uso.

## Corte reproducido: catálogo de perfiles y evaluación explícita

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El
[catálogo](deadline-profiles-api.md) publica, reemplaza y retira configuraciones
versionadas globales o privadas. Conserva definición, algoritmo, ejemplos,
recibos e historia inmutable con autorización y auditoría atómicas. El evaluador
puro comprueba integridad, aplicabilidad, cantidad, precisión y corte; conserva
candidatas y bloqueos. Las evaluaciones persistentes y el seguimiento operativo
siguen pendientes según [el contrato de ciclo de vida](deadline-lifecycle.md).

| Comprobación | Resultado fresco |
| --- | --- |
| Suite Rust con PostgreSQL/Redis desechables | **2057 aprobadas, 0 fallidas, 1 TSA externa ignorada**, 1174.029 s. |
| Casos nuevos | **179 aprobados**, ya incluidos en el total. |
| Formato del workspace | Aprobado, 4.284 s. |
| Compilación del workspace | Aprobada, 7.834 s con caché. |
| Clippy 1.98.1, todos los targets | Aprobado con warnings denegados, 11.149 s. |
| Rust 1.88, todos los targets | Aprobado, 39.946 s. |
| `scripts/demo.sh` | Aprobado, 16.347 s. |
| `scripts/api-demo.sh`, servicios y restauración reales | Aprobado, 325.807 s. |

Rust local fue 1.94.0. La suite ejecutó `scripts/test-backends.sh cargo test
--workspace` con qpdf 12.4.1 y bases aisladas; no omitió adaptadores por falta de
variables. Las duraciones incluyen compilación y preparación cuando corresponden;
no son mediciones de latencia de producto. No cambiaron dependencias.

Los 179 casos nuevos comprenden once de dominio, 112 de aplicación, cuarenta de
infraestructura y dieciséis de HTTP con puertos controlados. Cubren reglas fijas
y cantidades ordenadas, cortes, corpus, codificación estricta, extracción
comprobada, bloqueos, ámbito privado, permisos, CAS, revocación/cierre entre
preparación y confirmación, inventario, eventos durables, rollback y restauración.
La falta de cantidad no se sustituye por un máximo. Un bloqueo de aplicabilidad
no oculta corrupción. El registro de eventos no tiene todavía consumidor.

El recorrido HTTP real comprueba cuatro roles, aislamiento global/privado,
recibos, corpus sintético, conflicto secuencial, retiro y revocación de membresía.
La restauración recuperó **diez respuestas exactas** del catálogo y sus recibos,
además de repetir los recorridos existentes, verificar auditoría y conservar el
ZIP de evidencia. Se compararon los estados de base antes de reconciliar y
antes/después de restaurar, incluidos perfiles, eventos y posición lógica de su
secuencia. El estado físico de caché de la secuencia no forma parte de esa
comparación. Este guion no prueba por sí solo carreras concurrentes de perfiles
ni cierre; esos escenarios pertenecen a los tests del adaptador.

TDD registró módulos y APIs ausentes antes de implementar. Las regresiones
adicionales demostraron el rechazo de herencia de tablas en el esquema y de una
importación inicial sobre catálogos o eventos ocupados. Los reintentos con recibo
previo siguen conciliándose. Las repeticiones focales no aumentan el total.

La primera campaña HTTP terminó después de capturar perfiles, sin diagnóstico.
La repetición instrumentada confirmó que el estado restaurado y el recibo eran
idénticos y localizó el fallo al esperar la dirección del servidor restaurado.
El guion anterior esperaba aproximadamente diez segundos. Se añadió diagnóstico
sin comandos ni credenciales y una espera acotada a sesenta segundos. En la
repetición aprobada el servidor restaurado anunció su dirección a los **12
segundos**, tras validar el inventario; se mantuvieron todas las aserciones de
datos, evidencia y HTTP. No se modificó código Rust para resolver ese fallo.

La CI inicial detectó un fallo previo a la suite Rust en la preparación de las
pruebas SQL independientes de calendarios. Su fixture aplicaba todas las
migraciones salvo las de calendario y después creaba esas tablas; los eventos
nuevos ya dependen de ellas. El fallo se reprodujo localmente antes de corregir
la fixture para instalar sólo prerrequisitos anteriores y después el calendario.
Se conservaron las migraciones de producto y todas las aserciones. Aprobaron
**21 pruebas SQL** en 14.603 s, **nueve del generador** en 0.389 s y **nueve de
preparación de formatos** en 0.382 s. La comprobación de vectores también aprobó
en 0.296 s. Son pruebas existentes, separadas de las 2057 de Rust.

Las **1441** fuentes, fixtures, scripts y manifiestos controlados conservan
**1438** huellas desde la suite Rust. Los dos guiones HTTP modificados después
se comprobaron con la restauración final; la tercera diferencia es la fixture
SQL, verificada con sus 21 casos. El código Rust y las migraciones permanecen
idénticos. Las 153 fuentes de código modificadas son ASCII y menores de 400
líneas, máximo 370. La revisión estática independiente no dejó hallazgos
accionables. La corrección de la fixture requiere su propia CI remota.

No se implementó una pantalla Qadra en este corte ni se repitió localmente una
campaña de navegador o cobertura instrumentada. No se atribuye a estas pruebas
la aceptación jurídica de perfiles, la persistencia de plazos, la entrega de
alertas ni el cierre del objetivo completo. La comprobación del PDF de 300
páginas y la conservación de autoría se registran en
[la verificación académica](academic-report-verification.md).

## Corte reproducido: insumos temporales autorizados

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El
[servicio de insumos](deadline-inputs.md) autentica, resuelve y comprueba fuentes
exactas y cabezas observadas antes de calcular. PostgreSQL carga el conjunto
bajo autorización y auditoría comunes. La preparación queda en memoria;
los perfiles, evaluaciones persistentes, reevaluación y alertas siguen pendientes.

| Comprobación | Resultado fresco |
| --- | --- |
| Suite Rust con PostgreSQL/Redis desechables | **1878 aprobadas, 0 fallidas, 1 TSA externa ignorada**, 545.103 s. |
| Casos nuevos de insumos | **50 aprobados**, incluidos en la suite global. |
| Formato del workspace | Aprobado, 1.539 s. |
| Compilación del workspace | Aprobada, 0.282 s con caché; compilación inicial aprobada en 18.180 s. |
| Clippy 1.98.1, todos los targets | Aprobado con warnings denegados, 14.052 s. |
| Rust 1.88, todos los targets | Aprobado, 19.084 s. |
| Conservación de fuentes durante campaña final | **1110** fuentes, fixtures, scripts y manifiestos intactos. |

Rust local fue 1.94.0. La suite usó `scripts/test-backends.sh cargo test --workspace`,
qpdf 12.4.1 y bases desechables; los adaptadores no quedaron omitidos por ausencia
de variables. No cambiaron dependencias, esquema ni cánones existentes.

Los casos nuevos son uno de permiso, trece de material, trece de cabezas, diez
de servicio, ocho de backend y cinco de acceso PostgreSQL. Cubren las tres
familias, revisiones exactas y retiradas, padre fijo con revisión variable,
acuerdo UUID cero eliminado de una cabeza posterior, calendarios modificados,
proyecciones discordantes, recibos corruptos, R0, cierre y permisos revocados.
Los rechazos internos del puerto conservan la auditoría previa; cada lectura
aceptada añade un evento. El rechazo de la segunda autenticación del servicio
es posterior al commit del puerto y no revierte esa lectura auditada.

TDD capturó módulo, permiso y adaptador ausentes antes de implementar. El primer
focal aprobó doce casos de material y diez de servicio; los trece de cabezas y
trece de PostgreSQL aprobaron después. El último caso de material entró en la
campaña global. Esas repeticiones no se suman como pruebas nuevas. La primera
campaña se detuvo en Clippy por un atributo `allow(dead_code)` duplicado en
un módulo auxiliar; se retiró el atributo externo del test y se ejecutó la
campaña completa sobre las fuentes corregidas, sin modificar producción.

Las 28 fuentes Rust propias son ASCII y menores de 400 líneas, máximo 314.
Las revisiones independientes de aplicación, PostgreSQL y contratos no encontraron
hallazgos accionables. No se añadieron rutas HTTP ni composición CLI: no se
repitieron localmente recorridos HTTP/CLI/navegador ni cobertura instrumentada.
Las pruebas de restauración existentes de la suite no equivalen a una nueva
campaña HTTP del módulo. La CI del corte fuente c51b49c terminó con 19 controles
aprobados y un paso de release omitido; no verifica estos cambios posteriores.

La comprobación del manuscrito y PDF se registra en
[la verificación académica](academic-report-verification.md).

## Corte reproducido: extraccion de fuentes temporales exactas

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El modulo
[deadline_triggers](deadline-triggers.md) verifica coherencia entre seleccion y
material de resolucion, notificacion o resultado de audiencia, conserva el campo
y su precision, y coordina ese tiempo con la aritmetica. Distingue errores de
integridad, campos ausentes, desconocimiento y calificacion temporal declarada.
No implementa perfiles normativos ni evaluaciones persistentes.

| Comprobacion | Resultado fresco |
| --- | --- |
| Suite Rust con PostgreSQL y Redis desechables | **1828 aprobadas, 0 fallidas, 1 externa de TSA ignorada**, 494.638 s. |
| Pruebas nuevas de extraccion y coordinacion | **45 aprobadas**, incluidas en el total global. |
| Formato del workspace | Aprobado, 1.468 s. |
| Compilacion del workspace | Aprobada, 14.238 s. |
| Clippy 1.98.1, workspace y todos los targets | Aprobado con warnings denegados, 19.131 s. |
| Rust 1.88, workspace y todos los targets | Aprobado, 17.104 s. |
| Conservacion durante la campana | **1086** fuentes, fixtures, scripts y manifiestos sin cambios. |

La compilacion y las pruebas usaron Rust 1.94.0. La suite completa se ejecuto
mediante `scripts/test-backends.sh cargo test --workspace`, con qpdf 12.4.1 y
PostgreSQL/Redis aislados; no se atribuye cobertura de adaptadores a una ejecucion
sin sus variables de entorno. No cambiaron dependencias ni canones existentes.

Las pruebas nuevas comprenden un caso de contrato, once de campos, once de
integridad, diez de resultados de audiencia, seis de calificacion y seis de
coordinacion. Incluyen fuentes ajenas, padres discordantes, revisiones y acuerdos
exactos, UUID cero, None frente a Unknown, preservacion de copias, precision y
desfase, candidata UTC literal y calendario independiente de la extraccion.
El estado Concluded no aporta por si solo un tiempo de final de audiencia.

TDD capturo el modulo ausente en los targets de contrato e integridad, y despues
en campos y calificacion, antes de implementar la extraccion. El primer focal
aprobo 29 casos; el focal completo aprobo 45. Las repeticiones no se suman al
total. Los seis targets entraron en la suite global sobre las fuentes finales.
Dos revisiones independientes del codigo no encontraron defectos accionables.
Las fuentes Rust nuevas son ASCII y menores de 400 lineas, maximo 349 tras formato.

La base de aritmetica obtuvo 18/18 checks remotos aprobados en el commit
269232d; esa comprobacion no sustituye la CI de la nueva entrega. No se repitieron
localmente cobertura instrumentada, release, CLI, HTTP ni navegador para este
cambio de dominio. La restauracion cubierta por la suite de backends es distinta
de una nueva campana HTTP de restauracion.

Los apartados academicos propios y su comprobacion de PDF se documentan en
[la verificacion academica](academic-report-verification.md). La vinculacion de
fuentes verificadas mediante aplicacion, los perfiles, la persistencia,
reevaluacion, alertas y Qadra de plazos siguen pendientes.

## Corte reproducido: aritmetica temporal de plazos

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El dominio agrega
[aritmetica explicita](deadline-arithmetic.md) de dias naturales o computables,
meses civiles y horas transcurridas. Reutiliza el conteo de calendario y conserva
regla, declaracion original, candidata intermedia, fuentes y bloqueos. La
[decision de arquitectura](adr/0032-explicit-deadline-arithmetic.md) separa esta
operacion de perfiles normativos y plazos operativos.

| Comprobacion | Resultado fresco |
| --- | --- |
| Suite Rust con PostgreSQL y Redis desechables | **1783 aprobadas, 0 fallidas, 1 externa de TSA ignorada**, 431.740 s. |
| Pruebas nuevas de aritmetica | **42 aprobadas** en cinco targets, ya incluidas en el total. |
| Formato del workspace | Aprobado, 1.326 s. |
| Compilacion del workspace | Aprobada, 10.736 s. |
| Clippy 1.98.1, workspace y todos los targets | Aprobado con warnings denegados, 15.213 s. |
| Rust 1.88, workspace y todos los targets | Aprobado, 14.124 s. |
| Conservacion durante la campana | **1071** fuentes, fixtures, scripts y manifiestos controlados sin cambios. |

La compilacion y la suite usaron el toolchain local Rust 1.94.0; Clippy y la
comprobacion de MSRV usaron las versiones expresas indicadas. El guion configura
las bases aisladas y Redis: esta campana no depende de una ejecucion sin las
variables que habilitan las pruebas de adaptadores.

Los cinco targets contienen 12 casos civiles, ocho de calendario, cinco de
ajuste final, doce horarios y cinco de contrato. Verifican inclusion explicita,
duracion positiva, homologo mensual sin ajuste silencioso, bisiestos, anos
limite, cantidades maximas, conservacion del desfase declarado, falta de
precision, excepciones, fuentes, cobertura y candidata previa a un ajuste
bloqueado. El resultado horario se calcula en UTC; una fecha civil no adquiere
hora por disponer de desfase.

TDD capturo fallos de importacion del modulo inexistente antes de implementarlo.
La primera ejecucion focal aprobo los 42 casos. Despues se separaron los cinco
casos de ajuste final para respetar el limite de archivo y se ejecuto la suite
completa sobre esas fuentes finales. No se suman las repeticiones focales.
Las diez fuentes Rust cambiadas son ASCII y menores de 400 lineas, maximo 328.
No cambiaron dependencias ni canones de hechos o calendarios. Dos revisiones
independientes del contrato y el codigo no encontraron defectos accionables.

La [investigacion normativa](deadline-rule-research.md) mantiene por separado
los extremos mensuales y de aplicabilidad aun no cerrados. Los vectores son
matematicos sinteticos; no acreditan una interpretacion juridica universal.
Faltan calificacion estructurada, perfiles respaldados, fuentes exactas resueltas
por el servicio, persistencia, reevaluacion, alertas y Qadra para plazos.

Esta entrega no cambia HTTP, CLI ni interfaz. No se repitieron localmente los
guiones de demostracion, recorridos web, cobertura instrumentada, binario release
o campana de restauracion HTTP. Los resultados web del corte anterior conservan
su procedencia. La correccion de navegacion de ese corte tiene 18/18 checks CI
aprobados en su commit b1a8b58; esa CI no prueba la nueva aritmetica. La
actualizacion del manuscrito y su PDF sigue pendiente, como registra
[la verificacion academica](academic-report-verification.md).

## Corrección de sincronización de una prueba de navegación

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). Una ejecución de
CI del cliente terminó con 254 pruebas aprobadas y un fallo en la navegación
restringida del rol Client; la ejecución paralela de la misma suite aprobó las
255. La prueba cambiaba el hash antes de esperar que terminara la consulta que
abre el expediente. La comprobación de ausencia del enlace podía aprobarse
todavía en la lista y competir con esa apertura pendiente.

La prueba ahora espera el encabezado del resumen antes de comprobar que no hay
enlace de etapas y de intentar esa ruta. Conserva las aserciones de regreso al
tablero y ausencia de solicitudes protegidas; no cambia la aplicación ni amplía
los tiempos de espera. La prueba corregida aprobó **20 repeticiones con dos
workers, 19.7 s**, y su formato fue validado. Son repeticiones de un escenario,
no veinte escenarios nuevos. No se repitieron las suites completas locales ni
los servicios reales para este cambio exclusivo de sincronización de pruebas.

## Corte reproducido: resoluciones y notificaciones en Qadra

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El cliente de
[hechos declarados](procedural-facts-api.md#cliente-qadra) agrega navegación por
expediente, formularios explícitos, selección histórica, preparación y
confirmación, corrección, retiro y conciliación del recibo exacto. Reutiliza la
marca, tipografía y controles de Qadra. Los plazos y alertas siguen pendientes.

| Comprobación | Resultado fresco |
| --- | --- |
| Suite unitaria del cliente | **195 aprobadas, 0 fallidas**; incluye 27 de hechos y una de navegación nuevas. |
| Formato y compilación web | Aprobados; 3.868 s y 2.119 s en la campaña integrada. |
| Navegador con HTTP simulado | **255 aprobadas, 0 fallidas**, 203.410 s del comando; incluye 32 nuevas. |
| Navegador con servicios reales aislados | **20 aprobadas, 0 fallidas**, 316.943 s del guion completo; incluye tres nuevas. |
| Regresión del selector histórico | Rojo reproducido y corrección aprobada; incluida en las 32 nuevas. |
| Inspección visual | Detalle y formulario a 1440 y 390 px; sin desbordamiento horizontal. |
| Conservación durante la campaña integrada | **1432** archivos controlados sin cambios. |

Las pruebas nuevas ya están incluidas en los totales, no se suman otra vez.
La suite de cliente contrasta los 28 vectores de valores del dominio, sin
modificarlos. Conserva precisión temporal y desfase opcional, distingue datos
desconocidos de campos ausentes, valida fuentes exactas y coteja identidad,
revisión, actor, acción, operación, valores y recibos. No calcula cánones
criptográficos en JavaScript ni transforma la administración observada en CAS.

Los escenarios simulados cubren ambas familias, sus tres acciones, listas e
historia paginadas, roles, cierre, revocación y respuestas tardías. Recorren
selección de participantes archivados, resultados retirados, acuerdo con UUID
cero, revisiones antiguas del mismo padre y dos soportes directos. Una respuesta
perdida conserva el comando: consultar una revisión temporalmente ausente no
habilita reenvío, y un recibo distinto no confirma la escritura. Se conserva el
segundo explícito cero y se retiran componentes que exceden la precisión elegida.

La campaña real usa PostgreSQL, Redis, CA interna, TSA local y puertos
desechables. Los tres escenarios nuevos comprueban correcciones competidas,
conservación del borrador, respuesta perdida después de un commit real,
conciliación sin otro envío, cuatro roles, cierre, revocación y limpieza de datos
privados. Una notificación conserva padre y resultado retirados, participante
archivado y dos versiones PDF/DOCX exactas, aun existiendo una versión documental
posterior. Las correcciones y retiros mantienen la revisión original consultable.

TDD conserva los fallos iniciales por navegación y módulos ausentes. La revisión
adicional reprodujo un fallo real: después de consultar R1, un error al leer R2
dejaba R1 seleccionable sin rotular claramente la revisión. El selector ahora
retira la selección anterior al iniciar otra consulta y muestra número y estado
antes de vincular. Los fallos iniciales de selectores de prueba, textos simulados
sin normalizar y páginas ficticias incompletas se corrigieron por separado;
no se relajaron validadores ni contratos para hacer pasar esos datos.

Tras la campaña se alinearon dos etiquetas opcionales con la clase `checkbox`
ya existente de Qadra. Se repitieron las dos comprobaciones de diseño, la
compilación y el formato para ese ajuste visual; no se suman a los totales.
Las 62 fuentes de código cambiadas son ASCII y menores de 400 líneas (máximo
304). No cambiaron dependencias ni código Rust. La compilación conserva el aviso
de tamaño de un chunk web mayor de 500 kB; no se midió rendimiento en esta entrega.

Los 1741 resultados Rust, 27 respuestas restauradas y comprobaciones MSRV/Clippy
del corte HTTP siguiente son evidencia anterior, no una repetición de esta
entrega. La CI de esa API se consultó con 18/18 comprobaciones aprobadas; no
sustituye la CI del cliente nuevo. No se repitieron cobertura instrumentada,
restauración, release ni CLI locales. El manuscrito, su PDF y la usabilidad con
personas reales siguen pendientes; la inspección visual automatizada no acredita
esa evaluación. La demo estable permanece intacta.

## Corte reproducido: API de hechos declarados

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). La
[API de hechos](procedural-facts-api.md) conecta ambas familias al servicio y
almacenamiento auditado existentes. Agrega preparacion, alta, correccion,
retiro, consultas exactas, listados e historia con un presupuesto HTTP compartido.
La interfaz Qadra de estos hechos, plazos operativos y alertas siguen pendientes.

| Comprobacion | Resultado fresco |
| --- | --- |
| Formato y compilacion del workspace | Aprobados; 1.331 s y 11.992 s. |
| Suite con PostgreSQL/Redis desechables | **1741 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 343.456 s. |
| Clippy 1.98.1 del workspace, todos los targets, warnings denegados | Aprobado, 5.544 s. |
| Rust 1.88 del workspace, todos los targets | Aprobado, 5.170 s. |
| Pruebas nuevas incluidas | **54 aprobadas**: 3 de errores, 10 de entrada, 15 de proyeccion y 26 de rutas. |
| Guion HTTP con servicios aislados | Aprobado, 104.032 s; incluye ambas familias y restauracion. |
| Respuestas HTTP de hechos conservadas tras restaurar | **27** respuestas exactas, con fuentes historicas y recibos. |
| Conservacion durante la campana | **1061** huellas de fuentes, fixtures y manifiestos sin cambios. |

Las 54 pruebas nuevas estan incluidas en la suite global; no se suman de nuevo.
Los 28 vectores de valores existentes recorren el DTO y el limite HTTP sin
cambiar semantica o normalizacion. Se comprueban precision temporal, datos
desconocidos frente a campos opcionales, UUID cero, familia y padre fijos,
revision y accion esperadas, proyecciones exactas y errores publicos acotados.
El cuerpo completo se limita a 512 KiB, incluidos datos escapados y streaming;
los objetos rechazan claves extra, repetidas y representaciones como arreglos.

TDD conserva rojos por APIs ausentes y errores no mapeados. Las regresiones
conductuales reproducidas incluyen campos extra aceptados por variantes vacias
Serde, un envoltorio de objeto duplicado que rechazaba peticiones validas y el
estado HTTP de la revision cero en la ruta. Las variantes vacias ahora exigen
objetos estrictos; la lectura usa el DTO ya protegido y las rutas rechazan una
revision no positiva como entrada invalida. Los ajustes de expectativas para
revision inicial y version documental, y los avisos de Clippy sobre enums grandes
y clones de valores Copy, se registran aparte de esos fallos conductuales.

El guion real reproduce ambas familias y sus tres acciones, cuatro roles,
aislamiento entre expedientes, revocacion y cierre despues de preparar,
conflictos de revision y operacion, digest de envio alterado e historia terminal.
Una captura posterior a reapertura conserva la administracion vigente sin
convertirla en una condicion de revision del comando. Las fuentes incluyen
ficha manual archivada, resultado de audiencia retirado, padre retirado o una
revision anterior exacta y soportes PDF/DOCX. Los lotes directos de cero, una y
dos versiones no se amplian con el soporte del padre. La captura R0 importada tambien se reprodujo: conserva metadatos sin fabricar revision, digest, autor o fecha.

La importacion reconciliada conserva ambas tablas y `pg_dump`/`pg_restore`
preservan respuestas exactas, valores, fuentes, autores y recibos. La auditoria
se verifica antes y despues de restaurar. El guion usa PostgreSQL, Redis, PKI y
TSA local desechables; no modifica el runtime estable ni consulta un PSC externo.

Las 41 fuentes de codigo cambiadas son ASCII y menores de 400 lineas; el maximo
es 365. No cambian dependencias ni canones de valores, fuentes o recibos.
No se repitieron navegador, cobertura instrumentada o release locales en esta
entrega. Tampoco se modifico el manuscrito ni se compilo un PDF. La CI de la
persistencia anterior, PR24, se consulto con 18/18 comprobaciones aprobadas;
es evidencia de esa rama, no sustituye la CI de la nueva API.

## Corte reproducido: persistencia auditada de hechos declarados

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El
[adaptador PostgreSQL](procedural-facts-persistence.md) conserva resoluciones y
notificaciones con fuentes historicas exactas, permisos por expediente, recibos
y auditoria atomica. Incluye migracion, parsers SQL/Rust, catalogo, inventario,
importacion y restauracion. HTTP y Qadra de estos hechos siguen pendientes.

| Comprobacion | Resultado fresco |
| --- | --- |
| Formato y compilacion del workspace | Aprobados; 1.269 s y 7.011 s. |
| Suite con PostgreSQL/Redis desechables | **1687 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 408.982 s. |
| Clippy 1.98.1 del workspace, todos los targets, warnings denegados | Aprobado, 3.290 s. |
| Pruebas nuevas incluidas | **74 aprobadas**: 73 en 16 targets de infraestructura y una unitaria de catalogo. |
| Rust 1.88 del workspace, todos los targets | Aprobado, 35.435 s. |
| Guion HTTP con servicios aislados | Aprobado, 90.653 s; verifica los flujos existentes y compatibilidad de migracion/arranque. |
| Conservacion durante la campana | 1025 huellas sin cambios. |
| Corpus de fuentes y recibos | 43 vectores reproducibles; bytes, digests y longitudes anteriores conservados. |

Las 74 pruebas focales ya estan incluidas en la suite global, no se suman de
nuevo. Cubren las dos familias, R0 historico y revisiones administrativas,
permisos/asignacion, cierre y revocacion antes de confirmar, correcciones
competidas, operacion unica entre familias y Clock observado bajo el bloqueo.
Las fuentes directas se admiten como lote completo; los antecedentes historicos
no lo expanden. Se comprueban participantes manuales/tipificados, padres y
acuerdos exactos, incluidas proyecciones alteradas con digests recalculados.

Los listados prueban seleccion de cabeza antes del filtro, cursores exclusivos
con UUID cero, padre fijo e historia descendente. Un fallo de auditoria impide
confirmar cambios o devolver consultas. Importacion rechaza raices solas o
revisiones huerfanas de cualquiera de las familias. El inventario verifica todas
las paginas; las pruebas de pg_dump/pg_restore conservan bytes, fuentes, autor y
capturas despues de modificaciones administrativas y baja del autor.

TDD conserva fallos por APIs ausentes y regresiones conductuales de permisos,
catalogo, inventario, importacion y administracion historica. Esta ultima
permitia capturar una notificacion antes de la revision administrativa de su
resolucion, o una resolucion antes de la de su resultado de audiencia. Las
lecturas ahora rechazan ambas inconsistencias sin sustituir la historia por el
estado vigente. Tambien se corrigieron el nombre requerido de la restriccion
de operacion y la comparacion de cuerpos SQL para esquemas con signos de
porcentaje. La comprobacion SQL directa rechaza fuentes omitidas, sustituidas
o reescritas aun con recibos recalculados.

El corpus JSON de PFSRC1 tenia doce componentes temporales no codificados en
seis proyecciones de fecha/minuto. Se retiraron esos campos para coincidir con
la precision declarada. Ninguno de los 43 vectores cambia sus bytes canonicos,
SHA-256, longitud, nombre u orden. El generador Python y los parsers SQL/Rust
coinciden. Un error inicial de sintaxis de migracion, un fixture de reemplazo de
funcion y un aviso Clippy del helper de prueba se corrigieron por separado; no
se presentan como regresiones funcionales.

Las 61 fuentes nuevas Rust/SQL son ASCII y menores de 400 lineas, maximo 394.
No cambian dependencias ni los formatos criptograficos anteriores. La campana
usa PostgreSQL y Redis desechables; la unica prueba ignorada es la TSA externa.
El guion HTTP ejecuta las rutas existentes: no atribuye acceso web a los nuevos
hechos. No se repitieron navegador, cobertura instrumentada o release para esta
entrega, ni se modifico el manuscrito o compilo un PDF. Los resultados CI 18/18
de PR23 corresponden al servicio anterior; no acreditan esta persistencia.

## Corte reproducido: servicio de hechos declarados

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El
[servicio de aplicacion](procedural-facts-application.md) coordina fuentes
historicas exactas, autenticacion, admision del lote directo, reautenticacion,
preparacion, envio y lecturas. Sus [recibos PFSRC1/PFTXN1](procedural-facts-receipts.md)
vinculan valores, proyecciones, actor y comando. No existe todavia adaptador de
hechos, migracion, transaccion auditada, API o Qadra para estos recursos.

| Comprobacion | Resultado fresco |
| --- | --- |
| Formato y compilacion del workspace | Aprobados; 1.229 s y 12.602 s. |
| Suite con PostgreSQL/Redis desechables | **1613 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 417.677 s. |
| Clippy 1.98.1 del workspace, todos los targets, warnings denegados | Aprobado, 20.155 s. |
| Pruebas nuevas incluidas | **92 aprobadas**, todas en aplicacion. |
| Rust 1.88 de aplicacion y dominio, todos los targets | Aprobado, 10.753 s. |
| Conservacion durante la campana | 926 huellas sin cambios. |
| Corpus independiente | 43 vectores: 25 PFSRC1 y 18 PFTXN1; Python reproducible y bytes Rust coincidentes. |

Las 158 pruebas focales comprenden las 92 nuevas y 66 anteriores; no se suman
otra vez a la suite global. Los vectores forman parte de cinco pruebas nuevas.
Su generador Python no invoca Rust; el mock del puerto verifica los bytes y
devuelve el SHA-256 de Python. No se presenta ese mock como prueba del algoritmo.
Las cotas reproducidas son 19..36847 bytes para PFSRC1, 141..4145 para el envio
de resolucion y 157..4161 para el de notificacion.

TDD registra fallos iniciales por APIs ausentes. Se corrigieron dos regresiones
conductuales adicionales: dos acuerdos de una revision podian contener datos
comunes contradictorios (14 aprobadas/1 fallida), y cambiar el acuerdo podia
permitir sustituir su resultado historico aun con recibos recalculados validos
(12 aprobadas/1 fallida). El cierre incluye ambos negativos y el cambio de
acuerdo valido. Errores iniciales de fixtures y ajustes de Clippy se distinguen
de esas regresiones: no son fallos conductuales del producto.

Se comprobaron autenticacion previa, revocacion y cambio de actor o rol tras
admision, lote unico de dos versiones, deduplicacion sin perder funciones,
padre historico sin expansion documental, retiro sin readmision, fuentes
faltantes o sustituidas, cierre, revision obsoleta y recibos ajenos. Las lecturas
prueban alcance, padre fijo, revisiones exactas, limites, orden, filtros y
cursores, incluidas declaraciones retiradas y capturas administrativas cerradas.
El servicio no infiere efectos juridicos ni una politica de fechas futuras.

La suite global ejecuto los backends existentes mediante `scripts/test-backends.sh`
con PostgreSQL y Redis aislados. Para los hechos nuevos, los puertos son mocks:
estos resultados no prueban aun transaccion, membresia o rollback de un adaptador
inexistente. La prueba ignorada corresponde al proveedor TSA externo.

Las 30 fuentes nuevas Rust/Python son ASCII y menores de 400 lineas,
con maximo 389. Se conservaron 42 fuentes existentes de dominio y resultados.
Solo se agrega `serde_json` como dependencia de desarrollo ya presente en el
workspace, para leer el corpus. No hay nuevas dependencias productivas ni
migraciones. Sin nueva cobertura instrumentada, release, CLI, HTTP, navegador,
manuscrito o PDF. La evidencia de esta entrega es local en este corte; los CI
verificados de PR21 (16/16) y PR22 (18/18) corresponden a sus heads anteriores.

## Corte reproducido: comandos y fuentes exactas de hechos

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). La
[base de aplicacion](procedural-facts-application.md) agrega comandos de alta,
correccion y retiro, seleccion exacta y comprobaciones puras de identidad,
administracion y participantes. Define permisos por rol y puertos. No incluye
servicio coordinador, recibos canonicos, adaptador, migracion, HTTP o Qadra.

| Comprobacion | Resultado fresco |
| --- | --- |
| Formato y compilacion del workspace | Aprobados; 1.233 s y 15.512 s. |
| Suite con PostgreSQL/Redis desechables | **1521 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 398.030 s. |
| Clippy 1.98.1 del workspace, todos los targets, warnings denegados | Aprobado, 1.216 s. |
| Pruebas nuevas incluidas en la suite | **67 aprobadas**: 66 de aplicacion y una de permisos en dominio. |
| Rust 1.88 de aplicacion y dominio, todos los targets | Aprobado, 13.526 s. |
| Conservacion de fuentes durante el cierre | 896 huellas sin cambios. |

TDD registra fallos iniciales por API ausente en comandos, permisos, base,
seleccion, administracion y participantes. Una regresion adicional reprodujo
12 aprobadas y una fallida: la comprobacion aislada de estado aceptaba una
revision agotada. Ahora tambien exige sucesor valido; el cierre completo
incluye ese ajuste. La revision independiente amplio el positivo tipado para
distinguir digest de ficha y digest de sujeto. El primer verde focal de 14
pruebas precede esas aserciones; la tabla refleja las fuentes finales.

Los casos cubren cambio coordinado de padre en comando y valores, expediente
ajeno, familia distinta con UUID igual, revision obsoleta, retiro terminal,
selecciones con dos revisiones o acuerdos, digests documentales compartidos,
administracion sin revision y sin perfil, cierre, avance sin CAS y cambios de
una misma revision administrativa inmutable. Los participantes se resuelven por
revision exacta; se verifican sus canones y el sujeto ligado, incluidas fuentes
archivadas. Estos tests no prueban autorizacion efectiva de un nuevo adaptador:
esa integracion aun no existe. Los backends existentes si se ejecutaron con sus
bases y Redis aislados mediante `scripts/test-backends.sh`.

Los 20 archivos Rust nuevos son ASCII y menores de 400 lineas; el mayor tiene
367. Se conservaron los formatos y fuentes de dominio anteriores de etapas,
resultados, tiempo declarado y valores de hechos. Sin cambios de dependencias,
migraciones, CLI, HTTP o interfaz. No se repitieron cobertura instrumentada,
release o navegador; tampoco se actualizo el manuscrito ni se compilo un PDF.
La prueba ignorada sigue siendo la del proveedor TSA externo. Este corte tiene
evidencia local y no dispone de CI remoto para su entrega.

## Corte reproducido: valores de resolucion y notificacion

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El
[modelo puro](procedural-facts.md) conserva resoluciones y practicas de
notificacion separadas, seleccion de revisiones exactas, procedencia, tiempos,
funciones personales y soportes directos. Sus [canones](procedural-facts-canonical.md)
preservan cada declaracion sin inferir efectos juridicos ni sustituir fuentes
por su cabeza actual. Este corte no incluye aplicacion, persistencia o HTTP.

| Comprobacion | Resultado fresco |
| --- | --- |
| Formato y compilacion del workspace | Aprobados; 1.136 s y 5.619 s. |
| Suite normal con PostgreSQL/Redis desechables | **1454 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 369.749 s. |
| Clippy 1.98.1 del workspace, todos los targets, warnings denegados | Aprobado, 0.956 s. |
| Suite del dominio | **272 aprobadas**, incluidas **32 nuevas**, sin fallos ni ignoradas. |
| Corpus binario independiente | **28 vectores** coinciden byte por byte; incluido en dos de las pruebas Rust nuevas, no se suma al total. |
| Rust 1.88 del dominio, todos los targets | Aprobado, 1.770 s. |
| Conservacion de fuentes durante el cierre | 876 huellas sin cambios. |

TDD conserva el fallo inicial por API inexistente y fallos previos por ausencia
de canones y constantes. El primer intento de valores encontro tambien un error
del fixture: utilizaba un constructor de fecha inexistente. Se corrigio la prueba
para usar el parser existente antes de la corrida aprobada; no se cuenta ese
error como una falla conductual del modelo. Todos esos logs se conservan separados.
La primera suite completa tambien aprobo 1454/0/1; Clippy rechazo despues el
tamano de la variante de representacion. Su procedencia se movio a un Box sin
cambiar valores o canones y se ajusto una asignacion del test senalada por lint.
La tabla corresponde a la repeticion completa posterior a esos ajustes.

El generador Python no invoca Rust. Su salida versionada se reprodujo exactamente,
incluyendo textos con CRLF y Unicode sin composicion, desconocimiento, tiempos y
desfases, fuentes y evidencias por funcion. Las cotas alcanzadas son PFRES1
27..17700 bytes y PFNOT1 67..58671 bytes. Una version documental seleccionada
para dos funciones se incorpora una sola vez en el lote derivado, conservando ambos
localizadores; expectativas de digest contradictorias se rechazan. El dominio
reune referencias: no ejecuta admision, consulta fuentes ni verifica permisos.

Los 21 archivos nuevos de Rust, generador y fixtures son ASCII y menores de
400 lineas; el mayor tiene 357. Los 17 archivos anteriores de etapas y resultados
conservan sus bytes, y los demas dominios anteriores no cambiaron. No se modificaron
dependencias, migraciones o interfaz. Las revisiones independientes del modelo,
contrato y canones no encontraron hallazgos accionables.

No se repitieron localmente cobertura instrumentada, release, CLI, HTTP o navegador
para este modelo puro. Las cifras anteriores conservan su corte propio. Este
corte solo dispone de evidencia local; el CI remoto de entregas anteriores no
verifica estos valores nuevos. La prueba externa ignorada sigue correspondiendo
al proveedor TSA. Autorizacion, auditoria atomica,
persistencia, interfaz, integracion de PR y actualizacion academica siguen pendientes.

## Corte reproducido: precision temporal declarada

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El nuevo
[valor temporal](procedural-time.md) preserva tiempo desconocido, fecha,
minuto y segundo, con desfase opcional. Conservar una hora local sin zona no
fabrica un instante UTC; conservar un minuto no inventa segundos. Solo un
segundo con desfase explicito expone el instante de esa declaracion.

| Comprobacion | Resultado fresco |
| --- | --- |
| Formato y compilacion del workspace | Aprobados; 1.145 s y 14.658 s. |
| Suite normal con PostgreSQL/Redis desechables | **1422 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 382.788 s. |
| Clippy 1.98.1 del workspace, todos los targets, warnings denegados | Aprobado, 15.935 s. |
| Pruebas focales nuevas | **15 aprobadas**, 1.102 s; rojo conductual previo de 1 aprobada y 14 fallidas. |
| Suite del dominio | **240 aprobadas**, 0 fallidas e ignoradas, 2.462 s. |
| Rust 1.88 del dominio, todos los targets | Aprobado, 1.321 s. |
| Conservacion de fuentes durante el cierre completo | 855 huellas sin cambios. |

Las quince pruebas nuevas estan incluidas en las 1422 del workspace. Verifican
campos ausentes, precision e igualdad literal, UTC declarado distinto de ausencia,
horas/minutos/segundos invalidos, desfases de minuto hasta ambos extremos de
14 horas y fechas cuyo intervalo excederia los anios UTC 1..9999. Se conservan
byte por byte las 17 fuentes anteriores del dominio de etapas y resultados de
audiencia, incluidos sus canones y tipos temporales. No hubo migracion de datos.

La [decision de hechos declarados](adr/0031-declared-procedural-facts.md) fija
la separacion entre resoluciones y practicas de notificacion, pero su flujo
persistente, permisos especificos, API e interfaz siguen pendientes. El valor
temporal no introduce esas capacidades ni habilita computo juridico o alertas.

No se repitieron cobertura instrumentada, release, CLI, HTTP o navegador para
este valor puro. Las mediciones que siguen conservan su corte propio; el CI
remoto verifica el commit publicado y se registra por separado. La unica prueba
externa ignorada sigue siendo la del proveedor TSA.

## Corte reproducido: conteo civil de dias

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). El nuevo
[componente aritmetico](deadline-day-counting.md) cuenta clasificaciones de una
revision de calendario a partir de una primera fecha incluida y una cantidad
positiva. Conserva cada dia, su regla y fuentes, y el acumulado. Devuelve una
fecha candidata o un bloqueo explicito por clasificacion sin resolver, falta de
cobertura o agotamiento del ano 9999. No deriva efectos de notificacion ni
representa un vencimiento operativo; tampoco introduce API, persistencia o avisos.

| Comprobacion | Resultado fresco |
| --- | --- |
| Formato y compilacion del workspace | Aprobados; 1.090 s y 37.216 s. |
| Suite normal con PostgreSQL/Redis desechables | **1407 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 383.786 s. |
| Clippy 1.98.1 del workspace, todos los targets, warnings denegados | Aprobado, 36.111 s. |
| Pruebas focales nuevas | **15 aprobadas**, 0.753 s; rojo conductual previo de 1 aprobada y 14 fallidas. |
| Suite del dominio | **225 aprobadas**, 0 fallidas e ignoradas, 2.797 s. |
| Rust 1.88 del dominio, todos los targets | Aprobado, 6.366 s. |
| Conservacion de fuentes durante el cierre completo | 852 huellas sin cambios. |

Las quince pruebas se incluyen en las 1407 del workspace y no se suman otra
vez. Comprueban trazas y procedencia, excepciones, febrero bisiesto, inicio fuera
de cobertura, clasificacion sin resolver antes o despues de la candidata, anios
extremos y cantidad `u32::MAX`. Incluso una cantidad maxima recorre a lo sumo
1097 fechas: la cobertura finita y una fecha exterior; no reserva memoria en
funcion de la cantidad solicitada. Un sabado computable se cuenta conforme al
calendario; el algoritmo no incorpora un fin de semana propio.

Cuatro fechas candidatas de fixtures sinteticos se cotejaron con un recorrido
independiente de fechas civiles. Este cotejo verifica aritmetica, no una regla
juridica ni un calendario oficial. No se ejecuto una comparacion automatizada de
las cuatro trazas completas entre ese artefacto y Rust.

Este corte no repite cobertura instrumentada, release ni recorridos CLI, HTTP o
navegador: el cambio solo agrega el componente puro y sus pruebas. Esas cifras
del corte de calendarios que sigue son historicas y no constituyen mediciones del
nuevo codigo. La prueba externa ignorada sigue siendo la del proveedor TSA.

## Corte reproducido: calendarios jurisdiccionales

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). Se verificaron
el catálogo global por rol, las revisiones inmutables de alcance y reglas, la
clasificación de fechas civiles y los recibos de operaciones. El contrato está
en [la API de calendarios](judicial-calendars-api.md) y su decisión en
[ADR-0030](adr/0030-versioned-jurisdictional-calendars.md). Este corte registra
comprobaciones locales, incluida Qadra. La integración remota y la actualización
académica siguen pendientes; el CI de la PR aporta evidencia separada. Las cifras
de audiencias de la sección siguiente son históricas.

| Comprobación | Resultado fresco |
| --- | --- |
| Formato y compilación de todo el workspace | Aprobados; formato 1.126 s y compilación 0.205 s. |
| Suite normal con PostgreSQL/Redis desechables | **1392 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 364.567 s. |
| Suite instrumentada con PostgreSQL/Redis desechables | **1392 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 442.435 s. |
| Cobertura global y tres umbrales del 90 % | **31 403 / 33 823 líneas, 92.8451 %; aprobados.** |
| Clippy 1.98.1, todos los targets, warnings denegados | Aprobado, 3.722 s. |
| Rust 1.88, todos los targets | Aprobado, 26.878 s. |
| Política de dependencias con cargo-deny 0.20.2 | Aprobada, 1.141 s, sin nuevas excepciones. |
| Generador independiente de vectores | Nueve pruebas aprobadas. |
| Restricciones SQL independientes con PostgreSQL | 21 pruebas aprobadas, incluidas carreras de escritura. |
| Verificación de catálogo e inventario al arrancar | 15 pruebas aprobadas tanto en PostgreSQL 18.6 como en PostgreSQL 16.13. |
| Binario release | **13 450 384 bytes**, menor que 26 214 400; 133.309 s. |
| Demostración CLI | Aprobada con target absoluto (7.673 s) y relativo (7.386 s); calibración distinguida abajo. |
| Demostración HTTP con respaldo y restauración | Aprobada, 210.700 s; diez respuestas exactas nuevas de calendarios. |
| Formato, compilación y pruebas unitarias de Qadra | Aprobados; **167 pruebas**, formato 4.410 s, build 2.795 s y unitarios 1.112 s. |
| Navegador con API simulada | **223 aprobadas**, 191.473 s del comando. |
| Navegador con PostgreSQL/Redis aislados | **17 aprobadas**, 291.660 s del script; 2.8 minutos de Playwright. |

Por crate: `domain` 3887/3983, `application` 6100/6377,
`infrastructure` 15100/16323, `web` 5405/5895 y `bin` 911/1245 líneas.
Son 91 pruebas Rust adicionales: 18 de dominio, 25 de aplicación, 30 de
infraestructura y 18 de HTTP. Las pruebas de infraestructura usan conexiones
explícitas a bases desechables; las 15 comprobaciones de arranque están incluidas
en las 30, no se suman otra vez. Las pruebas Python/SQL y los recorridos HTTP
son campañas separadas. La única prueba ignorada corresponde al proveedor TSA
externo.

Después de la suite normal se corrigieron dos localizadores hexadecimales de
prueba para el lint de Rust 1.98.1 y se retiró un `mut` innecesario de otro test.
No cambiaron las fuentes Rust de producción. Los dos tests de canon y los 18 de
HTTP afectados pasaron de nuevo; la suite instrumentada completa también incluye
los tres archivos corregidos. Los dos intentos de Clippy rechazados se conservan
separados de la corrida aprobada y no se cuentan como pruebas adicionales.
Las 848 huellas del corte Rust final permanecieron iguales durante Clippy, MSRV,
cobertura y compilación release.

### Invariantes y persistencia del calendario

Owner administra el catálogo; Owner, Litigator y Paralegal pueden consultarlo sin
seleccionar un expediente. Client queda denegado. La primera revisión fija un
ámbito explícito: fuero, entidades, autoridad, órgano, territorio y uso declarado.
No se infiere competencia a partir de nombres ni se preseleccionan entidades.

Cada revisión conserva una cobertura civil finita, siete reglas semanales,
excepciones no solapadas y referencias públicas declaradas. Dentro de cobertura,
una excepción sustituye la regla semanal completa; una clasificación sin resolver
se distingue de una fecha excluida. Fuera de cobertura no se asigna regla ni se
supone que el día sea computable. Consultar historia o fechas de una revisión
exacta conserva su resultado después de reemplazar o retirar la cabecera.

Siete vectores independientes cotejan JCAL1 entre el generador, el dominio, SQL
y la reconstrucción del adaptador, incluidos 99 y 191 910 bytes. El canon JCTX1
cubre actor, operación, calendario, acción, revisión esperada, digest y motivo;
sus extremos comprobados son 91 y 4095 bytes. La validación conserva Unicode en
los datos y cuenta escalares para las cotas de texto. El cuerpo máximo comprobado
ocupa 232 723 bytes UTF-8 o 517 267 con escapes Unicode; ambos caben en el límite
HTTP de 1 MiB. Las URL se validan con el mismo perfil acotado, sin descargarlas.

Preparar no reserva revisiones ni operaciones. El commit revalida cuenta y rol
después del bloqueo compartido de auditoría, comprueba la cabecera y la operación,
y lee entonces el reloj de captura. Estado y auditoría se confirman en una sola
transacción. Las carreras entre sucesores y el reuso de una operación en raíces
distintas admiten un solo ganador; revocar al actor durante la espera impide la
escritura posterior. Un fallo de auditoría no deja cambios de calendario.

El arranque comprueba funciones, restricciones, expresiones generadas, claves,
triggers y privilegios, además de recorrer raíces e historia. Rechaza huecos,
recibos alterados, cambios de ámbito, autores ausentes y retiros incompatibles.
La baja o el cambio posterior de correo del autor no invalida una captura
histórica legítima. El primer importador de documentos también rechaza un destino
que ya contiene cualquiera de las dos tablas de calendario con filas; esta
ocupación se reprodujo primero como fallo y se corrigió antes de la suite final.

### HTTP, recuperación y demostración CLI

El recorrido HTTP comprueba publicación, reemplazo y retiro, dos Owners que
compiten por la revisión, ámbito inmutable, rechazo de operaciones repetidas,
lectura global del personal, denegación a Client, historia paginada y fechas dentro
y fuera de cobertura. Incluye valores máximos con Unicode y recibos exactos.

Después del respaldo y la restauración se comparan diez respuestas completas de
calendarios, con dos raíces y cuatro revisiones, y las filas de sus dos tablas.
También vuelven a pasar las 21 respuestas anteriores de programación y resultados
con sus cuatro tablas. Se preservan los soportes cifrados, la auditoría y los ZIP
de evidencia del flujo integrado. No se restauró sobre la demo persistente.

El primer recorrido HTTP se interrumpió por una expectativa incorrecta del script:
una consulta inválida produce `400 invalid_query`, no 422. Se corrigieron esa
expectativa y la descripción del contrato, sin cambiar Rust, y se repitió el
recorrido completo. El primer intento de CLI también reprodujo que el script
buscaba el binario en `target/` aun usando `CARGO_TARGET_DIR`; el script ahora
respeta ese directorio y completó el recorrido con archivos temporales. Un segundo
rojo reprodujo la ruta relativa tras cambiar al directorio temporal; fijar la ruta
absoluta antes de ese cambio permitió repetir y aprobar también esa variante.

La calibración Argon2id de esta CLI obtuvo **504.6 ms de promedio sobre cinco
corridas**; al repetir con target relativo obtuvo **501.3 ms**, también sobre
cinco corridas. Ambas están dentro de la banda de 500 a 1000 ms. Son observaciones nuevas del
entorno local; no reemplaza las mediciones anteriores fuera de banda ni elimina
la necesidad de calibrar el entorno de despliegue. Las mediciones coincidieron
con otras tareas y no constituyen un benchmark aislado.

### Interfaz Qadra

El catálogo se abre desde la administración del despacho o la Agenda sin exigir
un expediente seleccionado. Owner puede publicar, reemplazar y retirar; el resto
del personal autorizado dispone de consulta, historia y clasificación de fechas.
La vista mensual y la lista de días mantienen la revisión exacta y distinguen
clasificación sin resolver de falta de cobertura.

Los formularios solicitan ámbito, entidades, fuentes y reglas expresos. El resumen
previo muestra el título que quedará inmutable, y los conflictos conservan el
borrador hasta comparar la base actual. Una respuesta incierta habilita la consulta
del recibo exacto, sin reenviar automáticamente la escritura. Las respuestas tardías
no reemplazan una selección posterior y la denegación de acceso limpia la vista.

Las 167 pruebas unitarias y 223 simuladas incluyen 16 y 19 nuevas, respectivamente.
Se comprobaron permisos, navegación global, reglas y fuentes, días extremos,
paginación, historia, conflictos y conciliación. Las capturas de detalle y
formulario se revisaron en anchos de 1440 y 390 píxeles, sin cortes horizontales.
El formulario móvil conserva desplazamiento vertical para los campos explícitos.
Los 17 recorridos con servicios reales incluyen tres nuevos: ciclo completo y
recibo de respuesta perdida, conflicto entre dos sesiones con borrador conservado
y consulta global según rol. El primer intento se detuvo antes de Playwright por
un campo mal nombrado en el fixture de una fuente, en 155.308 s. Se corrigió
`official_url`, sin cambiar producto, y se repitió toda la campaña con salida cero.

Se auditaron 42 fuentes nuevas o modificadas de interfaz y fixtures: todas ASCII,
con máximo de 298 líneas. Los 22 archivos existentes de estilos, marca y activos
comprobados conservaron sus huellas. Las cifras de la campaña simulada y real
se mantienen separadas; ninguna sustituye una evaluación de usabilidad humana.

### Límites de este corte

El catálogo configura y clasifica fechas; el cálculo automático de vencimientos,
los hechos de notificación, la reevaluación y las alertas siguen pendientes.
Las referencias conservan metadatos declarados, sin archivar ni autenticar el
contenido remoto. La CA interna y TSA local siguen siendo demostración técnica.
La evaluación formal de usabilidad no se sustituye por pruebas automatizadas.

## Corte reproducido: sesiones y resultados declarados

Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`). Se comprobaron
alta, rectificación y retiro de registros, anclas históricas de programación,
continuaciones entre audiencias, comparecencias, acuerdos y soportes exactos.
El contrato está en [la API de resultados](hearing-results-api.md) y la decisión
en [ADR-0029](adr/0029-declared-hearing-sessions.md). Las cifras siguientes corresponden a campañas terminadas; se distinguen las
pruebas Rust, la integración HTTP y los recorridos de Qadra.

| Comprobación | Resultado fresco |
| --- | --- |
| Formato y compilación de todo el workspace | Aprobados; compilación 7.275 s. |
| Suite normal con PostgreSQL/Redis desechables | **1301 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 388.971 s. |
| Suite instrumentada con los mismos servicios aislados | **1301 aprobadas, 0 fallidas y 1 externa ignorada**, salida 0, 384.320 s. |
| Cobertura global y tres umbrales del 90 % | **28 704 / 31 010 líneas, 92.5637 %; aprobados.** |
| Clippy 1.98.1, todos los targets, warnings denegados | Aprobado, 16.274 s. |
| Rust 1.88, todos los targets | Aprobado, 16.005 s. |
| Política de dependencias con cargo-deny 0.20.2 | Aprobada, sin nuevas excepciones. |
| Instalador verificado de formatos | Nueve pruebas aprobadas. |
| Binario release | **12 872 600 bytes**, menor que 26 214 400; 98.501 s. |
| Demostración CLI | Aprobada, 6.726 s; la calibración se distingue abajo. |
| Demostración HTTP con respaldo y restauración | Aprobada, 107.072 s. |
| Formato, compilación y pruebas unitarias de Qadra | Aprobados; **151 pruebas**, formato 4.046 s, build 2.734 s y unitarios 0.963 s. |
| Navegador con API simulada | **204 aprobadas**, 223.553 s del comando; 3.7 minutos de Playwright. |
| Navegador con PostgreSQL/Redis aislados | **14 aprobadas**, 258.642 s del script; 2.5 minutos de Playwright. |

Por crate: `domain` 3220/3292, `application` 5531/5802,
`infrastructure` 14237/15405, `web` 4805/5279 y `bin` 911/1232 líneas.
Son 118 pruebas Rust adicionales respecto de programación. La prueba ignorada
corresponde al proveedor TSA externo. Los adaptadores de identidad, expedientes
y documentos se ejecutaron con bases desechables y variables explícitas;
no se infiere su verificación de una ejecución sin esos servicios.

Las 759 huellas del corte Rust permanecieron iguales al terminar las campañas.
Al incorporar después la corrección de inicialización concurrente de audiencias,
solo cambió `crates/infrastructure/tests/postgres_startup.rs`: su única prueba
se repitió con PostgreSQL real y pasó en 5.948 s del comando. Es una repetición
del mismo caso, no otra prueba que se sume a las 1301. Las fuentes de producción
permanecieron idénticas. Se conservan por separado rojos de TDD, errores de
entorno y las comprobaciones aprobadas.

La CLI midió Argon2id en **409.6 ms de promedio sobre cinco corridas**, por debajo
de la banda objetivo de 500 a 1000 ms. Se conservan los parámetros existentes y
la medición histórica de 529.4 ms; el resultado nuevo no sustituye aquel ensayo.
La calibración del entorno de despliegue sigue pendiente antes del cierre
operativo, como en los cortes anteriores fuera de banda. No se declara cumplido
ese subcriterio por el solo hecho de que la demostración funcional terminó.

### Invariantes de sesiones y resultados

Cada sesión o acto declarado tiene identidad propia y revisiones inmutables.
El alta elige expresamente una revisión de programación, incluso cancelada;
una continuación puede citar un resultado histórico retirado de otra audiencia
del mismo expediente. Rectificar conserva esas referencias y exige un motivo.
Retirar conserva contenido y soporte; no equivale a anular un acto judicial.

Los vectores independientes verifican los bytes y digests de HRES1 y HRTX1,
incluidos sus límites de 146 933 y 4296 bytes. Se comprueban normalización,
precisión de fecha o instante, desfases, años extremos, comparecencias repetidas,
orden e identidad de acuerdos y límites de texto por escalares Unicode. Las
pruebas distinguen un día conocido de una hora desconocida, sin completarla.

La aplicación rechaza fuentes de otro expediente, proyecciones alteradas y
recibos incompatibles con actor, acción o revisión. La administración capturada
no puede preceder a las fuentes históricas de las que depende. Una rectificación
revalida el soporte, aunque sea la misma versión; un retiro copia la admisión
histórica sin abrir nuevamente el archivo. Un documento V2 no reemplaza el
soporte V1. Las respuestas inciertas se concilian con el recibo exacto, sin
reenviar automáticamente una mutación.

PostgreSQL revalida cuenta, rol, membresía y expediente activo después del
bloqueo compartido de auditoría; consulta entonces el reloj de captura. No exige
que sigan vigentes la administración o etapa observadas al preparar, ni que el
perfil administrativo actual esté completo. Fallos de raíz, revisión, auditoría
y commit diferido no dejan cambios parciales. Las carreras entre rectificación
y retiro, o entre raíces que reutilizan una operación, tienen un único ganador.

El arranque rechaza huecos, ciclos, cabeceras alteradas, referencias incompatibles
y protecciones SQL ausentes. Las pruebas reprodujeron y corrigieron capturas
administrativas anteriores a sus fuentes. También se comprobó la reconstrucción
de fechas bajo cuatro configuraciones DateStyle: el canon conserva YYYY-MM-DD
independientemente de la presentación de fechas de la conexión.

### Contrato HTTP y recuperación

La API admite hasta 512 KiB de JSON, incluidos espacios; un byte adicional
produce 413. Rechaza campos y consultas desconocidos, claves repetidas, arreglos
posicionales, datos después del JSON, tipos de contenido incompatibles, tiempos
ambiguos y comandos que no corresponden a la ruta. El límite admite valores
máximos con texto Unicode. La historia devuelve metadatos ligeros y carga el
detalle exacto cuando se solicita.

La demostración HTTP registra una sesión parcial con una ficha archivada y un
PDF sellado V1 después de crear V2. Reprograma y cancela la audiencia, avanza la
etapa y conserva las fuentes del resultado. Rectifica, rechaza una revisión
competidora, retira y registra una continuación desde el antecedente retirado.
También selecciona una programación cancelada y prueba los cuatro roles,
revocación posterior a la preparación y cierre administrativo. El recibo de un
resultado ausente se distingue del error de una fuente ausente o de un expediente
inaccesible.

El respaldo y restauración comparan 21 respuestas completas de audiencias y
resultados, junto con las filas de sus cuatro tablas. Se preservan las tres raíces
y cinco revisiones de resultados, además de cuatro raíces y ocho revisiones de
programación. Se mantienen los soportes cifrados, la auditoría y los ZIP de
evidencia de los flujos previos.

### Interfaz Qadra y navegador

El panel de cada audiencia permite registrar sesiones o actos, consultar su
historia, rectificar contenido y retirar registros con motivo. La programación
se elige por revisión exacta, incluso cancelada; una continuación conserva el
antecedente seleccionado aunque haya sido retirado. Los selectores recuperan
fichas históricas y documentos de versión exacta. La precisión de fecha conserva
la distinción entre día conocido y hora desconocida.

Son 32 pruebas unitarias, 25 escenarios simulados y tres recorridos reales nuevos
respecto del corte de audiencias corregido. Verifican permisos y expediente
cerrado, resultados vacíos, consulta por estado, paginación, historia, fuentes
archivadas, borradores conservados ante conflictos y conciliación de un envío
cuya respuesta se perdió. La lectura del recibo no reenvía la mutación. Las
respuestas tardías de una vista anterior no reemplazan el expediente activo.
Los recorridos reales incluyen una carrera de revisiones y revocación de acceso.

Se revisaron capturas de historial, detalle y formulario en escritorio y móvil,
además de comprobar sus dimensiones y navegación. Las 42 fuentes nuevas o
modificadas del cierre de interfaz conservaron sus huellas durante las campañas;
son ASCII y su máximo es 331 líneas. Los 19 archivos existentes de marca y
estilos comprobados permanecen iguales. El import de la nueva hoja de estilos
reutiliza los componentes y variables de Qadra. Esta revisión funcional y visual
no equivale a una evaluación de usabilidad con participantes humanos.

### Actualización de la demo persistente

Después de las campañas aisladas se actualizó la demo local conservando las
filas de sus 23 tablas existentes, 31 archivos de configuración y material
protegido, roles, credenciales y tres registros Redis aún no caducados. Las dos
tablas nuevas quedaron vacías, sin crear sesiones declaradas a partir de citas.
La API, el frontend y la denegación de acceso anónimo respondieron correctamente
con el binario candidato cuya huella se había fijado antes de actualizar.

La operación requirió recuperación explícita. El primer intento se detuvo antes
de migrar porque exigía un directorio documental local que este despliegue en
PostgreSQL no utiliza; se restableció el binario previo y se movió esa validación
antes de la parada. El segundo migró y verificó todas las filas, pero la
comprobación inmediata del nombre del proceso npm interrumpió el reinicio.
Se recreó la unidad transitoria con el mismo supervisor y políticas, y se
completaron las comprobaciones de estado, archivos y sesiones. No se repitió la
migración ni se restauró la base sobre trabajo posterior. Los dos intentos y
sus respaldos se conservaron por separado; este cierre no se informa como una
actualización que hubiera transcurrido sin incidencias.

### Alcance de la comprobación

Estas pruebas verifican captura, autorización, integridad e historia del registro.
No acreditan que un acto ocurrió, que una persona quedó notificada ni que un
acuerdo tenga efectos judiciales. Los términos automáticos, el calendario de
plazos, las alertas y el ciclo de recursos siguen pendientes. Se conserva la CA
interna y TSA local como demostración técnica; la prueba del proveedor externo
permanece identificada por separado.


## Correcciones reproducidas durante la validación de audiencias

La inicialización concurrente de PostgreSQL conserva el timeout breve solo en
las esperas que deben rechazarse. La rama exitosa espera a los competidores y
los libera después de 750 ms, sin aplicarles el límite de 500 ms que causó el
fallo de CI. Tras reproducir el fallo se aprobaron formato, compilación, las
**1183 pruebas Rust (0 fallos, 1 externa ignorada)** con servicios desechables
(335.294 s) y Clippy de todo el workspace con Rust 1.94 (53.355 s).

Se reprodujo además una pérdida de foco: el evento de hash de una navegación
ya terminada podía devolver el foco al contenedor mientras se escribía en el
filtro. Qadra omite ese evento redundante y conserva el tratamiento de cambios
de ruta externos. El test de permisos espera el resumen del expediente antes
de probar una ruta prohibida al Cliente, evitando competir con la selección
pendiente. Son correcciones distintas: una del producto y otra de la prueba.

La verificación posterior aprobó formato, compilación, **119 pruebas unitarias,
179 escenarios con API simulada** (214.992 s) y **11 recorridos reales**
(310.470 s de preparación y ejecución; 3.2 minutos de Playwright). Se conservan
por separado los fallos reproducidos, los intentos interrumpidos por espacio
insuficiente en el directorio temporal y las ejecuciones aprobadas. Estas cifras
complementan el corte original siguiente; no actualizan retrospectivamente su
cobertura, medición CLI ni PDF.

## Corte reproducido: programación de audiencias

- Fecha local: 16 de septiembre de 2026 (`America/Mexico_City`).
- Alcance: programación de cuatro tipos, reemplazo y cancelación organizativa,
  referencias exactas de participantes y soportes, historial inmutable, recibos
  propios de operación y agenda autorizada.
- Contrato: [API de audiencias](hearings-api.md); decisión:
  [programación auditada](adr/0028-audited-hearing-scheduling.md).
- Entorno: Rust/Cargo 1.94.0, PostgreSQL 18.6, Valkey 8.1.9, OpenSSL 3.5.7,
  qpdf 12.4.1, Node.js 22.22.2 y npm 10.9.7, Linux x86_64. Clippy usa
  adicionalmente 1.98.1 para comprobar el toolchain observado en CI.

### Resultados ejecutados

| Comprobación | Resultado fresco |
| --- | --- |
| Formato, compilación y Clippy 1.98.1 del workspace | Aprobados. |
| Rust 1.88, todos los targets | Aprobado. |
| Suite normal mediante `scripts/test-backends.sh` | **1183 aprobadas, 0 fallidas y 1 externa ignorada; salida 0**, 360.492 s. |
| Suite instrumentada con los mismos servicios aislados | **1183 aprobadas, 0 fallidas y 1 externa ignorada; salida 0**, 370.401 s. |
| Cobertura y tres umbrales del 90 % | **25 201 / 27 226 líneas, 92.5623 % global; aprobados.** |
| Política de dependencias con `cargo-deny 0.20.2 check` | Aprobada; sin nuevas excepciones. |
| Compilación release | **12 176 936 bytes**, menor que 26 214 400; 126.825 s. |
| `scripts/demo.sh` | Aprobado, 9.097 s. |
| `scripts/api-demo.sh` | Aprobado, 166.180 s, incluidas audiencias y restauración. |
| Formato, compilación y pruebas unitarias de Qadra | Aprobados; **119 pruebas unitarias**. |
| Navegador con API simulada | **178 aprobadas**, 152.37 s del comando. |
| Navegador con servicios reales aislados | **11 aprobadas**, 2.1 minutos de Playwright; 222.42 s del script con preparación. |

La cobertura por crate fue 2730/2802 líneas en `domain`, 4813/5055 en
`application`, 12653/13694 en `infrastructure`, 4094/4458 en `web` y
911/1217 en `bin`. Se conserva la exportación de la instrumentación junto con
los logs y las huellas de fuentes; exportar de nuevo el informe no se cuenta
como otra ejecución de pruebas.

Son **117 pruebas Rust adicionales** respecto del cierre de participantes. La
suite usó PostgreSQL y Redis desechables, bases independientes para identidad,
expedientes y documentos, y qpdf preparado con su instalador verificado. No se
contabilizan adaptadores omitidos por variables ausentes. La prueba ignorada es
la del proveedor TSA externo. El instalador de qpdf también pasó sus nueve
pruebas. Se conservan los avisos informativos y las políticas previas.

Clippy 1.98.1 detectó `chunks_exact` con tamaño constante en el auxiliar que
lee los vectores de una prueba. Se sustituyó por `as_chunks::<2>()` y se
comprobaron otra vez las cuatro pruebas del códec, Clippy de todo el workspace
y Rust 1.88 con todos los targets. No se cambió el código del producto. La
suite normal anterior se identifica como ese corte; la instrumentada posterior
se informa con su propia ejecución.

La medición CLI de Argon2id fue **540.7 ms de promedio en cinco ejecuciones**,
dentro de la banda de 500 a 1000 ms. Se ejecutó mientras otras verificaciones
estaban activas; no es una calibración aislada ni sustituye mediciones anteriores.

### Invariantes y concurrencia de audiencias

Los vectores independientes de `HEAR1` y `HTXN1` comprueban bytes, proyecciones,
digests, normalización y límites. Se probaron segundos y desfases explícitos,
años locales y UTC, participantes repetidos, contexto incompatible, antecedente
obligatorio para individualización y secuencia sin desbordamiento. La API
rechaza arreglos posicionales, duplicados, campos desconocidos, fechas ambiguas,
cuerpos excesivos y comandos incompatibles con la ruta.

PostgreSQL revalida miembro, rol, cuenta activa, cierre, administración, etapa,
participantes y soporte al confirmar. Los ensayos intercalan cambios después de
preparar: ninguno deja una audiencia ni un evento parcial. Se inyectaron fallos
de raíz, revisión, auditoría y commit diferido, comparando las filas completas
antes y después. Una carrera entre reemplazo y cancelación produce un único sucesor
para la revisión esperada.
Dos raíces con el mismo UUID de operación producen una única confirmación.

Una lectura bloqueada observa una membresía retirada aunque la conexión tenga
por defecto aislamiento repetible. Un reloj controlado demuestra que la fecha
de captura de la escritura se consulta después de esperar el bloqueo de
la auditoría. Fallar el evento impide devolver contexto, detalle, historia,
listado o agenda. Filtros de cabecera y autorización preceden a la paginación.

Las fichas manuales y tipificadas conservan su revisión e identidad exactas al
editar o archivar la ficha o su identidad. Una nueva selección obsoleta se
rechaza. Cancelar después de avanzar la etapa conserva el contexto original y
captura por separado la administración vigente. El soporte histórico V1 permanece
vinculado después de crear V2. Un sellado concurrente del soporte invalida la
preparación; cancelar copia lo ya capturado sin reabrir el archivo ni declarar
una nueva comprobación de integridad.

El arranque rechaza historia incompleta, referencias alteradas, selección de una
revisión originalmente archivada, permisos excesivos y protecciones ausentes o
deshabilitadas. Las pruebas SQL reprodujeron antes de corregir la aceptación de
un digest de etapa falso y de un hueco de revisiones; también reprodujeron una
omisión del inventario al resolver fuentes exactas. La migración no inventa
citas anteriores. El import inicial rechaza tablas de audiencias ocupadas.

### HTTP y recuperación

La demostración HTTP prueba preparación normalizada, recibos por revisión,
reutilización rechazada de operación, digest de envío distinto, permisos de los
cuatro roles y aislamiento. Registra una audiencia, la reemplaza y cancela tras
un cambio de etapa; retiene fichas archivadas e identidad tipificada original.
Individualización comprueba soporte PDF sellado V1 cuando V2 ya existe. La agenda
usa intervalos explícitos y cursor de tiempo y UUID.

El respaldo y la restauración compararon **12 respuestas completas de audiencias**
y las filas de sus dos tablas, con **3 raíces y 5 revisiones**. El inventario
restaurado incluyó 11 expedientes, 20 revisiones administrativas, 6 registros
iniciales de etapa, 12 raíces documentales, 15 versiones, 3 clasificaciones,
6 raíces de participantes y 9 revisiones manuales, 4 revisiones tipificadas,
2 identidades con 4 revisiones y una credencial. Las fuentes de etapas, soportes,
autores y recibos se conservaron. La importación legacy mantuvo cuatro documentos
y su prefijo de 63 eventos originales; los ZIP se compararon byte por byte.

El guion se corrigió para incluir también ambas tablas de audiencias en el
inventario SQL general. La ejecución registrada ya cargó esa corrección y
comprobó las filas, además de las respuestas HTTP. Las pruebas PostgreSQL específicas
cubren respaldo completo, autoría de una cuenta posteriormente inactiva y lectura
de un expediente cerrado; otra comprueba el canon y los guards con `search_path`
vacío durante la restauración.

### Qadra y navegador

La interfaz permite programación, revisión previa del comando normalizado,
reemplazo, cancelación, referencias exactas, historia y agenda filtrada. Conserva
los 18 archivos originales de marca y estilos; las capturas de 1440 y 390 píxeles
incluyen listado, formulario, agenda y referencias desplegadas, sin desborde.
Las nuevas referencias permiten distinguir fichas homónimas por UUID y revisión.

Las regresiones reprodujeron una revisión histórica presentada como vigente y
una apertura desde Agenda que lanzaba tres lecturas frente al límite de dos
operaciones simultáneas del servidor. La corrección distingue la revisión exacta
y espera contexto/listado antes de abrirla; no aumenta el límite ni repite
escrituras. Los escenarios simulados finales pasaron tras esas correcciones.

Los tres recorridos reales nuevos comprueban respuesta perdida después del
commit y conciliación por recibo, retención manual/tipificada, dos sesiones con
revisión esperada, cancelación tras cambio de etapa, permisos/asignaciones/cierre/
revocación y soporte sellado V1 con V2 existente. El ZIP histórico permanece
idéntico. Los ocho recorridos anteriores también aprobaron.

Las primeras campañas reales detectaron dos errores del guion: usar como método
el atributo `status` de una respuesta nativa, y contar módulos JavaScript públicos
como llamadas HTTP de negocio. Se corrigieron los auxiliares y se repitieron los
once recorridos en servicios nuevos. Los intentos fallidos se conservaron; el
resultado aprobado corresponde a la repetición de 222.42 segundos.

Una revisión estática focal de aplicación y transporte no encontró divergencias
con el contrato en permisos, recibos, cancelación, contexto, objetos estrictos,
fechas, paginación o errores. Es una revisión del alcance descrito y no sustituye
la campaña ejecutada ni una auditoría general del sistema.

### Actualización de la demostración local

Después de la campaña integrada se actualizó el servidor de desarrollo con un
respaldo privado de PostgreSQL, una instantánea RDB de Redis y copias de sus
archivos de configuración y evidencia. Se detuvo el escritor y se esperó el fin
de sus procesos y conexiones antes de copiar los archivos mutables. La migración
conservó las filas completas de las **21 tablas existentes** y creó las dos
tablas de audiencias vacías; no inventó citas históricas. También se conservaron
las huellas de **30 archivos** de claves, certificados, configuración y acceso.

Los nuevos procesos se comprobaron por identidad, directorio y pertenencia al
servicio. Salud y frontend respondieron 200; la agenda nueva rechazó consulta
sin sesión con 401. El navegador mostró la pantalla de acceso. Esta comprobación
de arranque no se contabiliza como otro recorrido funcional de los once anteriores.

Esta programación no registra celebración, asistentes reales, resultados ni
acuerdos; no activa términos ni completa el calendario judicial, las alertas,
los recursos o la identidad y firma documental individual. La evaluación de
usabilidad con personas sigue pendiente. No se sustituyen los objetivos
aprobados ni se cierran conclusiones académicas con esta entrega.

## Repetición de cierre: identidades y declaraciones

La repetición del 15 de septiembre de 2026, por la noche en
`America/Mexico_City`, comprobó el software de `f2dfd3c`. Los resultados del
primer corte se conservan a continuación con sus propios denominadores.

| Comprobación ejecutada de nuevo | Resultado |
| --- | --- |
| Formato, compilación y Clippy de todo el workspace | Aprobados. |
| Suite normal con `scripts/test-backends.sh` | **1066 aprobadas, 0 fallidas, 1 externa ignorada; salida 0.** |
| Suite instrumentada con PostgreSQL/Redis desechables | **1066 aprobadas, 0 fallidas, 1 externa ignorada; salida 0.** |
| Cobertura y sus tres umbrales del 90 % | **21 967 / 23 773 líneas, 92.4031 % global; aprobados.** |
| Rust 1.88 y `cargo-deny 0.20.2 check` | Aprobados. |
| Binario release | **11 673 656 bytes**, por debajo de 26 214 400. |
| `scripts/demo.sh` y `scripts/api-demo.sh` | Aprobados, incluida restauración y comprobación independiente con OpenSSL. |
| Formato, compilación y pruebas unitarias de Qadra | Aprobados; **90 pruebas unitarias**. |
| Navegador con API simulada | **147 aprobadas**, 1.9 minutos. |
| Navegador con servicios reales aislados | **8 aprobadas**, 2.0 minutos; script completo 263.047 segundos con preparación. |

La cobertura de esta repetición fue 2441/2513 líneas en `domain`, 4129/4342
en `application`, 11211/12150 en `infrastructure`, 3275/3568 en `web` y
911/1200 en `bin`. El conteo de infraestructura difiere en una línea del primer
corte; no se sustituyó su medición anterior ni se repitió para igualarla.
Los backends se ejecutaron con las variables aisladas configuradas y qpdf real.
La única prueba ignorada sigue siendo la del proveedor TSA externo. Esta
repetición normal también terminó con salida exterior 0; no atribuye una causa
a la discrepancia histórica del supervisor.

El instructivo del ZIP ahora explica que exportar no renueva la CRL y permite
consultar `lastUpdate` y `nextUpdate` con OpenSSL. La primera ejecución encontró
que la prueba del paquete aún esperaba siete comandos; se ajustó para ejecutar
los ocho y comprobar las fechas de la lista. La prueba dirigida y la suite
completa posterior aprobaron. El ZIP se regenera con el instructivo del software
actual: sus bytes pueden diferir de una descarga anterior a esta corrección,
sin modificar documento, firma, sello, certificados ni CRL almacenados. La
comparación binaria de respaldo/restauración se hizo bajo el mismo código.

La revisión de interfaz corrigió claves de error de certificado que no coincidían
con el contrato HTTP y añadió mensajes específicos para CRL y límites. La nueva
regresión falló antes de la corrección y pasó después. El primer intento de
navegador abortó antes de ejecutar pruebas por el lock de Astro del servidor
local; las configuraciones de prueba usan `--ignore-lock` y puertos separados.
Ambas campañas completas aprobaron conviviendo con la demostración local, cuya
API y Qadra continuaron respondiendo. Los triggers de CI incluyen el nuevo
fixture de participantes tipificados.

La demostración CLI registró **615.7 ms de promedio sobre cinco corridas de
Argon2id**, dentro de la banda de 500 a 1000 ms. Es una nueva medición local con
otras comprobaciones concurrentes; no sustituye los 1145.9 ms del primer corte
ni constituye calibración del despliegue. La revisión de fuentes propias
comprobó ASCII y el límite de tamaño, con máximo de 387 líneas. Las hojas de
estilo originales, marca, fuentes académicas protegidas y entregables previos
conservaron sus hashes. No se ejecutó evaluación de usabilidad con personas.

### Compatibilidad del cierre con CI

La primera ejecución remota utilizó Rust/Clippy 1.98.1. Detectó la nueva
advertencia `chunks_exact_to_as_chunks` en el lector de vectores de una prueba;
se sustituyó por `as_chunks::<2>().0.iter()`. El auxiliar conserva su entrada
y resultado. Las seis pruebas de códec aprobaron, así como Clippy 1.98.1,
`cargo +1.88.0 check --workspace --all-targets`, formato, compilación y otra
suite normal completa con **1066 aprobadas, 0 fallidas y 1 externa ignorada**.

Los primeros jobs remotos de pruebas y cobertura agotaron el disco durante el
enlace, antes de ejecutar la suite: el runner informó 0 MB y 83 MB libres.
El [perfil de depuración de CI](adr/0027-ci-debug-information.md) limita los
artefactos a tablas de líneas, conservando pruebas, instrumentación, umbrales
y perfil release. La compilación local de los 188 ejecutables de pruebas con
Rust 1.98.1 y ese perfil terminó correctamente; sus binarios sumaron
**5 329 180 320 bytes**, sin contar dependencias y archivos auxiliares.
La resolución en el runner queda sujeta a la siguiente ejecución de CI; la
compilación local no se presenta como prueba remota aprobada. La primera CI sí
aprobó las campañas web, el PDF, MSRV, formato, dependencias y tamaño release.
Estas correcciones de pruebas y configuración no cambian las fuentes del
manuscrito ni invalidan la correspondencia de sus 60 hashes con el PDF local.

### Resultado remoto e integración de participantes

El segundo head, `3ba738176de57a55ae7a3885b13f6bf8fd9e8c31`, obtuvo
19 comprobaciones aprobadas y la publicación de release omitida por tratarse de
una PR. Las suites remotas de pruebas y cobertura terminaron correctamente con
el perfil de tablas de líneas. Se integró por squash el 16 de septiembre de
2026 a las 05:42:25 UTC en `3494fa2bed6879ebf57441034000597b7d20e3a7`.
El árbol integrado coincide con el head comprobado. Este resultado cierra la
incertidumbre remota del apartado anterior sin sustituir sus mediciones locales.

El PDF remoto tuvo 281 páginas. Sus 60 fuentes académicas coincidieron con las
del PDF local y el texto extraído coincidió página por página; las 55 páginas
renderizadas para comparación fueron idénticas. Los archivos PDF tienen hashes
binarios distintos y se conservaron por separado. No se declara reproducción
binaria. Esta integración corresponde a participantes; no completa los flujos
procesales y de identidad que siguen pendientes.

## Corte reproducido: identidades y participantes tipificados

- Fecha local: 15 de septiembre de 2026 (`America/Mexico_City`).
- Alcance: identidades representadas por expediente, once perfiles procesales,
  revisión explícita de coincidencias, declaraciones con firma externa y confianza
  interna publicada, historial mixto y flujos Qadra.
- Decisiones: [identidades y perfiles](adr/0025-case-subjects-and-typed-participants.md)
  y [declaraciones internas](adr/0026-internal-participant-declarations.md).
- Entorno comprobado: Rust/Cargo 1.94.0, PostgreSQL 18.6, Valkey 8.1.9,
  OpenSSL 3.5.7, Node.js 22.22.2, npm 10.9.7 y qpdf 12.4.1, Linux x86_64.

### Pruebas y recuperación de participantes

| Comprobación | Resultado reproducido |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo build --workspace` | Aprobadas. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobada. |
| `cargo test --workspace` mediante `scripts/test-backends.sh` | **1066 aprobadas**, 0 fallidas y 1 externa ignorada. Cargo y el wrapper registraron salida 0. |
| Suite instrumentada mediante `scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only` | **1066 aprobadas**, 0 fallidas y 1 externa ignorada; salida 0. |
| `scripts/coverage-gate.sh` sobre el JSON instrumentado | Los tres umbrales del 90 % aprobados. |
| Binario release | **11 673 656 bytes**, menor que el límite de 26 214 400 bytes. |
| `cargo +1.88.0 check --workspace` | Aprobada con la MSRV declarada. |
| `cargo-deny 0.20.2 check` | Avisos, restricciones, licencias y fuentes aprobados. |
| `scripts/demo.sh` | Recorrido criptográfico CLI aprobado. |
| `scripts/api-demo.sh`, con firma externa y restauración | Recorrido completo y repetición final aprobados con salida 0. |
| Qadra, lógica y contratos de interfaz | **89 pruebas unitarias y 147 escenarios con HTTP simulado** aprobados. |
| `scripts/web-demo.sh`, Qadra con servicios reales | **8 recorridos aprobados** en 1.8 minutos. |
| Compilación y formato de Qadra | Aprobados. |

Son **133 pruebas Rust adicionales** respecto de las 933 del corte anterior.
Las variables de backend apuntaron a PostgreSQL con SCRAM y Redis desechables,
con bases separadas para identidad, expedientes y documentos; qpdf se preparó
con su instalador verificado. No hubo omisiones por ausencia de estos servicios.
La única ignorada sigue siendo `the_real_sandbox_issues_a_token`. Persisten el
aviso informativo de compatibilidad futura de Redis y los duplicados permitidos
de dependencias. No se cambiaron las políticas de seguridad para aprobar.

El supervisor exterior de la repetición no instrumentada informó código 143,
aunque tanto Cargo como el wrapper registraron su salida 0 después de terminar.
No se atribuye una causa no reproducida a esa discrepancia. La campaña
instrumentada independiente completó la misma suite y su cierre con salida 0.

Los ensayos dirigidos previos incluyeron 125 pruebas HTTP, 46 del perfil
criptográfico y sus regresiones, 19 de confianza publicada, seis del códec y
23 adaptaciones históricas del directorio y cierre. Son subconjuntos de las
campañas completas, no pruebas adicionales que deban sumarse. Los casos nuevos
comprueban permisos y aislamiento, revisión esperada, revisión de candidatos
más allá de la primera página y límite global de 16 coincidencias, integridad
de proyecciones compactas y conservación de señales históricas de certificado.

Las pruebas negativas reprodujeron y corrigieron una consulta compacta que no
rechazaba valores canónicos alterados y una recuperación de credencial que
necesitaba comprobar la vinculación completa entre declaración, perfil y
operación aceptada. Se rechazan declaraciones válidas de otra ficha y tiempos
de aceptación anteriores a su verificación. La publicación de CRL durante la
preparación y el vencimiento mientras el commit espera su bloqueo rechazan la
mutación sin filas ni eventos de éxito. El instante de verificación capturado
no se sustituye al entrar en la transacción.

La demostración HTTP preparó una declaración de 218 bytes, recibió una firma
externa de 384 bytes y rechazó una firma alterada. Repetir una operación ya
aceptada produjo conflicto. Editar la identidad a R2 conservó R1 en las fichas
anteriores; archivar el rol mantuvo el origen de su evidencia. Se compararon diez
respuestas completas después de restaurar la base y se verificó la firma otra
vez con OpenSSL y el certificado público recuperado. El inventario restaurado
incluyó ocho expedientes, 16 revisiones administrativas, tres registros iniciales,
10 raíces documentales, 12 versiones, tres clasificaciones, cuatro participantes,
seis revisiones manuales y tres tipificadas, una identidad con dos revisiones y
una credencial. El prefijo de importación legacy de cuatro documentos y 63 eventos
es una fixture separada del estado completo final.

### Cobertura de identidades y declaraciones

**21 968 de 23 773 líneas cubiertas: 92.4 % global.** Se incluye el código
instrumentado nuevo, sin exclusiones para aprobar los umbrales.

| Crate | Líneas cubiertas / instrumentadas | Cobertura |
| --- | ---: | ---: |
| `domain` | 2441 / 2513 | 97.1 % |
| `application` | 4129 / 4342 | 95.1 % |
| `infrastructure` | 11212 / 12150 | 92.3 % |
| `web` | 3275 / 3568 | 91.8 % |
| `bin` | 911 / 1200 | 75.9 % |

Es cobertura de líneas; no representa avance porcentual del TT ni cumplimiento
jurídico. Las mediciones anteriores conservan su fecha y denominador.

### Interfaz y límites de esta evidencia

La campaña real de Qadra aprobó ocho recorridos. Los dos nuevos verifican la
política de lectura y gestión de perfiles institucionales y el flujo personal
con firma externa, cambio de identidad, historia vinculada y nueva autenticación.
Una ejecución previa completó siete recorridos y detectó que el guion esperaba
el tablero después de recargar participantes; se corrigió la navegación y se
repitió el conjunto completo. Otro intento anterior se detuvo al arrancar durante
el ajuste de nombres de restricciones PostgreSQL y no ejecutó casos de navegador.

Después de la campaña real se mejoró únicamente la presentación del selector
de archivos y la separación del texto de revisión. Un escenario dirigido con
HTTP simulado comprobó la apertura del selector nativo, la declaración de 218
bytes y la firma separada de 384 bytes; compilación y formato aprobaron otra vez.
Tres recorridos adicionales verificaron el filtro de rol manual y su presentación
móvil. Estas repeticiones no se suman como nuevos escenarios. Se inspeccionaron
capturas de escritorio y móvil; los originales de marca y las siete hojas de
estilo originales conservan sus bytes. La revisión de 242 archivos de software
y configuración modificados encontró ASCII y fuentes menores de 400 líneas,
con máximo de 387; `Cargo.lock` se comprobó por separado para ASCII.

La demostración CLI midió Argon2id en **1145.9 ms de promedio sobre cinco
corridas**, por encima de la banda objetivo de 500 a 1000 ms. El entorno tenía
otras verificaciones concurrentes; este ensayo no aísla su efecto. Se conserva
la medición y los parámetros, y sigue pendiente calibrar el entorno de despliegue.
No se sustituye con las cifras históricas de otros ensayos.

La declaración acredita una firma verificable bajo la CA interna publicada;
no establece identidad civil, habilitación profesional ni validación FIREL.
No implementa inicio de sesión por certificado ni firma documental individual
por cuenta. La TSA local, la falta de anclaje externo de auditoría, las audiencias,
plazos, recursos, alertas, ciclo de miembros e informes conservan sus límites y
pendientes en la [matriz funcional](product-completion.md). Los recorridos
automáticos no sustituyen una evaluación de usabilidad con personas reales.

## Corte reproducido: adopción y transiciones de etapa

- Fecha local: 15 de septiembre de 2026 (`America/Mexico_City`).
- Alcance: adopción explícita, Investigación a Intermedia e Intermedia a Juicio,
  soportes de versión exacta, historial inmutable y flujo Qadra. La política
  `pdf_docx_v1` admite nuevos soportes mediante un worker acotado; no valida
  retrospectivamente ni completa la política de toda carga general.
- Decisiones: [etapas auditadas](adr/0023-audited-case-stage-transitions.md) y
  [admisión aislada](adr/0024-isolated-document-format-admission.md).
- Entorno: Rust/Cargo 1.94.0, PostgreSQL 18.6, Valkey 8.1.9, OpenSSL 3.5.7,
  Node.js 22.22.2, npm 10.9.7 y qpdf 12.4.1, Linux x86_64.

### Pruebas y recuperación

| Comprobación | Resultado reproducido |
| --- | --- |
| `cargo fmt --all`, `cargo build --workspace` | Aprobadas. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobada. |
| `bash scripts/test-backends.sh` | **933 aprobadas**, 0 fallidas y 1 externa ignorada. |
| Suite instrumentada y gate de cobertura | **933 aprobadas**, 0 fallidas y 1 externa ignorada; tres umbrales del 90 % aprobados. |
| Binario release | **10 288 216 bytes**, menor que el límite de 26 214 400 bytes. |
| `cargo +1.88.0 check --workspace` | Aprobada con la MSRV declarada. |
| `cargo-deny 0.20.2 check` | Avisos, restricciones, licencias y fuentes aprobados. |
| Instalador nativo | **9 pruebas aprobadas**; checksum, extracción acotada, reutilización y cuatro instaladores concurrentes. |
| `bash scripts/demo.sh` | Recorrido criptográfico CLI aprobado. |
| `bash scripts/api-demo.sh` | Flujo integrado y restauración aprobados con PDF/DOCX y PostgreSQL/Redis/TSA reales. |

Son **163 pruebas Rust adicionales** respecto de las 770 del corte anterior.
Las variables de backend se configuraron contra servicios desechables separados;
las pruebas nativas usaron la biblioteca verificada, sin omisiones por ausencia.
La única ignorada sigue siendo `the_real_sandbox_issues_a_token`. Persisten los
avisos informativos de compatibilidad futura de Redis y duplicados de dependencias.

El primer control de licencias rechazó `zlib-rs` porque `Zlib` no estaba en la
lista permitida. Se leyó el aviso incluido en su distribución y se documentaron
sus condiciones en ADR-0024 antes de incorporarla a la política permisiva.
No se añadieron excepciones de seguridad. Las bibliotecas nativas de qpdf quedan
fuera del inventario Cargo y tienen su propia preparación y revisión operativa.

Los casos verifican precisión temporal y desfase, canon CSTG1 entre Rust y SQL
incluido máximo de 5759 bytes, registro inicial sin procedencia fabricada,
permisos, revisión esperada y preparación única por referencia. El mismo archivo
puede servir en dos papeles sin duplicar descifrado ni validación. Los **35 casos
nuevos de PostgreSQL** comprueban transacciones, rollback, cierre/revocación,
concurrencia, versiones exactas, esquema, inventario y restauración. El servicio
no consulta de nuevo después del commit para construir la respuesta.

La revisión con pruebas negativas detectó y corrigió aceptación de un trigger
de secuencia deshabilitado al arrancar, una discontinuidad histórica después de
restaurar el trigger, soportes duplicados con distinto formato y prioridad de
error incorrecta ante sellado concurrente con evidencia excesiva. Un fixture de
versión 7 requería agrupar sus inserciones por la clave foránea diferida; fue
corregido en la prueba y no se atribuye como defecto del backend.

La campaña HTTP final comprobó dos avances simultáneos con una confirmación y
un conflicto, fecha/desfase preservados, V1 seleccionada después de añadir V2,
PDF y DOCX reales, adopción sin etapa anterior inventada, cierre, revocación e
historia paginada. Restauró **7 expedientes, 15 revisiones administrativas y 3
registros iniciales**, además de **9 raíces documentales, 11 versiones, 3 revisiones
de clasificación, 2 participantes y 6 revisiones del directorio**. Las filas
completas de etapas, detalle e historia conservan valores, fechas, actores y
origen inicial; los ZIP anteriores coinciden byte por byte. El prefijo importado
sigue siendo **4 documentos y 63 eventos**, distinto del total final de auditoría.

Los parsers tienen casos de PDF con xref/object streams, actualización incremental
y estructuras dañadas, DOCX de productor independiente y lotes comprimidos que
exceden el presupuesto compartido. Se comprueban límites instalados antes de
leer, tuberías de 4096 bytes, timeout y recolección del PID. Una regresión con
instrumentación reprodujo SIGXFSZ por el escritor de perfiles LLVM al salir.
El worker libera recursos, vacía la respuesta y termina sin handlers `atexit`;
las mismas **9 pruebas nativas** pasan también instrumentadas. El hijo no genera
perfiles propios; las pruebas de biblioteca permanecen instrumentadas. Esos
nueve casos repetidos no se suman al total como pruebas distintas.

### Cobertura del registro procesal

**15 817 de 17 064 líneas cubiertas: 92.7 % global.** Los tres crates sujetos
al umbral del 90 % pasan el gate versionado. Se incluye el código instrumentado
nuevo; no se han excluido parsers ni el crate de infraestructura para aprobar.

| Crate | Líneas cubiertas / instrumentadas | Cobertura |
| --- | ---: | ---: |
| `domain` | 1859 / 1893 | 98.2 % |
| `application` | 3294 / 3430 | 96.0 % |
| `infrastructure` | 7699 / 8354 | 92.2 % |
| `web` | 2129 / 2271 | 93.7 % |
| `bin` | 836 / 1116 | 74.9 % |

Es cobertura de líneas, no porcentaje de objetivos ni conformidad documental.
El 93.4 % del corte administrativo se conserva como medición histórica, con un
denominador diferente. La regla de salida del hijo y su ausencia de perfiles
propios se explican arriba; los ensayos de biblioteca sí aportan instrumentación.

### Interfaz y límites de interpretación

La suite de interfaz aprobó **56 pruebas unitarias y 129 escenarios con HTTP
simulado**, además de compilación y formato. Los seis recorridos iniciales con
servicios reales aprobaron en aproximadamente 1.3 minutos, incluidos adopción y
transiciones, junto con administración, participantes, clasificación y versiones.
La campaña final posterior a los controles de arranque y la salida del worker aprobó también
los **seis recorridos**, en aproximadamente **1.5 minutos**, sin sumar ambas
ejecuciones como doce escenarios distintos. Escritorio y móvil conservan el sistema de diseño Qadra; los originales de marca
y las siete hojas de estilo originales no se modificaron. La revisión de las 162 fuentes y configuraciones de software modificadas encontró solo
ASCII y archivos menores de 400 líneas, con máximo de 381. No se atribuye a los
ensayos automáticos una evaluación de usabilidad con usuarios del despacho.

La demostración CLI final midió Argon2id en **371.7 ms de promedio sobre cinco
corridas**, por debajo de la banda objetivo de 500 a 1000 ms. La cifra de 529.4 ms
del hardware y ensayo históricos no se sustituye ni se presenta como medición
actual. Se mantienen los parámetros existentes; la calibración del entorno de
despliegue sigue requiriendo revisión antes del cierre operativo.

Los soportes prueban admisión técnica, sin certificar autenticidad jurídica,
conformidad PDF/OOXML completa ni ausencia de malware. El worker tiene límites
de recursos, no un sandbox general de archivos/red. Su límite de memoria no
incluye buffers ni criptografía del proceso padre. La identidad tipificada,
recursos, audiencias, cómputo de términos, alertas, firma personal, ciclo de
miembros e informes conservan sus pendientes en la [matriz funcional](product-completion.md).
La TSA local y la auditoría sin anclaje externo conservan sus limitaciones.

## Corte reproducido: perfil y administración del expediente penal

- Fecha local: 14 de septiembre de 2026 (`America/Mexico_City`); registros UTC
  correspondientes al 15 de septiembre.
- Alcance: alta penal completa, perfil y revisiones administrativas, índice e
  historia autorizados, cierre y reapertura, conservación del registro inicial
  de etapa y formularios Qadra ante conflictos y resultados inciertos.
- Decisión: [administración auditada](adr/0022-audited-penal-case-administration.md).
- Entorno comprobado: Rust/Cargo 1.94.0, PostgreSQL 18.6, Valkey 8.1.9,
  OpenSSL 3.5.7, Node.js 22.22.2 y npm 10.9.7.

### Backend y recuperación de expedientes

| Comprobación | Resultado reproducido |
| --- | --- |
| `cargo fmt --all`, `cargo build --workspace` | Aprobadas. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobada. |
| Política de dependencias con `cargo-deny 0.20.2` | Avisos, restricciones, licencias y fuentes aprobados, sin nuevas excepciones. |
| `bash scripts/test-backends.sh` | **770 aprobadas**, 0 fallidas y 1 externa ignorada. |
| Suite instrumentada con `cargo llvm-cov --workspace` | **770 aprobadas**, 0 fallidas y 1 externa ignorada. |
| `bash scripts/demo.sh` | Recorrido criptográfico CLI aprobado. |
| `bash scripts/api-demo.sh` | Flujo documental, participantes, administración penal, concurrencia, importación y restauración aprobados. |

Las pruebas usaron PostgreSQL con contraseña SCRAM y Redis desechables. El
wrapper separa bases de identidad, expedientes y documentos, y los nuevos
ensayos aíslan esquemas y roles operativos. La prueba externa ignorada sigue
siendo `the_real_sandbox_issues_a_token`; no se ejecutó un proveedor externo.
Permanece la advertencia de compatibilidad futura de `redis 0.25.4`.

Las **66 pruebas Rust nuevas** respecto del directorio se distribuyen en 24 de
dominio/aplicación, 12 HTTP y 30 de infraestructura. Comprueban:

- Perfil completo, límites en escalares Unicode, controles rechazados antes del
  recorte, CRLF normalizado a LF solamente en información general, delitos
  ordenados sin duplicados literales y ausencia explícita de opcionales.
  Tres vectores CADM1 y el máximo de **12 717 bytes** coinciden entre Rust y SQL,
  con SHA-256 calculado por los adaptadores. Los valores máximos escapados caben
  en el límite HTTP completo de 64 KiB.
- Alta básica con R1 pendiente y tres eventos; alta penal con R1 completa,
  asignación del creador, Investigación inicial y cuatro eventos atómicos.
  La raíz previa se proyecta como R0 sin historia ni procedencia fabricadas.
  Completar perfiles pendientes no crea una etapa. El registro inicial conserva
  el digest, actor y tiempo de su R1 exacta después de editar o cerrar.
- Owner global, Litigator y Paralegal asignados, permisos de edición/consulta y
  denegación de perfiles e historia a Client. Su proyección básica conserva
  exactamente cuatro campos y refleja título/referencia actuales.
- Revalidación de cuenta y asignación después de esperar el bloqueo común;
  rechazo de lecturas básicas y administrativas si falla la escritura auditada.
  Ningún comando necesita una lectura posterior al commit para responder.
- Revisiones esperadas, un solo sucesor entre ediciones simultáneas, estado que
  conserva los textos vigentes y perfil completo que no puede eliminarse.
  La unicidad de NUC y carpeta se comprueba por separado sobre cabezas actuales,
  incluidos expedientes cerrados; una corrección libera el valor anterior al
  confirmar, sin eliminar historia ni revelar otro expediente en un conflicto.
- Cierre comprobado al confirmar cargas, nuevas versiones, clasificación,
  sellado y mutaciones de participantes. Un sello preparado antes del cierre
  es rechazado sin estado ni evento de éxito. Los recursos ajenos conservan su
  `404`; lectura, historia, verificación, evidencia y asignaciones Owner siguen
  disponibles. Reabrir no altera participantes archivados ni el registro de etapa.
- READ COMMITTED explícito aun con otro aislamiento predeterminado. Un ensayo
  SQL directo espera a otro escritor y vuelve a comprobar los identificadores
  recién confirmados. El rojo de visibilidad bajo REPEATABLE READ y su corrección
  quedaron reproducidos. Otro ensayo detectó un año fuera de rango UTC que la
  conversión local ocultaba; el guard ahora evalúa el instante en UTC.
- Reaplicación de migraciones, UTF8, restricciones, referencias diferidas,
  privilegios por columna y tablas, inventario y restauración real con
  `search_path` vacío. El primer import rechaza administración/etapas ocupadas;
  conciliar un recibo existente tolera revisiones válidas posteriores.

La campaña HTTP recuperó **5 expedientes, 11 revisiones administrativas y 2
registros iniciales de etapa**, además de **5 raíces documentales, 6 versiones,
3 revisiones de clasificación, 2 participantes y 6 revisiones del directorio**.
Comparó filas completas, detalles e historias, autores originales y ZIP de las
dos versiones byte por byte. Conciliar el recibo después de la administración
no alteró las filas ni la cadena.

El prefijo importado de este ensayo contiene **63 eventos y 4 documentos**;
no es el total de eventos después de todos los recorridos. Los 51 eventos del
corte anterior permanecen como medición histórica. El formato legacy se
reconstruye como fixture documental sobre baselines administrativos explícitos,
sin modificar raíces ni historia de la fuente; no representa una conversión
íntegra del historial administrativo actual. El respaldo/restauración completo
posterior sí conserva todas las tablas.

El primer intento de la campaña HTTP fue rechazado porque su fixture de carga
clasificada omitía `tags`. Se corrigió el guion y se reprodujo el recorrido
completo con código de salida 0. La primera regresión de infraestructura detectó
una semilla de prueba que atravesaba esquemas anteriores y nuevos sin distinguir
la columna de baseline; se corrigió el helper administrativo. Estos fallos de
los guiones no se presentan como defectos de la aplicación.

### Cobertura de la administración penal

**12 613 de 13 506 líneas cubiertas: 93.4 %.** Los tres crates sujetos al umbral
del 90 % pasan el gate versionado.

| Crate | Líneas cubiertas / instrumentadas | Cobertura |
| --- | ---: | ---: |
| `domain` | 1497 / 1531 | 97.8 % |
| `application` | 2817 / 2950 | 95.5 % |
| `infrastructure` | 5739 / 6066 | 94.6 % |
| `web` | 1726 / 1863 | 92.6 % |
| `bin` | 834 / 1096 | 76.1 % |

La cobertura es de líneas instrumentadas, no una medida de cumplimiento de
objetivos ni de validez jurídica. La suite del corte anterior y su 92.9 %
permanecen separados a continuación.

### Qadra y navegador en la administración penal

| Comprobación | Resultado reproducido |
| --- | --- |
| Pruebas unitarias web | **45 aprobadas**, sin omisiones. |
| Navegador con HTTP simulado | **88 aprobadas**, 35.0 s. |
| QA móvil dirigida con el estilo final | **1 aprobada**. |
| Navegador con Rust/PostgreSQL/Redis/TSA reales | **4 aprobadas**, 1.3 min. |
| `npm run build`, `npm run format:check` | Aprobadas; build de 1.56 s. |
| Revisión de fuentes/configuración de esta entrega | 138 archivos ASCII, todos menores de 400 líneas; máximo 382. |
| Originales del sistema de diseño | 7 CSS y 5 archivos de marca/procedencia idénticos byte por byte. |

Los recorridos prueban alta penal completa; edición básica y completar R0/R1
sin inventar etapa; consulta e historia; filtros por cabezas; cuatro roles;
revocación; cierre durante una carga y conservación del archivo/borrador;
reapertura y persistencia después de iniciar otra sesión. Las respuestas tardías
de consulta, cambio de expediente y denegación se descartan. Edición y estado
permiten consultar y comparar explícitamente la cabeza después de un conflicto
o un resultado incierto. El alta incierta busca los identificadores enviados,
incluyendo casos cerrados, y permite abrir una coincidencia sin atribuirle
automáticamente el éxito de aquel POST ni volver a enviarlo.

Se inspeccionaron resumen, historial, edición en conflicto y cierre en escritorio
y móvil. La corrección visual del textarea reutiliza fuente, borde, radio y foco
de Qadra en una hoja adicional. La suite simulada completa precede solo a ese
ajuste CSS; la QA móvil y los cuatro recorridos reales usan el estilo final.
Los ZIP históricos antes y después de cerrar el expediente son idénticos.

Las evidencias anteriores son locales. La usabilidad con personal real, las
transiciones/adopción de etapa, participantes con identidad tipificada, audiencias,
plazos, firma individual, informes y preparación operativa siguen pendientes
según [el alcance del producto](product-completion.md). El guard recorre
metadatos y el listado hace consultas adicionales acotadas por página; no se
presentan estos ensayos como medición de capacidad de producción. La TSA local
sigue siendo evidencia técnica, sin constancia de un PSC autorizado.

### Seguimiento de sincronización del navegador en CI

El 15 de septiembre de 2026 UTC, una ejecución de navegador agotó su espera al
buscar el expediente básico, mientras la ejecución paralela del mismo código
aprobó. El guion podía continuar con un PUT pendiente porque esperaba un título
que ya estaba visible; también podía enviar el filtro mientras seguía pendiente
la consulta inicial del índice. El job fallido no conservó capturas ni contexto,
por lo que no se atribuye retrospectivamente una respuesta HTTP específica.

Dos pruebas nuevas retienen explícitamente las respuestas para reproducir esos
órdenes. Ambas fallaron antes de corregir los helpers. Una consulta filtrada
superpuesta recibe `503` en el ensayo controlado: reproduce un camino compatible
con la admisión limitada, sin afirmar que fue la respuesta observada en CI.
El guion corregido espera el PUT del expediente exacto, su revisión confirmada y
el cierre del editor; las búsquedas esperan que termine la lectura precedente y
comprueban la respuesta `200`. No se cambió la aplicación, su concurrencia ni
el tiempo máximo de prueba.

Las dos regresiones aprobaron después de la corrección. La suite simulada
completa pasó **90 pruebas en 32.7 segundos**, incluido ese par nuevo. Los
**cuatro recorridos con servicios reales aprobaron en 57.4 segundos**, con
18.8 segundos para administración penal, y el formato aprobó. La revisión final
comprueba 140 fuentes/configuraciones ASCII menores de 400 líneas; las 337
fuentes Rust, migraciones y manifiestos conservan los hashes usados en las suites
globales. El workflow ahora conserva PNG y contexto de los fallos de
navegador durante siete días. Las 88 pruebas y el PDF del corte anterior
conservan su condición de evidencia histórica; las fuentes del manuscrito no
cambiaron por esta corrección del guion.

### Coordinación de consultas y acciones documentales

La siguiente ejecución de CI sobre `e01d1f4` terminó con los controles Rust,
web simulado y documento aprobados, pero fallaron ambos recorridos de navegador
con servicios reales: uno al descargar y otro al verificar después del sellado.
Los contextos conservados muestran el documento sellado y una alerta genérica
para errores del servidor. No contienen el estado HTTP ni su código, por lo
que no prueban retrospectivamente una respuesta `503`.

La revisión del flujo identificó una carrera productiva: confirmar el sello
iniciaba consultas de listado e historial sin esperar su finalización y liberaba
las acciones de la ficha. Con los dos trabajadores ocupados, una nueva operación
puede ser rechazada por la admisión del servidor. El mismo patrón aparece al
montar los lectores de una carga y al refrescar una nueva versión. Esta causa
comprobable en la aplicación se distingue de las esperas incorrectas del guion
corregidas en el seguimiento anterior.

La ficha documental ahora espera la consulta de listado antes de montar sus
lectores iniciales; el estado ocupado comprende las operaciones de contenido,
clasificación y sus recargas dependientes. Sellar, verificar y descargar se
coordinan con esas consultas. Una nueva versión mantiene la selección ocupada
hasta terminar listado e historial. Si una consulta posterior falla de forma
transitoria, el sello confirmado permanece visible; un `403/404` retira los
recursos protegidos. No se repite automáticamente una mutación.

La edición de participantes conserva el historial abierto y lo refresca
explícitamente junto con el listado. Tanto la confirmación como la revisión de
un conflicto esperan ambas lecturas, aunque terminen en distinto orden. Cambiar
el estado organizativo conserva el comportamiento de retirar el detalle y espera
el listado. Los borradores y las revisiones esperadas se mantienen; una lista
tardía no restaura datos retirados por denegación.

Las nuevas pruebas retienen respuestas de operaciones y consultas, comprueban
controles deshabilitados y luego liberan cada respuesta explícitamente. Hay
**19 casos nuevos de navegador**: 12 documentales y 7 de participantes. Las
regresiones dirigidas reprodujeron acciones prematuramente habilitadas antes de
los cambios. El ensayo de append ajustó su preparación para esperar los lectores
iniciales; su comprobación posterior con el guard desactivado fue un control
negativo de sensibilidad, no una ejecución exacta contra una revisión anterior.
La prueba que antes editaba mientras seguía pendiente un GET de clasificación
ahora exige que termine esa consulta y después confirma la revisión nueva.
Permanecen las pruebas de respuestas tardías entre documentos y sesiones.

| Comprobación final del 15 de septiembre de 2026 | Resultado reproducido |
| --- | --- |
| Pruebas unitarias web | **48 aprobadas**, sin omisiones. |
| Navegador con HTTP simulado | **109 aprobadas**, 36.5 s. |
| Navegador con Rust/PostgreSQL/Redis/TSA reales | **4 aprobadas**, 57.6 s. |
| `npm run build`, `npm run format:check` | Aprobadas; build de 1.45 s sin advertencias. |
| Revisión de fuentes/configuración de la entrega | 156 archivos ASCII menores de 400 líneas; máximo 382. |
| Fuentes Rust y originales Qadra | 337 hashes Rust/migraciones/manifiestos y 12 originales de diseño sin cambios. |

El recorrido real de administración penal tardó 18.6 s; participantes, 17.5 s;
clasificación, 11.3 s; y versiones, 6.1 s. El tiempo total incluye trabajo del
runner fuera de los escenarios. El guion penal espera que la nueva versión
termine sus recargas antes de navegar a participantes; no usa solamente la
aparición anticipada del nombre del archivo como señal de finalización.

El diagnóstico de las pruebas reales registra únicamente método, ruta con UUID
sustituidos, estado HTTP y código de error validado en `api-failures.json`.
Excluye query strings, cabeceras y cuerpos completos; tres pruebas unitarias
cubren su extracción y sanitización. CI conserva este archivo con las capturas
y el contexto de un recorrido fallido durante siete días. En la ejecución final
se registraron siete errores esperados: cinco conflictos `409` y dos `404` tras
revocaciones; no hubo respuestas `5xx` ni `server_busy`. Este resultado describe
esos cuatro escenarios y no se extrapola a cualquier carga concurrente.

Se revisaron las capturas finales de la ficha móvil, el conflicto de clasificación
en escritorio y el resumen penal móvil; el contenido y los controles son
legibles. Las capturas anteriores se conservaron. La revisión independiente de
las promesas, los estados ocupados y el descarte de respuestas tardías no encontró
otros defectos dentro del alcance corregido.

La corrección coordina operaciones y consultas dependientes dentro de las fichas.
No cambia los límites del backend, añade reintentos automáticos ni demuestra
capacidad de producción. Cambiar de pantalla o lanzar consultas manuales mientras
siguen pendientes otras operaciones conserva la admisión compartida del servidor.
Las suites Rust de 770 pruebas y su cobertura del 93.4 % no se repitieron en este
seguimiento: sus fuentes siguen idénticas a las comprobadas en el corte anterior.

## Corte reproducido: directorio de participantes

- Fecha local: 14 de septiembre de 2026 (`America/Mexico_City`); registros UTC
  correspondientes al 15 de septiembre.
- Alcance: fichas por expediente, revisiones con autoría, consulta autorizada,
  filtros, archivo/reactivación, conflictos y recuperación, con interfaz Qadra.
- Decisión: [directorio auditado](adr/0021-audited-case-participants.md).
- Entorno reproducido: Rust/Cargo 1.94.0, PostgreSQL 18.6, Valkey 8.1.9,
  OpenSSL 3.5.7, Node.js 22.22.2 y npm 10.9.7.

### Backend y recuperación de participantes

| Comprobación | Resultado reproducido |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo build --workspace` | Aprobadas. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobada. |
| Política de dependencias con `cargo-deny 0.20.2` | Aprobada, sin nuevas excepciones. |
| `bash scripts/test-backends.sh` | **704 aprobadas**, 0 fallidas y 1 externa ignorada. |
| Suite instrumentada con `cargo llvm-cov --workspace` | **704 aprobadas**, 0 fallidas y 1 externa ignorada. |
| `bash scripts/demo.sh` | Recorrido criptográfico CLI aprobado. |
| `bash scripts/api-demo.sh` | Permisos, conflictos, versiones, clasificación, participantes, importación y restauración aprobados. |

Las instancias PostgreSQL/Redis fueron desechables; las pruebas de participantes
usaron esquemas aislados en la base de expedientes, con rol operativo y contraseña
SCRAM explícitos. La prueba ignorada es `the_real_sandbox_issues_a_token`; no se
ensayó un proveedor externo. La advertencia de compatibilidad futura de
`redis 0.25.4` permanece, sin errores de Clippy.

La revisión de dependencias detectó `chacha20 0.10.1` retirado del registro,
heredado a través del generador usado por el cliente PostgreSQL. Se actualizó
exclusivamente a `0.10.2`: el [registro de cambios de RustCrypto](https://github.com/RustCrypto/stream-ciphers/blob/master/chacha20/CHANGELOG.md)
documenta una corrección de instrucciones SSE4.1 usadas en el backend SSE2 de
RNG y variantes de contador de 64 bits. La validación global, instrumentada y
las demostraciones finales usan ese lockfile. No se presenta como un aviso
RUSTSEC nuevo ni como un cambio de los algoritmos del prototipo.

Las **60 pruebas Rust nuevas** respecto de clasificación se distribuyen en 21
pruebas de dominio/aplicación, 12 del adaptador HTTP y 27 de PostgreSQL e
importación. Cubren:

- Validación previa al recorte de blancos, límites en escalares Unicode,
  opcionales vacíos, homónimos, signos, identidad independiente y canon `PART1`.
  Vectores fijos y el máximo de 2584 bytes coinciden entre Rust y SQL.
- Permisos Owner/Litigator/Paralegal/Client, membresía vigente, recursos ajenos,
  revalidación de rol, actividad y asignación tras esperar el bloqueo de auditoría.
- Creación con raíz y revisión activa inicial obligatoria; reemplazo completo
  y cambio exclusivo de estado con revisión esperada y autoría capturada. No
  se realiza GET después de confirmar la mutación para construir su respuesta.
- Rechazo sin cambios parciales ante fallos de fila, evento y commit diferido;
  lecturas, listado e historia no entregan datos si no pueden confirmar auditoría.
- Un solo ganador entre edición y archivo concurrentes; ambos órdenes de
  confirmación y reintento explícito del estado conservando los textos actuales.
  SQL vuelve a leer el predecesor después de commit o rollback de otro escritor.
- Cabezas actuales antes de filtros literales y paginación UUID; historia con
  cursor descendente exclusivo, incluyendo autores anteriores tras cambiar su
  perfil. Los valores persistidos corruptos no se normalizan silenciosamente.
- JSON estricto, suplantación de contexto/actor rechazada, revisiones cero
  distinguidas de errores sintácticos y cuerpo completo limitado a 8 KiB.
  Los textos máximos caben aun con caracteres astrales escapados.
- Restricciones, privilegios, UTF-8, continuidad, reaplicación de migración y
  restauración real con `pg_dump`/`pg_restore` y `search_path` vacío.

El ensayo de agotamiento usa una modificación administrativa deliberada para
alcanzar `u32::MAX` sin crear miles de millones de filas. Comprueba rechazo sin
vuelta a cero ni nuevo evento y verifica aparte que el arranque rechaza ese
inventario discontinuo; no demuestra que sea válido en operación.

Una regresión reprodujo que el importador inicial aceptaba un destino con fichas
insertadas directamente sin eventos. Ahora comprueba ambas tablas de participantes
como estado ocupado antes del primer import. La conciliación de un recibo
existente conserva los participantes válidos creados después y el prefijo legacy.

El recorrido HTTP final importa cuatro documentos y conserva 51 eventos como
prefijo, agrega V2 y clasificación, y crea **dos fichas con seis revisiones**.
Comprueba homónimos, cuatro roles, retiro de asignación con sesión existente,
una carrera con un ganador y un solo evento, archivo con rechazo obsoleto y
reactivación. El respaldo contiene además **cinco raíces documentales, seis
snapshots y tres revisiones de clasificación**. La restauración compara todas
las filas, actores, fechas, auditoría y recibos antes de abrir el servidor.
Listado e historial de participantes son idénticos; los ZIP V1/V2 documentales
siguen siendo idénticos y verificables con OpenSSL. Los 51 eventos identifican
solo el prefijo original, no el total final.

### Cobertura del directorio

| Crate | Líneas cubiertas / totales | Cobertura |
| --- | --- | --- |
| domain | 1279 / 1313 | 97.4 % |
| application | 2632 / 2765 | 95.2 % |
| infrastructure | 4888 / 5193 | 94.1 % |
| web | 1361 / 1474 | 92.3 % |
| bin | 834 / 1093 | 76.3 % |
| **Total** | **10994 / 11838** | **92.9 %** |

Los tres crates con umbral obligatorio superan 90 %. La cobertura mide líneas
Rust ejecutadas por la suite; las demostraciones externas y el navegador tienen
resultados separados. El porcentaje no acredita identidad jurídica, usabilidad
ni el cumplimiento completo del catálogo procesal.

### Interfaz Qadra y servicios reales

| Comprobación | Resultado reproducido |
| --- | --- |
| `npm test` en `web/` | **38 pruebas unitarias aprobadas**. |
| `npm run test:e2e -- --workers=1` | **65 pruebas aprobadas** con HTTP simulado, en 24.6 s. |
| `bash scripts/web-demo.sh` | **3 recorridos aprobados** con servicios reales, en 39.4 s. |
| `npm run build` y `npm run format:check` | Aprobadas. |

El nuevo recorrido real abre dos sesiones independientes y comprueba alta,
edición concurrente, archivo y reactivación, revisiones R1 a R8, filtros e
historial persistido. Ambos conflictos conservan la intención del usuario y
requieren consultar la revisión vigente y confirmar el reintento. Cambiar solo
el estado conserva los textos más recientes. Después de reingresar, Litigator
edita y Paralegal consulta; Client no solicita rutas de participantes. Retirar
la asignación con la misma sesión activa deniega y limpia los datos visibles.
Los ZIP documentales anteriores y posteriores al cambio siguen siendo idénticos.
Los otros dos recorridos reales mantienen versiones y clasificación documental.

Las regresiones simuladas cubren respuestas tardías, cambio de expediente,
sesión terminada, contexto de respuesta incorrecto, denegaciones y agotamiento
de revisiones. Una regresión falló antes de corregir la actualización de filas
observadas: después de conocer una revisión nueva se repite la consulta filtrada
en el servidor, conservando por separado el detalle y el borrador. Otras dos
aserciones reprodujeron el aviso de conflicto obsoleto después de consultar
los datos actuales; ahora la pantalla solicita confirmar sobre esa nueva base.

Dos ejecuciones reales previas detectaron defectos del guion de prueba: recargar
antes de terminar el logout y contar módulos JavaScript como solicitudes a la
API. El guion espera el formulario de acceso y captura exclusivamente rutas
`/api/v1`. La corrida completa final aprobó después de esas correcciones.

La revisión visual final cubre directorio y detalle de escritorio, historial
móvil y diálogos de conflicto y confirmación: textos legibles, controles visibles
y sin desbordamiento observado. Los siete CSS originales y los cinco archivos
de marca y procedencia coinciden byte a byte con su fuente original. Los estilos
nuevos se incorporan en `web/src/styles/participants.css`. Las 91 fuentes y
configuraciones web revisadas cumplen ASCII y menos de 400 líneas.

Estas comprobaciones no sustituyen pruebas de usabilidad con personas ni una
campaña de carga. El directorio registra información manual; quedan pendientes
la identidad jurídica tipada y verificada, los criterios judiciales de
certificados y duplicidad, y el perfil y estado procesal del expediente. El
[inventario funcional](product-completion.md) conserva esos criterios abiertos.

## Corte reproducido: clasificación documental auditada

- Fecha local: 14 de septiembre de 2026 (`America/Mexico_City`); registros UTC
  correspondientes al 15 de septiembre.
- Alcance: carga clasificada atómica, revisiones organizativas independientes
  del contenido, filtros exactos, concurrencia, restauración e interfaz Qadra.
- Decisión: [clasificación auditada](adr/0020-audited-document-classification.md).
- Entorno: Rust y Cargo 1.94.0, PostgreSQL 18.6, Valkey 8.1.9 compatible con
  Redis, OpenSSL 3.5.7, Node.js 22.22.2 y npm 10.9.7.

### Backend, restricciones y recuperación

| Comprobación | Resultado reproducido |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo build --workspace` | Aprobadas. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobada. |
| Política de dependencias con `cargo-deny 0.20.2` | Aprobada, sin nuevas excepciones. |
| `bash scripts/test-backends.sh` | **644 aprobadas**, 0 fallidas y 1 externa ignorada. |
| Suite instrumentada con `cargo llvm-cov --workspace` | **644 aprobadas** y 1 externa ignorada. |
| `bash scripts/demo.sh` | Recorrido criptográfico CLI aprobado. |
| `bash scripts/api-demo.sh` | Roles, aislamiento, concurrencia, carga clasificada, versiones, importación y restauración aprobados. |

PostgreSQL y Redis fueron instancias desechables con bases separadas de identidad,
expedientes y documentos. El guion de pruebas usa SCRAM y contraseñas aleatorias.
La prueba ignorada sigue siendo `the_real_sandbox_issues_a_token`; no se ejecutó
una campaña con un proveedor externo. Permanece la advertencia de compatibilidad
futura de `redis 0.25.4`, sin errores de Clippy.

Las 67 pruebas Rust nuevas respecto de versiones cubren:

- Canon `DMETA1`, escalares Unicode, controles, blancos, deduplicación, comas y
  orden UTF-8, con comparación de vectores entre Rust, SQL y OpenSSL. La base
  rechaza valores no canónicos, arrays anómalos, digest falso y codificación
  distinta de UTF-8 antes de aplicar la migración.
- Revisión cero sin procedencia inventada, reemplazo completo, valores vacíos,
  revisiones idénticas, agotamiento y cursor descendente exclusivo. Historial
  conserva correo y UUID capturados aunque cambie el perfil del actor.
- Autorización antes de consultar o preparar y dentro de la transacción;
  Client denegado, asociación ajena oculta, revocación de rol/asignación y
  revalidación después de esperar a otro escritor.
- Carga inicial con raíz, V1, R1 y dos eventos atómicos; rollback ante fallo de
  inserción, del segundo evento o de commit diferido. Fallos de lectura o
  historial no entregan valores sin confirmar auditoría.
- Reemplazos concurrentes con un solo ganador, append de contenido independiente,
  filas inmutables y lecturas del máximo después de esperar commit o rollback.
- Selección de la última versión y clasificación antes de filtrar y paginar;
  tipos, clasificación y etiquetas exactos sin reutilizar valores históricos.
- HTTP con JSON estricto, partes completas en cualquier orden, nombre de cabecera
  autoritativo, límites por parte y totales, y cuerpos fragmentados sin
  Content-Length. El detalle actual incluye clasificación; historia y acciones
  exactas de contenido conservan sus proyecciones y evidencia.
- Migración aditiva, reaplicación, snapshot legacy 7 seguido por 8, inventario,
  permisos y restauración de filas completas con comprobaciones activas.

Una prueba de transporte con fragmentos diferidos reprodujo que el parser podía
aceptar bytes que excedían el límite después del cierre multipart. La corrección
consume primero el cuerpo completo acotado y rechaza exceso o interrupción antes
de preparar la carga. Las pruebas cubren ambos límites exactos y el epílogo.

El demo de recuperación reprodujo otro defecto: `pg_restore` usa un search path
vacío y las funciones canónicas no encontraban sus auxiliares. La migración ahora
califica referencias al esquema instalado. Dos regresiones fallaron antes del
cambio; después aprobaron tanto el CHECK bajo search path vacío como un
`pg_dump`/`pg_restore` real, conservando restricciones y acceso del rol operativo.

El recorrido HTTP importa cuatro documentos y conserva el prefijo de 51 eventos;
añade V2 a uno y carga otro con clasificación inicial. Reemplaza R0 por R1 y
vacía valores en R2, rechaza la revisión obsoleta y comprueba filtros e historial.
El respaldo contiene **cinco raíces, seis snapshots y tres revisiones de
clasificación**. La restauración compara filas completas de documentos, raíces,
clasificación, actores capturados, usuarios, expedientes, asignaciones, auditoría
y recibos. Las dos historias JSON y los ZIP V1/V2 resultan idénticos; OpenSSL
verifica el contenido y evidencia. Los 51 eventos son el prefijo preservado,
no el total final después del recorrido.

### Cobertura de clasificación

| Crate | Líneas cubiertas / totales | Cobertura |
| --- | --- | --- |
| domain | 1146 / 1180 | 97.1 % |
| application | 2426 / 2559 | 94.8 % |
| infrastructure | 4358 / 4646 | 93.8 % |
| web | 1090 / 1199 | 90.9 % |
| bin | 834 / 1086 | 76.8 % |
| **Workspace** | **9854 / 10670** | **92.4 %** |

La medición usa `scripts/test-backends.sh cargo llvm-cov --workspace --json
--summary-only --output-path /tmp/tt-classification-coverage-final2.json`;
`scripts/coverage-gate.sh` comprueba los tres umbrales obligatorios de 90 %.
El corte anterior de versiones, con 577 pruebas y 91.7 %, se conserva abajo.

### Qadra y navegador

Formato y build web aprobados; **34 pruebas unitarias**, **47 pruebas de
navegador con respuestas simuladas** y **dos recorridos con servicios reales**
aprobados. Los mocks y el servidor live se ejecutaron secuencialmente. Las nuevas
regresiones comprueban valores Unicode y comas, límites, texto tratado como texto,
conflicto con borrador conservado, agotamiento, R0, limpieza explícita, filtros,
respuestas tardías, denegaciones y separación respecto de la versión seleccionada.

El recorrido nuevo real crea R1 en una sola carga multipart, edita R2, provoca
un conflicto desde dos contextos autenticados y conserva el formulario ante R3.
Tras comparar, confirma R4; añadir V2 conserva esa clasificación. Vaciar los
campos genera R5 y mantiene las cinco revisiones con autor capturado. Reingreso,
filtros vigentes y selección histórica siguen funcionando. Los ZIP de ambas
versiones permanecen idénticos tras editar y vaciar clasificación. El otro
recorrido conserva la validación real de versiones de contenido.

Se inspeccionaron escritorio, móvil, historial desplegable y comparación del
conflicto con confirmación visible. La clasificación actual aparece separada de
las versiones; las fechas se presentan en español de México con su valor UTC
conservado en el elemento `time`. Las siete hojas CSS originales y los recursos
de marca de Qadra se compararon byte por byte sin cambios; las ampliaciones usan
`web/src/styles/metadata.css` y componentes basados en los controles existentes.

La clasificación no completa por sí sola todos los casos documentales: siguen
abiertos la política de formatos admitidos, entrega íntegra sin sello y alertas
de alteración al Owner. Gestión procesal, identidad/firma individual, informes,
operación pública y usabilidad con personas reales conservan sus pendientes en
[el alcance del producto](product-completion.md). Este ensayo no prueba carga de
producción ni añade autoridad jurídica a la TSA local.

## Corte reproducido: versiones documentales inmutables

- Fecha local: 14 de septiembre de 2026 (`America/Mexico_City`); registros UTC
  correspondientes al 15 de septiembre.
- Alcance: append optimista, historial y operaciones de versión exacta,
  migración de snapshots existentes, reconciliación, recuperación e interfaz Qadra.
- Entorno: Rust y Cargo 1.94.0, PostgreSQL 18.6, Valkey 8.1.9 compatible con
  el protocolo Redis, OpenSSL 3.5.7, Node.js 22.22.2 y npm 10.9.7.

### Backend y recuperación

| Comprobación | Resultado reproducido |
| --- | --- |
| `cargo fmt --all`, `cargo build --workspace` | Aprobadas. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Aprobada. |
| Política de dependencias con `cargo-deny 0.20.2` | Avisos, restricciones, licencias y fuentes aprobados; sin nuevas excepciones. |
| `bash scripts/test-backends.sh` | **577 aprobadas**, 0 fallidas y 1 externa ignorada; PostgreSQL y Redis desechables. |
| Suite instrumentada con `cargo llvm-cov --workspace` | Las mismas **577 aprobadas** y 1 externa ignorada. |
| `bash scripts/demo.sh` | Recorrido CLI aprobado, incluyendo rechazos esperados ante alteración. |
| `bash scripts/api-demo.sh` | Roles, aislamiento, revocación, concurrencia, importación, versiones y restauración aprobados. |

La prueba ignorada sigue siendo `the_real_sandbox_issues_a_token`. No se
configuraron credenciales de un proveedor externo. Los adaptadores PostgreSQL
sí se ejercitaron: el guion provee bases separadas para identidad, expedientes
y documentos. La advertencia de compatibilidad futura de `redis 0.25.4` permanece;
no es un fallo de Clippy ni una migración a clientes asíncronos.

Las nuevas regresiones comprueban:

- Validación de consultas, autorización previa, reautenticación tras preparar
  contenido, rechazo de una cabeza obsoleta y agotamiento del número de versión.
- Migración de filas existentes con primeras versiones 1 y 7, conservación de
  vault/evidencia/prefijo de auditoría y repetición administrativa tras añadir 8.
- Raíces sin versión inicial rechazadas al commit, asociación exacta a expediente,
  secuencia contigua y privilegios operativos sin UPDATE sobre raíces.
- Cuatro appends concurrentes con un solo ganador, sellado de una versión
  histórica conservada y trigger SQL que vuelve a leer tras esperar un commit
  o rollback competidor.
- Rollback ante fallos de inserción, auditoría y restricciones diferidas;
  denegación de historial si no se confirma su evento.
- Tres snapshots cifrados con AES-GCM real, dos de contenido idéntico, con DEK
  distinta por versión y rechazo de sustitución de UUID, versión, vault o digest.
- Consultas que eligen la versión actual antes de aplicar filtros, historial
  descendente, Client denegado y casos ajenos indistinguibles de inexistentes.
- Reconciliación del snapshot legacy original tras nuevos appends y sellado;
  rechazo de esquema incompleto, secuencias incompletas y privilegios excesivos,
  incluidos roles asumibles mediante SET ROLE.

El demo mantiene cuatro documentos importados y 51 eventos históricos. Después
de importar añade una segunda versión a uno de ellos, rechaza un número esperado
obsoleto y las rutas sin versión ambiguas, sella/verifica la revisión nueva y
vuelve a exportar la primera. El respaldo contiene cinco snapshots de cuatro
documentos. La restauración compara raíces, snapshots, usuarios, expedientes,
asignaciones, auditoría y recibos; también conserva idénticos los ZIP de ambas
versiones y verifica su contenido con OpenSSL. El contador de 51 identifica el
prefijo preservado, no el total final de eventos después de ese recorrido.

El guion de concurrencia se actualizó para contar el recurso auditado con
versión y digest: una ejecución inicial llegó al sellado correcto y rechazó
su aserción anterior, que buscaba el formato sin versión. Tras corregir esa
consulta, el recorrido completo aprobó con un sellado, un conflicto y un evento.

CI detectó además tres fixtures nuevos que creaban roles sin contraseña y
dependían de la autenticación `trust` del entorno local. Se reprodujo el fallo
`28P01` en PostgreSQL aislado con SCRAM, se corrigieron las credenciales de esos
roles de prueba y aprobaron sus seis escenarios. Una contraseña incorrecta fue
rechazada en ese entorno. `scripts/test-backends.sh` ahora crea su instancia
desechable con SCRAM y una contraseña administrativa aleatoria, sin cambiar
servidores existentes. Tras la corrección, formato, compilación, Clippy, suite
completa e instrumentada aprobaron nuevamente: 577 pruebas, una externa ignorada
y los mismos numeradores de cobertura que se presentan abajo.

La adaptación a SCRAM también expuso una suposición en `scripts/web-demo.sh`:
su sustitución textual de URL solo admitía conexiones sin usuario y contraseña.
Se reprodujo el rechazo de conexión antes de iniciar el navegador. El guion
ahora crea una contraseña aleatoria propia del rol de prueba y reemplaza las
credenciales mediante `URL`, conservando host, puerto, base y parámetros.
El recorrido completo del navegador volvió a aprobar contra los servicios
desechables con SCRAM: una prueba real, sin cambios de interfaz. También se
comprobó la construcción de URL con credenciales existentes, IPv6, parámetros
y caracteres reservados en la contraseña.

### Cobertura de versiones

| Crate | Líneas cubiertas / totales | Cobertura |
| --- | --- | --- |
| domain | 1045 / 1079 | 96.8 % |
| application | 2319 / 2452 | 94.6 % |
| infrastructure | 3889 / 4175 | 93.1 % |
| web | 821 / 927 | 88.6 % |
| bin | 834 / 1086 | 76.8 % |
| **Workspace** | **8908 / 9719** | **91.7 %** |

`scripts/coverage-gate.sh` aprobó los tres umbrales obligatorios del 90 %.
La medición anterior de consultas, 539 pruebas y 91.0 %, se conserva más abajo
con su alcance; no se sustituyen sus cifras por las de este cambio.

### Qadra y navegador

Formato y build web aprobados; **28 pruebas unitarias** y **32 pruebas de
navegador con HTTP simulado** aprobadas. Cubren cursor de historial, acciones
exactas, conflictos con archivo retenido, agotamiento sin refresco engañoso,
actualización de la fila actual y descarte de respuestas tardías. Una denegación
de historial o detalle retira sus datos y la opción de añadir versiones.
La revisión independiente detectó los casos de agotamiento y denegación de
detalle; sus regresiones fallaron antes de aplicar las correcciones.

`scripts/web-demo.sh` aprobó **un recorrido con servicios reales**: sesión,
expediente, versión 1 sellada y ZIP, versión 2 con nombre/contenido diferentes,
sellado/verificación/ZIP de la segunda, selección histórica y ZIP de la primera
idéntico al original. Al cerrar sesión, recargar y entrar de nuevo recuperó
ambas versiones. No hubo errores JavaScript y se comprobaron vistas móviles.
Este escenario no intercepta HTTP.

Las pruebas simuladas usan un puerto propio y no reutilizan un servidor
encontrado en el puerto predeterminado. Astro impide dos servidores del mismo
proyecto aun con puertos distintos; una tentativa simultánea de mocks y live
falló por ese bloqueo. Las verificaciones finales se ejecutaron secuencialmente.
No se detuvieron servidores de otros proyectos.

Se conservan la marca, tokens y hojas de estilo originales de Qadra. Los
componentes de historial y carga de revisiones usan las clases existentes y
`web/src/styles/versions.css`. Clasificación, actividad procesal, firma personal,
autenticación por certificado, mediciones de producción y usabilidad con personas
reales siguen pendientes en [el alcance de cierre](product-completion.md).

## Corte reproducido: consultas documentales e integración Qadra

- Fecha local: 14 de septiembre de 2026 (`America/Mexico_City`).
- Alcance: listado y detalle documental autorizados, búsqueda literal de nombre,
  filtro de sellado y conexión del sistema de diseño Qadra a expedientes reales.
- Decisión: [consultas de metadatos](adr/0018-authorized-document-queries.md).
  El [plan de cierre](product-completion.md) conserva las funciones pendientes.

### Backend y servicios reales

```bash
cargo fmt --all
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/test-backends.sh
bash scripts/api-demo.sh
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-document-queries-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-document-queries-coverage.json
```

La suite completa y su ejecución instrumentada aprobaron **539 pruebas**, sin
fallos y con una ignorada del proveedor externo. PostgreSQL y Redis fueron
reales y desechables, con bases separadas de identidad, expedientes y documentos.
Formato, compilación, Clippy, demostración HTTP y umbrales aprobaron.

Las veinte pruebas nuevas cubren validación del filtro, permisos de lectura,
autenticación, aislamiento, orden y paginación, comodines tratados literalmente,
metadatos sin decodificar el contenido cifrado ni la evidencia, cambios de rol,
inactividad, revocación y fallo de auditoría. Una prueba concurrente bloquea la
inserción de auditoría: listado y detalle no devuelven resultados antes del
commit, y retirar la asignación espera ese orden.

La demo HTTP reproduce consultas de los cuatro roles, dos expedientes,
revocación, sellado concurrente con un éxito y un conflicto, y migración y
restauración de **cuatro documentos con 51 eventos**. Compara evidencia ZIP y
verifica sus componentes con OpenSSL. El corte anterior de 46 eventos conserva
su fecha; los eventos nuevos proceden de las consultas añadidas al ensayo.

La primera ejecución de la suite detectó una carrera preexistente en el fixture
TCP del adaptador remoto simulado: una prueba liberaba un puerto antes de probar
su inaccesibilidad y otro servidor de prueba podía reutilizarlo. Se reprodujo
la interferencia y se reemplazó por un servidor que recibe la petición y cierra
sin respuesta, reteniendo el puerto. El adaptador no cambió. La suite del stub
aprobó después y veinte repeticiones acotadas también; esos conteos no se suman
a las 539 pruebas de la suite completa.

### Cobertura reproducida

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1045/1079 | 96.8 % |
| `application` | 2138/2277 | 93.9 % |
| `infrastructure` | 3602/3892 | 92.5 % |
| `web` | 637/737 | 86.4 % |
| `bin` | 834/1086 | 76.8 % |

Total: **8256/9071 líneas (91.0 %)**. Los tres crates sujetos al umbral del 90 %
aprueban. La cobertura corresponde al workspace Rust, no a los archivos Svelte.

### Actualización de la dependencia TLS

El control de dependencias de CI detectó un aviso publicado el 14 de septiembre
para `rustls` 0.23.43. Se actualizó únicamente su versión y checksum en
`Cargo.lock` a 0.23.45, identificada como corregida en
[el aviso del proyecto](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc).
No se añadió ninguna excepción a la política de dependencias.

Después del cambio aprobaron nuevamente formato, build, Clippy, las 539 pruebas
Rust con servicios desechables y el escenario de navegador real. `cargo-deny`
0.20.2 aprobó advisories, bans, licenses y sources con la base actualizada. La
medición de cobertura de esta sección precede al ajuste del lockfile; no se
presenta como una nueva medición posterior. Las fuentes del reporte no cambiaron
por esta actualización de dependencia y su PDF permanece válido para el contenido
documental compilado.

### Interfaz y pruebas con HTTP simulado

La referencia Qadra se comprobó antes de adaptar sus flujos: 18 pruebas
unitarias y 14 de navegador, además de formato y compilación, aprobaron en
este entorno. Después de la integración aprobaron **21 pruebas unitarias y
23 de navegador con HTTP simulado**, junto con `npm run build` y
`npm run format:check` (Node.js 22.22.2). La automatización usa Node.js 24.

Las pruebas añaden expedientes persistentes, filtros y páginas solicitados al
servidor, permisos Client y descarte de resultados de una sesión, expediente,
búsqueda o detalle anteriores. Dos regresiones reproducidas antes de corregir
el código cubren una apertura que quedaba bloqueada al cambiar la búsqueda y
un detalle atrasado que reemplazaba la selección de una carga nueva.

La revisión visual conserva los originales de marca y las siete hojas de
estilo de Qadra. Las ampliaciones se concentran en `cases.css`. Se comprobaron
expedientes, lista y detalle en escritorio y móvil. Una prueba de geometría
verifica que buscador, botón y selector no se solapen a 390 píxeles; verificar
solo el ancho de la página no detectaba ese defecto de composición.

### Navegador con servicios reales

```bash
bash scripts/web-demo.sh
```

Un escenario Playwright aprobó con la API Rust, PostgreSQL y Redis aislados y
la TSA OpenSSL local. Desde la interfaz realizó login con recuperación MFA,
creación de expediente, carga de documento, detalle persistido, sellado,
verificación y descarga ZIP. El contenido descargado coincide byte por byte
con la muestra generada. Después de logout, recarga e inicio con otro código,
el expediente y el documento siguen disponibles desde consultas del servidor.
Se comprobó una vista de 390 píxeles sin desbordamiento horizontal ni errores
JavaScript. El script elimina servicios, claves y credenciales desechables.

La comprobación de navegador no intercepta HTTP. Las pruebas de UI con respuestas
simuladas se documentan por separado y no sustituyen este escenario real.
Usabilidad, carga de producción, versiones, clasificación, gestión procesal,
autenticación por certificado y firma por credencial individual siguen pendientes.
Los resultados de compilación y revisión académica están en
[la verificación del reporte](academic-report-verification.md).

## Corte reproducido: documentos por expediente y auditoría transaccional

- Fecha local: 12 de septiembre de 2026 (`America/Mexico_City`).
- Base: `f5d6716`; rama: `feat/document-case-authorization`.
- Alcance y decisiones: [criterios de entrega](next-goal.md),
  [ADR-0016](adr/0016-case-document-transactions.md) y
  [operación y restauración](database-operations.md).

### Verificación del estado final

```bash
cargo fmt --all
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/test-backends.sh
bash scripts/demo.sh
bash scripts/api-demo.sh
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-case-doc-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-case-doc-coverage.json
```

La suite completa final, ejecutada durante cobertura, aprobó **519 pruebas**,
sin fallos y con una ignorada del proveedor externo: 58 pruebas netas más que
el barrido anterior. PostgreSQL y Redis fueron reales y desechables, con bases
separadas de identidad, expedientes y documentos. Formato, build, Clippy,
demostraciones y umbrales terminaron con código cero. CI configura también
`DOCUMENT_TEST_DATABASE_URL`; no se cuentan retornos por variables ausentes
como ejercicio de esos adaptadores.

La demo HTTP reproduce cuatro roles, dos expedientes y revocación con la misma
sesión. Dos procesos del servidor compiten por sellar: uno obtiene `200`, otro
`409`, queda un evento de sellado y ambos entregan ZIP idénticos verificables
con OpenSSL. El ensayo posterior importa cuatro documentos (tres sellados),
conserva exactamente 46 entradas históricas, repite sin duplicación y restaura
un respaldo `pg_dump`/`pg_restore`; compara estado SQL y evidencia byte por byte
y verifica nuevamente firma, certificado/CRL y sello con OpenSSL.

Las regresiones incluyen rollback por fallo de inserción y commit, revocación
ordenada, escritores de auditoría concurrentes, barreras durables antes del
import, fallos de marcadores después del commit, restauraciones parciales,
recibos alterados y permisos PostgreSQL por propiedad, columna, esquema y roles
asumibles. Identidad retira desafíos/sesiones ante fallo de auditoría y conserva
revocaciones; no se afirma atomicidad distribuida con Redis. La validación de
cadena/CRL del firmante usa el instante del sello; TSA conserva la política
OpenSSL actual y puede rechazar una autoridad expirada hoy.

### Cobertura final

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1045/1079 | 96.8 % |
| `application` | 2073/2212 | 93.7 % |
| `infrastructure` | 3496/3785 | 92.4 % |
| `bin` | 834/1086 | 76.8 % |
| `web` | 578/683 | 84.6 % |

Total: **8026/8845 líneas (90.7 %)**. Pasan los tres umbrales obligatorios del
90 %. No se ensayó despliegue público ni carga de producción. Se conservaron
los 19 archivos locales protegidos del reporte, presentación y entregables;
no había un `runtime-data` local que migrar. Los ensayos usan sus propias fuentes.

Los cortes siguientes son históricos y conservan sus mediciones originales.

## Corte reproducido: barrido del backend

- Fecha local: 11 de septiembre de 2026 (`America/Mexico_City`).
- Base: `0cc6921`; rama de revisión: `feat/backend-hardening`.
- Alcance: defectos concurrentes, invariantes persistidos y claridad de las
  fronteras de identidad, almacenamiento y HTTP. Ver
  [el barrido](backend-review.md) y [ADR-0015](adr/0015-backend-concurrency-and-invariants.md).

### Verificaciones ejecutadas sobre el estado final

```bash
cargo fmt --all
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/test-backends.sh
bash scripts/demo.sh
bash scripts/api-demo.sh
```

La suite completa aprobó **461 pruebas**, sin fallos y con una ignorada del
proveedor de sellado externo. Son 35 pruebas netas adicionales frente al corte
de expedientes. Se usaron PostgreSQL y Redis desechables y bases separadas;
las pruebas de backends no se omitieron por ausencia de variables. Compilación,
formato, Clippy y ambas demostraciones terminaron con código cero.

Se reprodujeron fallos antes de corregir consumo concurrente de MFA, permisos
basados en identidades caducadas, controles en correo, deserialización inválida,
desbordamiento de versión, sobrescritura de evidencia, lectura parcial de la
bitácora y coordinación de migraciones. Las pruebas Redis cubren también un
contador heredado ya bloqueado sin TTL, ventanas que no se amplían y cuentas
sin contador que no crean claves al consultarse.

Las pruebas HTTP comprueban rechazo por saturación antes de leer el cuerpo,
permisos de trabajo retenidos después de cancelar la petición, recuperación de
capacidad tras fallo o cancelación y cabeceras/body de identidad estrictos.
La demostración integrada mantiene los cuatro roles, sesiones, expedientes,
revocación, evidencia documental y comprobación independiente con OpenSSL.

### Cobertura reproducida del barrido

```bash
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-hardening-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-hardening-coverage.json
```

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1039/1079 | 96.3 % |
| `application` | 1820/1950 | 93.3 % |
| `infrastructure` | 2467/2643 | 93.3 % |
| `bin` | 834/1045 | 79.8 % |
| `web` | 554/675 | 82.1 % |

Total: 6714/7392 líneas (90.8 %); los tres umbrales
obligatorios del 90 % aprobaron con PostgreSQL y Redis reales.

No se hizo una prueba de carga de producción ni se resolvieron TLS, pooling,
handshake Redis, transacción entre documento y bitácora, asociación documental
por expediente ni la amenaza de firma HTTP. Esos límites siguen explícitos en
[backend-review.md](backend-review.md) y [next-goal.md](next-goal.md).

Las fuentes y entregables locales del reporte y presentación se conservaron.
Los cortes que siguen son evidencia anterior y mantienen sus propios conteos.

## Corte reproducido: expedientes y asignaciones

- Fecha local: 11 de septiembre de 2026 (`America/Mexico_City`).
- Fecha UTC observada en las demostraciones: 12 de septiembre de 2026.
- Rama: `feat/case-membership`, basada en `cb1f79c` de `main`.
- Alcance: metadatos de expedientes, asignaciones y autorización por pertenencia.
  La decisión está en [ADR-0014](adr/0014-case-membership-and-isolation.md).

### Verificaciones ejecutadas

```bash
cargo fmt --all
cargo build --workspace
bash scripts/test-backends.sh
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/demo.sh
bash scripts/api-demo.sh
```

La suite completa ejecutada por `test-backends.sh` aprobó **426 pruebas**, sin
fallos y con una ignorada del proveedor externo de sellado. PostgreSQL y Redis
estuvieron disponibles: se ejecutaron las dos pruebas de identidad y las siete
de expedientes. Se añadieron siete pruebas de dominio, diez de casos de uso,
siete de persistencia y ocho HTTP, con fallo inicial antes de implementar.
Formato, compilación, Clippy y ambas demostraciones terminaron con código cero.
El workflow de CI se validó con `actionlint`; las nuevas pruebas usan una base
separada para no alterar la precondición del bootstrap de identidad.

Las pruebas reales de persistencia cubren reconexión, filtrado antes de paginar,
revocación, usuarios inexistentes o inactivos, asignaciones concurrentes sin
duplicados y rollback si falla la inserción de la membresía del creador. La
demostración HTTP cubre los cuatro roles y muestra que:

- Owner consulta todos los expedientes y administra las asignaciones.
- Litigante crea un expediente y obtiene automáticamente su asignación.
- Un UUID ajeno y uno inexistente tienen el mismo error `case_not_found`.
- Retirar la asignación elimina el acceso en la siguiente petición con la misma
  sesión, tanto de detalle como de listado.
- Cliente asignado consulta metadatos, pero carga, sellado, verificación y
  descarga documental responden `403`.
- Cambiar el rol o desactivar una cuenta afecta a su sesión ya emitida.
- El flujo documental mantiene cifrado, sello local y ZIP verificable con OpenSSL.

Los servicios temporales se detuvieron y sus datos se eliminaron al terminar.
El script de pruebas también permite reproducir la cobertura con servicios
reales mediante un comando Cargo como argumento.

### Cobertura reproducida

```bash
bash scripts/test-backends.sh cargo llvm-cov --workspace --json --summary-only --output-path /tmp/tt-cases-coverage.json
bash scripts/coverage-gate.sh /tmp/tt-cases-coverage.json
```

| Crate | Líneas cubiertas | Cobertura |
| --- | --- | --- |
| `domain` | 1029/1069 | 96.3 % |
| `application` | 1815/1951 | 93.0 % |
| `infrastructure` | 2393/2573 | 93.0 % |
| `bin` | 834/1039 | 80.3 % |
| `web` | 470/579 | 81.2 % |

Total: 6541/7211 líneas (90.7 %). Los tres crates con
umbral obligatorio superaron el 90 %.

### Límites del avance

Los expedientes guardan título y referencia; las asignaciones controlan acceso
de usuarios. Participantes procesales, audiencias y plazos siguen pendientes.
Los documentos aún no pertenecen a expedientes y el personal conserva permisos
documentales globales; Cliente sigue denegado. Documentos y bitácora permanecen
en archivos separados, y las mutaciones de expedientes todavía no tienen
historial de auditoría. La UI continúa como placeholder.

Este avance no recompiló el reporte ni la presentación: sus fuentes y cambios
locales se conservaron. Las medidas documentales, de rendimiento y del binario
que siguen son históricas, no resultados nuevos de esta corrida.

## Evidencia histórica: 17 de agosto de 2026

Esta sección conserva el corte anterior para comparación. Sus conteos,
cobertura, tiempos, tamaños y pendientes describen aquella revisión.

### Corte reproducido

- Fecha local: 17 de agosto de 2026 (`America/Mexico_City`).
- Fecha observada en la salida UTC de la demostración: 17 de agosto de 2026.
- Revisión verificada: rebanada vertical HTTP local en la rama
  `docs/avance-cripto-beamer`.
- Rust: `rustc 1.94.0` y `cargo 1.94.0`.
- OpenSSL: `3.5.7` del 9 de junio de 2026.
- Cobertura: `cargo-llvm-cov 0.8.7`.

### Suite automatizada

Comando:

```bash
cargo test --workspace
```

Resultado:

```text
395 funciones de prueba descubiertas
394 aprobadas
0 fallidas
1 ignorada
```

La prueba ignorada es el humo contra el sandbox real del proveedor de
sellado. Se conserva para documentar el adaptador, pero no se ejecuta como
requisito de esta entrega: la API es inestable y el sandbox es de pago con
precios no transparentes. Los otros 13 casos del
adaptador remoto se ejecutaron contra el stub HTTP local y aprobaron,
incluidos token inmediato, procesamiento diferido, rechazo, errores HTTP y
redacción de la credencial.

`cargo fmt --all -- --check`, `cargo build --workspace` y
`cargo clippy --workspace --all-targets -- -D warnings` terminaron con código
cero. El binario release mide 7 886 552 bytes, por debajo del límite de 25 MiB.
El workflow de CI quedó configurado para levantar PostgreSQL y Redis tanto en
el job de pruebas como en el de cobertura, y su YAML se parseó localmente.

### Cobertura

Comandos:

```bash
cargo llvm-cov --workspace --json --summary-only \
  --output-path /tmp/tt2026-coverage.json
bash scripts/coverage-gate.sh /tmp/tt2026-coverage.json
```

Resultado:

```text
domain             973/ 1013 lines   96%  (gate: >=90%)
application       1749/ 1885 lines   92%  (gate: >=90%)
infrastructure    2257/ 2436 lines   92%  (gate: >=90%)
bin                834/ 1034 lines   80%  (gate: none)
web                319/  436 lines   73%  (gate: none)
```

Las proporciones sin truncar son 96.1 %, 92.8 %, 92.7 %, 80.7 % y 73.2 %,
respectivamente. El total del workspace es 6 132 de 6 804 líneas, 90.1 %.
La corrida de cobertura levantó PostgreSQL y Redis reales para no contabilizar
como cubiertos adaptadores que las pruebas omiten cuando esos servicios no
están disponibles.

### Demostración integral

Comando:

```bash
bash scripts/demo.sh
```

La ejecución terminó con código cero y reprodujo:

1. Creación de la CA interna y emisión del certificado del firmante.
2. Emisión de un certificado de TSA y de un token RFC 3161 local.
3. Hash SHA-256 del documento de muestra.
4. Cifrado AES-256-GCM, alteración de un byte y rechazo autenticado.
5. Rotación de la KEK sin volver a cifrar el documento.
6. Firma RSA-3072 y verificación integral de cuatro componentes.
7. Alteración del documento y rechazo con causas por componente.
8. Exportación del ZIP de evidencia y verificación con OpenSSL y `unzip`.
9. Revocación del certificado y detección mediante CRL.
10. Calibración Argon2id, TOTP y bitácora encadenada con detección de cambios.

La verificación independiente produjo `Verified OK`, `certificado.pem: OK` y
`Verification: OK`. Los archivos temporales y secretos de demostración fueron
eliminados automáticamente al terminar.

### Demostración de la aplicación HTTP

Comando:

```bash
bash scripts/api-demo.sh
```

La ejecución terminó con código cero y, sin variables de Cincel, levantó el
servidor en un puerto efímero; creó usuarios persistidos; completó TOTP;
comprobó `401`, RBAC, logout y recuperación de un solo uso; cargó y persistió
un documento cifrado; comprobó que el texto claro no aparece en el repositorio;
lo firmó y selló mediante la TSA local; verificó los cuatro componentes;
exportó el ZIP; y comprobó firma, certificado, CRL y sello con `openssl`. La
cadena de auditoría terminó válida con al menos diez eventos. El contrato y sus
límites se documentan en
[`docs/http-api.md`](http-api.md).

La suite incluye además una prueba que renombra deliberadamente un registro
JSON bajo el UUID de otro documento. El repositorio detecta que la identidad
interna no coincide con la ruta solicitada y rechaza el registro como
inconsistente.

### Entregables documentales

`make -C latex` generó `latex/main.pdf` con 232 páginas en tamaño carta y
`make -C presentacion` generó `presentacion/presentacion.pdf` con 14
diapositivas 16:9. Se renderizaron las 232 páginas de la tesis y las 14
diapositivas; se inspeccionaron ampliadas las páginas modificadas de
implementación, pruebas, conclusiones y anexos. El log final de Beamer no
contiene advertencias `Overfull`, `Underfull` ni `LaTeX Warning`.

### Decisión sobre el proveedor de sellado

El registro en el entorno de Cincel pudo completarse, pero eso no garantiza la
operación del servicio. Durante el consumo, la API no ofreció respuestas y
estabilidad suficientes para una campaña repetible; además, el sandbox
requiere pago y sus precios no son transparentes para presupuestar la muestra.
RTC-01 se registra como incidencia materializada; su probabilidad y VME se
conservan como línea base, no como medición de un costo ya incurrido.

Por decisión del proyecto, Cincel no es una dependencia técnica ni un criterio
de evidencia de la entrega actual. El adaptador y su stub se conservan para
demostrar la intercambiabilidad del puerto, pero la prueba contra el servicio
real permanece ignorada y no se agenda como campaña pendiente.

La decisión está registrada en
[`docs/adr/0009-local-timestamp-authority.md`](adr/0009-local-timestamp-authority.md).
La mitigación activa es `LocalOpensslTsa`, una TSA RFC 3161 cuyo certificado
RSA-3072 es emitido por la CA interna con `extendedKeyUsage = timeStamping`.
Opera detrás del mismo puerto que el adaptador remoto y sus tokens son
aceptados por `openssl ts -verify`. Esta vía demuestra continuidad técnica;
no aporta independencia de tercero ni sustituye una constancia NOM-151 emitida
por un PSC autorizado.

### Límites abiertos

- La evidencia externa de un PSC autorizado queda fuera del alcance de esta
  entrega. Una integración futura requerirá elegir un proveedor con contrato,
  disponibilidad y precios verificables; no se presenta la TSA local como
  sustituto jurídico de una constancia NOM-151.
- Argon2id quedó calibrado en el hardware de referencia (AMD Ryzen 7 7730U,
  16 hilos lógicos) con `m=262144,t=2,p=1`: cinco corridas promediaron 529.4 ms,
  dentro de la banda objetivo de 500 a 1 000 ms. Si el hardware de despliegue
  difiere, la medición debe repetirse.
- La API local ya entrega identidad multiusuario, sesiones revocables, RBAC,
  carga, persistencia cifrada en JSON, sellado, verificación, exportación y
  auditoría. La identidad procede del bearer token; `X-Actor` fue retirado.
- PostgreSQL persiste usuarios y Redis conserva el estado efímero de identidad.
  Documentos y auditoría siguen en archivos locales sin una transacción común;
  la pertenencia a casos y las consultas ampliadas permanecen pendientes.
  La interfaz documental de `web/` se verifica por separado a continuación.

La transcripción extensa de una corrida anterior se conserva en
[`docs/demo-transcript.md`](demo-transcript.md).

## Verificación de la interfaz web

Comprobaciones ejecutadas el 11 de septiembre de 2026 en Windows, con
Node.js 24.21.0 y npm 11.19.0. Estos resultados corresponden a `web/` y no
actualizan las mediciones históricas de Rust o criptografía anteriores.

- `npm test`: 18 pruebas aprobadas del cliente HTTP, errores, sesiones,
  descarga binaria, nombres compatibles con el ZIP, tamaño de documentos,
  UUID, permisos visibles, estados documentales, filtros y rutas por rol.
  Incluyen respuestas tardías que no deben afectar una sesión posterior.
- `npm run test:e2e -- --workers=1`: 14 pruebas aprobadas en Chromium con
  Playwright. Cubren
  alta inicial, MFA con TOTP o recuperación, carga, sellado, verificación,
  descarga, logout, roles, rechazo MFA, sesión vencida, alta de integrantes,
  auditoría, resultados obsoletos, recuperación de documentos pendientes de
  sello, resumen de sesión, filtros, vista de tarjetas, historial del navegador,
  visibilidad de contraseña y menú accesible en una pantalla de 390 px de ancho.
  Las regresiones cubren consultar otra vez el mismo documento, recibir una
  respuesta después de cerrar sesión y consumir las acciones de navegación.
  Los 12 flujos de `workflow.spec.mjs` no emitieron errores de JavaScript.
- `npm run build`: compilación estática completada con Astro 7 y Svelte 5.
- `npm run format:check`: sin diferencias de formato.
- `npm audit`: la comprobación anterior del mismo día reportó cero
  vulnerabilidades. Esta revisión de interfaz no modifica las dependencias.
- Revisión visual de capturas de inicio en escritorio y móvil, acceso móvil
  y detalle documental de escritorio, generadas por las pruebas de navegador.

Las pruebas interceptan las rutas HTTP con respuestas de prueba; no se
ejecutaron la API Rust, PostgreSQL, Redis ni la TSA en esta comprobación.
Tampoco se repitieron `scripts/api-demo.sh`, `scripts/demo.sh` ni las pruebas
de Cargo. No se modificó código Rust. La integración completa con servicios
reales requiere el entorno descrito en `docs/http-api.md`.

La nueva automatización `.github/workflows/web.yml` ejecuta formato,
pruebas, compilación y pruebas de navegador en Linux. Este informe no afirma
una corrida remota de ese workflow.

### Integración de marca Qadra

Comprobaciones ejecutadas el 12 de septiembre de 2026, después de incorporar
el nombre y los assets originales de Qadra en `web/`:

- `npm run format:check` y `npm run build`: completados correctamente.
- `npm run test:e2e -- --workers=1`: 14 pruebas aprobadas en Chromium con las
  mismas respuestas HTTP simuladas. No se repitieron las pruebas unitarias
  porque esta corrección no modifica la lógica del cliente.
- Los SHA-256 del logo SVG, favicon SVG, logo PNG y licencia coinciden con
  los archivos originales. Su procedencia se conserva en
  `web/public/brand/qadra/README.md`.
- Revisión del nombre, carga local de imágenes y marca en acceso de
  escritorio y móvil, y en la navegación del espacio documental.

Esta comprobación tampoco ejecuta los servicios reales del backend.

### Revisión de español de México

El 12 de septiembre de 2026 se revisaron los textos de acceso, navegación,
documentos, administración, ayuda y errores, así como las guías de `web/`.
Se corrigieron tildes, signos de apertura y concordancia; el documento HTML
declara `es-MX`. Las entidades HTML y los escapes Unicode permiten mostrar
los caracteres correctos y conservar los archivos de código en ASCII.

- `npm test`: 18 pruebas aprobadas con las expectativas de texto actualizadas.
- `npm run test:e2e -- --workers=1 --max-failures=2`: 14 pruebas aprobadas en
  la ejecución final. Una ejecución anterior agotó los 30 segundos de espera
  durante el acceso simulado; la repetición completa pasó con el mismo límite.
- `npm run format:check` y `npm run build`: completados correctamente.
- Revisión visual del acceso móvil, inicio móvil y detalle documental de
  escritorio: acentos legibles y sin desbordamiento por los textos corregidos.

Las comprobaciones de navegador mantienen la API simulada; no se probaron
los servicios reales del backend en esta revisión.
