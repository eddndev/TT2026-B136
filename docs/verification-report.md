# Informe de verificación local

La actualización académica posterior de estos resultados y la comprobación del
PDF se documentan en [la revisión del reporte](academic-report-verification.md).
Esa revisión documental no constituye una nueva ejecución de la suite Rust.

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
