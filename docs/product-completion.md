# Cierre funcional del prototipo web

## Alcance y evidencia

El cierre comprende gestión documental, gestión procesal penal, interfaz web,
conciliación de alcance y validación del producto. Los objetivos aprobados se
conservan en [la introducción](../latex/chapters/01-introduccion.tex); el catálogo
funcional está en [análisis y diseño](../latex/chapters/03-analisis-diseno.tex).
El [informe de verificación](verification-report.md) distingue resultados
reproducidos de mediciones históricas. Este documento distingue capacidades
implementadas y trabajo pendiente; la matriz no sustituye su evidencia.

La aplicación `web/` integra la identidad Qadra con expedientes reales, rutas
autorizadas y consultas persistentes. Ofrece carga inicial, historial y selección
de versiones, sellado, verificación y descarga exactos. La clasificación actual
tiene tipo, clasificación, etiquetas y revisiones auditadas independientes del
contenido, con carga inicial atómica y filtros exactos. Las pruebas con HTTP
simulado y los recorridos contra servicios reales tienen evidencias separadas.
`frontend/` conserva el ejemplo anterior; el producto continúa en `web/`.

El directorio por expediente agrega fichas independientes de las cuentas, valores
manuales o tipificados, archivo/reactivación e historial inmutable con autoría. El rol de acceso
y la asignación autorizan cada operación; el texto del rol procesal no concede
permisos. Su base y límites se describen en
[ADR-0021](adr/0021-audited-case-participants.md). Este avance no cierra los
criterios de identidad jurídica ni la gestión procesal completa.

Las fichas tipificadas vinculan una revisión exacta de identidad representada y
uno de once perfiles. La revisión de posibles duplicados usa señales declaradas,
soportes y huellas de certificados, con decisiones explícitas y control de
concurrencia. Qadra permite completar fichas manuales, consultar la identidad
actual por separado y recuperar evidencia pública de declaraciones firmadas
externamente. La CA interna y una CRL publicada delimitan esa demostración:
no equivalen a FIREL ni acreditan civilmente una identidad o una profesión.
Los contratos están en [ADR-0025](adr/0025-case-subjects-and-typed-participants.md),
[ADR-0026](adr/0026-internal-participant-declarations.md) y
[la API tipificada](typed-participants-api.md).

El registro penal completo añade NUC, carpeta judicial, autoridades, delitos e
información general, con revisión administrativa y registro inicial de
Investigación. La edición, el cierre administrativo y la reapertura conservan
la historia y la etapa inicial. Completar una ficha anterior no inventa etapas;
la API básica mantiene su proyección para Client. La administración y el cierre
se describen en [ADR-0022](adr/0022-audited-penal-case-administration.md).

La adopción explícita y los avances Investigación a Intermedia e Intermedia a
Juicio tienen dominio, aplicación, persistencia auditada, HTTP y flujo Qadra.
Cada soporte fija documento, versión y digest; se comprueban integridad y formato
PDF/DOCX en un proceso acotado. Se distinguen registro inicial histórico, etapa
actual, actos declarados y captura del sistema. Perfil completo, estado activo,
rol y asignación se revalidan al confirmar; cierre y revocación no se eluden con
una preparación anterior. Lectura e historia siguen disponibles en expedientes
cerrados según permisos. El diseño está en [ADR-0023](adr/0023-audited-case-stage-transitions.md)
y [ADR-0024](adr/0024-isolated-document-format-admission.md).

La programación de audiencias incorpora cuatro tipos, referencias históricas de
participantes, soporte exacto de antecedente para individualización, reemplazos
y cancelación organizativa. Qadra conecta el directorio del expediente con una
agenda transversal autorizada. Las comprobaciones por capa, la campaña global,
HTTP, restauración y navegador real se distinguen en el
[informe de verificación](verification-report.md). Su contrato y límites están en
[la API de audiencias](hearings-api.md) y [ADR-0028](adr/0028-audited-hearing-scheduling.md).

