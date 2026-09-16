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
| Audiencias, plazos y calendario | Audiencias vinculadas al expediente, plazos, calendario configurable, vencimientos y alertas persistentes. | Consultar fuentes normativas oficiales vigentes al implementar reglas; casos de prueba de fechas, días inhábiles, excepciones y cambios de calendario. No sustituir validación de las reglas por una simple suma de días. |
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
| Transición de etapa procesal | Implementado para adopción y dos avances ordinarios | Perfil completo, documentos/versiones exactos, fechas declaradas, admisión PDF/DOCX, historia, conflictos, cierre y revocación tienen flujo persistente en Qadra. El registro no certifica la procedencia jurídica del acto ni implementa recursos, audiencias o plazos. |
| Audiencia y activación de plazos | Pendiente | Programación, cambios, cancelaciones y creación consistente de plazos vinculados. |
| Calendario judicial | Pendiente | Calendarios aplicables, inhábiles y excepciones, con pruebas normativas y de fechas. |
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
- Client mantiene acceso a metadatos de expedientes asignados y denegación
  documental. Ampliarlo requiere una política de recursos explícita y pruebas
  de aislamiento; ocultar botones no constituye autorización.
- El criterio OE-2 conserva su redacción aprobada sobre cuatro etapas. La
  [propuesta de recursos](procedural-resources-scope.md) distingue las tres etapas
  del CNPP y recursos vinculados a resoluciones, sin modificar ese criterio.
  Corregirlo literalmente requiere autorización explícita; implementar las dos
  transiciones no cierra recursos ni el cómputo automático exigido.
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