Las sesiones y resultados declarados añaden raíces independientes con ancla de
programación y continuidad exactas, comparecencias históricas, acuerdos ordenados,
procedencia y precisión temporal conservadas. La rectificación y el retiro
mantienen historia y recibos propios; Qadra permite prepararlos y consultarlos.
La verificación local comprende pruebas por capa, campaña global, API con
restauración e interfaz simulada y real; sus métricas se conservan en el informe
de verificación. No activan plazos ni alertas y no acreditan actos judiciales. Véanse [el contrato](hearing-results-api.md) y
[ADR-0029](adr/0029-declared-hearing-sessions.md).

El backend, la API y Qadra para el catálogo de calendarios jurisdiccionales están
verificados localmente, incluida la restauración y los recorridos de navegador
con servicios reales. Sus
revisiones conservan ámbito global, fechas civiles, cobertura,
patrón semanal, excepciones y referencias públicas declaradas. Owner gestiona;
el personal consulta sin membresía de expediente y Client queda denegado.
La clasificación distingue exclusión, falta de resolución y falta de cobertura,
sin fabricar disponibilidad o efectos jurídicos. La cobertura y las campañas
web están aprobadas localmente; la integración remota y la actualización del
manuscrito siguen pendientes. Véanse [el contrato](judicial-calendars-api.md) y
[ADR-0030](adr/0030-versioned-jurisdictional-calendars.md).

El [conteo civil diario](deadline-day-counting.md) ya calcula una fecha candidata
sobre valores exactos de calendario y conserva cada dia con su clasificacion,
fuente y acumulado. Se detiene ante datos sin resolver, falta de cobertura o
agotamiento del rango de fechas. Sus quince pruebas estan incluidas en la suite
completa reproducida. La primera fecha incluida y la cantidad todavia son
entradas matematicas: los hechos declarados ya tienen persistencia propia, pero
falta vincularlos a un perfil aplicable, resolver los datos requeridos de
recepcion y conservar la evaluacion y sus alertas para obtener un plazo operativo.

La [precision temporal declarada](procedural-time.md) conserva datos desconocidos,
fecha, minuto y segundo, con desfase opcional. Esta implementacion de dominio no
registra hechos por si misma. La separacion de resoluciones y practicas de
notificacion queda adoptada en [ADR-0031](adr/0031-declared-procedural-facts.md),
con contrato y persistencia descritos abajo; la API HTTP esta implementada y
su interfaz Qadra sigue pendiente.
El [modelo puro de hechos](procedural-facts.md) ya conserva las dos familias,
referencias historicas seleccionadas, desconocimiento, funciones personales y
soportes directos. Sus [canones propios](procedural-facts-canonical.md) distinguen
precision y desfase, y conservan cada localizador aunque un documento se comparta.
La [base de aplicacion](procedural-facts-application.md) incorpora comandos de
alta/correccion/retiro, seleccion exacta, comprobaciones puras y contratos de
puertos. El servicio de aplicacion ya coordina autenticacion, verificacion de
fuentes exactas, admision del lote directo, reautenticacion y comprobacion de
recibos de preparacion y respuesta. Sus [canones de fuentes y operacion](procedural-facts-receipts.md)
conservan las vistas historicas y se contrastan con vectores independientes.
El [adaptador PostgreSQL](procedural-facts-persistence.md) implementa ahora
persistencia, autorizacion efectiva por expediente e historia auditada atomica.
La verificacion focal local cubre ambas familias, fuentes exactas, permisos,
concurrencia y restauracion. El cierre anterior del backend incluyo una campana
global local y el guion HTTP de las capacidades ya expuestas; sus resultados
constan en el informe de verificacion.
La [API HTTP de hechos](procedural-facts-api.md) y su composicion estan
implementadas. Pasaron 25 pruebas focales unitarias de entrada/proyeccion y
26 pruebas de rutas HTTP con puertos controlados. La suite global y el recorrido
HTTP con servicios reales y restauracion tambien estan aprobados localmente.
Qadra sigue pendiente, y estos
hechos aun no habilitan plazos operativos.

## Entregas y condiciones de cierre

| Entrega | Alcance verificable | Dependencias y evidencia requerida |
| --- | --- | --- |
| Consultas documentales e integración Qadra | Listado paginado, detalle, búsqueda literal de nombre y filtro de sellado; selección y creación de expedientes; carga, sello, verificación y evidencia en el expediente seleccionado. | Autorización vigente antes de exponer metadatos; cuatro roles, expedientes ajenos, revocación, filtros antes de paginación, eventos de consulta y errores. Pruebas HTTP, PostgreSQL y navegador. |
| Versiones documentales | Identidad estable, cifrado vinculado a UUID/versión, evidencia histórica inmutable y selección explícita de snapshot. | ADR-0019, append optimista, migración V1/V7, aislamiento, conflictos, restauración y evidencia byte por byte; resultados en el informe de verificación. |
| Clasificación documental | Tipo, clasificación y etiquetas organizativas con revisiones auditadas; búsqueda autorizada por metadatos. | ADR-0020, carga atómica, canon Unicode, revisiones esperadas, concurrencia, permisos y restauración sin alterar evidencia; resultados en el informe de verificación. |
| Directorio de participantes | Fichas por expediente, revisión esperada, historial, filtros, archivo y reactivación con valores vigentes. | ADR-0021, independencia de cuentas, cuatro roles, asociación ajena, concurrencia, auditoría y restauración. No acredita identidad ni firma judicial. |
| Identidad y firma personal | Resolver autenticación con certificado de socios, identidad del firmante y custodia de claves; altas, bajas, invitaciones y recuperación segura de credenciales. | La clave de firma configurada para el servidor no demuestra firma individual por usuario. Separar recuperación MFA de recuperación de contraseña. Definir contratos y probar revocación con sesiones existentes y fallos de entrega. |
| Administración del expediente penal | Alta penal completa, edición, historia administrativa, filtros, cierre y reapertura; inicial Investigación para nuevas altas completas. | ADR-0022, unicidad de identificadores actuales, R0/R1 pendientes explícitos, cuatro roles, CAS, cierre concurrente, auditoría y restauración completa. |
| Adopción y transiciones de etapa | Adopción para perfiles completos sin etapa; dos avances ordinarios, fechas declaradas y soportes exactos con admisión PDF/DOCX; historia y conflicto explícito en Qadra. | ADR-0023/0024, secuencia y origen de R1, autorización antes y después de preparar, cierre/revocación concurrentes, auditoría, restauración y navegador real. No incluye recursos ni decisiones jurídicas automáticas. |
| Participantes tipificados | Identidades representadas, once perfiles, soportes exactos, revisión de candidatos y declaraciones internas con firma externa. | ADR-0025/0026, unión histórica manual/tipificada, CAS de identidades y fichas, confianza publicada, permisos, cierre, auditoría y restauración. La demostración interna no acredita identidad civil, profesión ni FIREL. |
| Recursos procesales | Resoluciones y soportes exactos, actos e historia propios, audiencias, términos calculados y alertas asociados. | Pendiente; propuesta y criterios en [alcance de recursos](procedural-resources-scope.md). No es una cuarta transición ni se satisface con documentos o fechas manuales. |
| Programación de audiencias | Cuatro tipos, reemplazo/cancelación con recibos propios, contexto y participantes exactos, historia y agenda autorizada. | ADR-0028; persistencia, autorización, auditoría y Qadra implementados. Evidencias de concurrencia, soporte histórico, resultados inciertos, restauración y navegador en el informe de verificación. No registra celebración, asistentes reales ni acuerdos. |
| Sesiones y resultados declarados | Raíces propias, ancla y continuidad exactas, comparecencias, acuerdos, procedencia, rectificación, retiro e historia; Qadra y persistencia auditada. | ADR-0029; implementado y verificado localmente, pendiente de integración remota. Fuentes históricas admitidas, soporte readmitido al rectificar, recibos y recuperación; no acredita actos ni efectos jurídicos. |
| Catálogo de calendarios jurisdiccionales | Ámbito inmutable, revisiones, cobertura, reglas semanales, excepciones y referencias públicas; consulta civil exacta y retiro con recibo. | ADR-0030; backend, API, Qadra, cobertura y restauración verificados localmente. Integración remota y manuscrito pendientes; conservar evidencia de autorización global, canon independiente, concurrencia e inventario. Las referencias no preservan contenido remoto ni acreditan aplicabilidad. |
| Hechos declarados de resolución y notificación | Dos familias por expediente, padre fijo, tiempos y personas declarados, fuentes exactas, corrección, retiro terminal, recibos e historia. | ADR-0031; dominio, aplicación, backend y API implementados. Backend, pruebas focales, suite global y comprobacion HTTP con servicios reales y restauracion aprobados localmente; Qadra pendiente. No acredita efectos jurídicos ni habilita cálculos o recursos. |
| Plazos y calendario | Plazos vinculados, calendario configurable, vencimientos y alertas persistentes. | Cómputo, reevaluación y alertas pendientes. El catálogo y los hechos declarados son insumos parciales: falta seleccionar y validar sus datos para perfiles normativos aplicables, evaluación persistente y casos frontera. No extraerlos de acuerdos libres ni sustituir el cómputo exigido por fechas manuales o una suma indiscriminada de días. |
| Tablero, informes y bitácora | Indicadores obtenidos de datos autorizados, filtros y exportaciones; consulta de auditoría separada de su verificación criptográfica. | No presentar el tamaño de una página como total del despacho. Probar aislamiento de agregados e informes, concurrencia y acceso a resultados generados. |
| Validación integral | Casos positivos y negativos del catálogo completo, flujos reales desde navegador, rendimiento, fallos y recuperación; usabilidad con personal del despacho. | PostgreSQL y Redis aislados, TSA local, evidencias reproducibles, comparación visual de escritorio y móvil, métricas con entorno y fecha. Usabilidad requiere participantes reales y resultados observados. |

Las entregas se implementan en ramas `feat/` y se integran mediante PR y squash
con CI aprobado. Cada entrega funcional actualiza contrato, decisiones,
operación y apartados académicos afectados. El diseño aprobado se mantiene;
sus divergencias se resuelven explícitamente antes de marcar un objetivo como
cumplido. Las conclusiones y la presentación corresponden al cierre académico
posterior y no se completan a partir de un resultado parcial del backend.

## Fidelidad del diseño

La referencia visual es el sistema existente en `web/src/styles/`, los componentes
de `web/src/components/` y los originales con procedencia y licencia en
`web/public/brand/qadra/`. Se conservan marca, tipografía, colores, escala de
espaciado, navegación, controles, tarjetas y comportamiento adaptable a móvil.
Las nuevas vistas reutilizan estos elementos. Las ampliaciones se separan en
archivos para mantener el límite de tamaño de código.

La integración reemplaza las rutas documentales globales y las referencias
temporales por operaciones de la API de expedientes. Una verificación
criptográfica se solicita como acción explícita; no se usa para inferir el
detalle o la existencia de un documento. Al cambiar de expediente o sesión se
descartan respuestas anteriores y se evita mostrar datos del contexto previo.

## Matriz de cierre del catálogo funcional

Los estados describen el flujo completo de producto, no solo una primitiva o
una ruta. Cada cierre exige evidencia positiva y negativa, autorización,
persistencia, auditoría e interfaz aplicables. Los criterios se derivan del
catálogo versionado en análisis y diseño; no reemplazan objetivos aprobados.

| Caso de uso | Estado | Alcance y condición pendiente para cierre |
| --- | --- | --- |
| Registro de despacho y selección de plan | Parcial | Conservar bootstrap; conciliar selección comercial con instancia de un solo despacho y completar enrolamiento recuperable. |
| Ciclo de vida de miembros | Parcial | Invitaciones, directorio consultable, baja y protección del último Owner; selección por usuario en asignaciones. |
| Inicio de sesión y sesiones | Parcial | Certificado de socio, recuperación de contraseña e inactividad; contraseña/MFA y logout ya tienen implementación. |
| Control de acceso por perfil | Parcial | Extender la matriz a cada módulo pendiente y comprobar el registro de accesos exigido por el catálogo. |
| Registro y administración de expediente penal | Implementado para el alta penal completa | NUC/carpeta y autoridades, delitos, metadatos, unicidad actual, Investigación inicial, edición y cierre con historia verificados. Las fichas anteriores se completan sin fabricar etapa; su adopción y las transiciones usan el recurso independiente de etapas. Los valores declarados no son certificaciones institucionales. |
| Directorio de participantes | Implementado para identidad representada y credencial interna de demostración | Once perfiles, datos declarados, identidad versionada, revisión explícita de coincidencias, unicidad de identidad/rol, soporte y firma interna; compatibilidad manual, consultas y estado auditados. Quedan fuera la acreditación civil/profesional, FIREL real y la certificación jurídica de expediente penal activo. El cierre organizativo bloquea mutaciones. |
| Transición de etapa procesal | Implementado para adopción y dos avances ordinarios | Perfil completo, documentos/versiones exactos, fechas declaradas, admisión PDF/DOCX, historia, conflictos, cierre y revocación tienen flujo persistente en Qadra. El registro no certifica la procedencia jurídica del acto ni implementa recursos o plazos. La programación de audiencias usa su propio historial. |
| Audiencia y activación de plazos | Parcial; programación y sesiones declaradas implementadas localmente | Programación, agenda, resultados declarados, comparecencias, acuerdos, continuidad, rectificación y retiro tienen flujo propio. Sus campañas locales concluyeron; la integración remota permanece pendiente. Faltan activación consistente de plazos, alertas, catálogo restante y aceptación integral; registrar texto no calcula efectos jurídicos. |
| Calendario judicial | Parcial; catálogo con backend, API y Qadra verificados | Configuración global de cobertura, reglas y excepciones con fuentes declaradas, restauración, cobertura y recorridos de navegador aprobados; CI de calendarios aprobado. Faltan integración remota, actualización académica, selección aplicable para plazos, evaluación y reevaluación ante cambios. El conteo civil puro aporta una candidata; una URL o la clasificación de una fecha no acredita la regla normativa. |
| Monitoreo de plazos y alertas | Pendiente | Vencimientos, entrega de alertas, reintentos y ausencia de duplicados. |
| Carga y clasificación documental | Parcial | Conciliar la política de formatos de carga general. Carga cifrada, límites, clasificación atómica, filtros y versiones implementados. La admisión PDF/DOCX ahora valida soportes nuevos de etapas; no se aplica retrospectivamente ni convierte toda carga general en validación estructural. |
| Consulta e integridad documental | Parcial | Entrega íntegra de contenido sin requerir sello y alerta de seguridad al Owner ante alteraciones. Historial, filtros de clasificación, verificación explícita y exportación de evidencia sellada implementados. |
| Firma de contrato y sello | Parcial | Vincular credencial y autorización al firmante individual y comprobar estado del certificado antes de firmar. |
| Verificación de firma y sello | Parcial | Política de identidad individual; la selección histórica explícita conserva los verificadores existentes. |
| Tablero de control | Parcial | Indicadores procesales agregados autorizados; la navegación Qadra no equivale a indicadores globales. |
| Informes | Pendiente | Filtros, generación y acceso autorizado a resultados; comprobar fallos y concurrencia. |
| Consulta de actividad | Parcial | Listado filtrado de eventos y permisos por recurso; verificación de la cadena ya implementada. |

La validación final incluye usabilidad con participantes reales. Ningún resultado
de cobertura ni una demostración parcial cambia automáticamente estos estados.

## Conciliación de alcance

- El prototipo atiende un solo despacho. Registro y selección de planes deben
  conciliarse con esta delimitación antes de introducir facturación o aislamiento
  multiinquilino. Ninguna de esas ampliaciones es requisito implícito para
  conectar la interfaz existente.
- La autenticación implementada usa contraseña y MFA. El sello documental usa
  la credencial configurada al arrancar el servidor. Las declaraciones internas
  de participantes verifican una firma externa de 384 bytes sobre una declaración
  de 218 bytes; no reciben la clave privada. Ese flujo no vincula automáticamente
  la identidad representada con una cuenta de acceso. La autenticación por
  certificado y la firma documental individual de usuarios continúan pendientes.
- El corte de sesiones declaradas midió Argon2id en 409.6 ms de promedio sobre
  cinco corridas, por debajo de la banda de 500--1000 ms. La calibración de costo
  de intento permanece pendiente para el entorno de despliegue; aprobar la CLI
  no satisface ese criterio. Los parámetros no cambiaron en esta entrega.
- Client mantiene acceso a metadatos de expedientes asignados y denegación
  documental. Ampliarlo requiere una política de recursos explícita y pruebas
  de aislamiento; ocultar botones no constituye autorización.
- El criterio OE-2 conserva su redacción aprobada sobre cuatro etapas. La
  [propuesta de recursos](procedural-resources-scope.md) distingue las tres etapas
  del CNPP y recursos vinculados a resoluciones, sin modificar ese criterio.
  Corregirlo literalmente requiere autorización explícita; implementar las dos
  transiciones no cierra recursos ni el cómputo automático exigido.
- El catálogo de programación distingue cuatro tipos de audiencia; el marco
  también menciona medidas cautelares y continuaciones. Las sesiones declaradas
  modelan continuidad mediante otra raíz con antecedente exacto preexistente,
  con comparecencias y acuerdos propios. Esto no amplía el catálogo de citas ni
  cubre las reglas específicas de medidas cautelares. CU-08 y RF-07 permanecen
  parciales: sus criterios aprobados incluyen activación de plazos y alertas,
  además de la captura implementada y la aceptación integral.
- CU-09 permanece parcial. El catálogo distingue fechas `countable`, `excluded`,
  `unresolved` y `outside_coverage` según el ámbito declarado. No adopta un
  calendario universal, no descarga normas ni asigna calendarios comparando el
  nombre de una autoridad. El cómputo automático exigido sigue pendiente y
  requiere hechos jurídicos estructurados, fuentes aplicables y un corpus de
  aceptación antes de activar vencimientos, reevaluación o alertas. Este avance
  no completa por sí solo los demás casos del catálogo ni su aceptación integral.
- Las alertas internas y la entrega por correo tienen contratos distintos. No se
  da por completada una notificación por persistir únicamente un vencimiento.
- El archivo de una ficha es organizativo. No prueba una transición jurídica,
  nombramiento o legitimación; una etiqueta procesal no modifica RBAC. La revisión
  de fuentes oficiales en ADR-0021 mantiene separados certificado, identidad y
  representación. La declaración de demostración de ADR-0026 comprueba posesión
  de una clave y vincula el contenido exacto registrado con confianza interna
  capturada. El criterio de FIREL del catálogo permanece fuera de esa equivalencia;
  no se presenta una cadena interna como credencial judicial externa.
- La TSA local y la CA interna permiten verificar evidencia técnica. La campaña
  con un PSC externo permanece fuera de la entrega actual, según
  [la decisión sobre TSA local](adr/0009-local-timestamp-authority.md).

## Validación y dependencias externas

Antes de cerrar, la matriz de los casos de uso debe enlazar cada comportamiento
con su prueba y resultado. La cobertura de líneas y las pruebas criptográficas
existentes no prueban por sí mismas el catálogo procesal ni la usabilidad.
Los ensayos de navegador con respuestas simuladas se registran separados de
aquellos que ejecutan Rust, PostgreSQL, Redis y TSA. Una captura o un render
verifica presentación; no demuestra por sí solo una operación confirmada.

El servidor requiere qpdf 12.4.1 en Linux x86_64 y comprueba su worker antes de
aceptar tráfico. Preparación, variables y límites están en
[operación del validador](document-format-operations.md). La admisión técnica no
acredita autenticidad jurídica ni implica renderizado o detección de malware.

La usabilidad requiere acordar participantes, tareas y condiciones, obtener
observaciones reales, corregir los problemas detectados y registrar los
resultados. Preparar el protocolo o simular un usuario no completa ese ensayo.
Mientras falte la participación externa, continúan las tareas de implementación
y validación técnica que puedan ejecutarse independientemente.

El despliegue público requiere además resolver los límites operativos enumerados
en [la revisión del backend](backend-review.md), revisar el modelo de amenaza de
firma HTTP y delimitar la ausencia de anclaje externo de auditoría. La
demostración local no se presenta como validación de producción.
