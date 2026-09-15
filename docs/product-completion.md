# Cierre funcional del prototipo web

## Alcance y evidencia

El cierre comprende gestión documental, gestión procesal penal, interfaz web,
conciliación de alcance y validación del producto. Los objetivos aprobados se
conservan en [la introducción](../latex/chapters/01-introduccion.tex); el catálogo
funcional está en [análisis y diseño](../latex/chapters/03-analisis-diseno.tex).
El [informe de verificación](verification-report.md) distingue resultados
reproducidos de mediciones históricas. Este documento organiza trabajo pendiente;
no declara implementadas las capacidades que enumera.

La aplicación `web/` integra la identidad Qadra con expedientes reales, rutas
autorizadas y consultas persistentes. Ofrece carga inicial, historial y selección
de versiones, sellado, verificación y descarga exactos. La clasificación actual
tiene tipo, clasificación, etiquetas y revisiones auditadas independientes del
contenido, con carga inicial atómica y filtros exactos. Las pruebas con HTTP
simulado y los recorridos contra servicios reales tienen evidencias separadas.
`frontend/` conserva el ejemplo anterior; el producto continúa en `web/`.

## Entregas y condiciones de cierre

| Entrega | Alcance verificable | Dependencias y evidencia requerida |
| --- | --- | --- |
| Consultas documentales e integración Qadra | Listado paginado, detalle, búsqueda literal de nombre y filtro de sellado; selección y creación de expedientes; carga, sello, verificación y evidencia en el expediente seleccionado. | Autorización vigente antes de exponer metadatos; cuatro roles, expedientes ajenos, revocación, filtros antes de paginación, eventos de consulta y errores. Pruebas HTTP, PostgreSQL y navegador. |
| Versiones documentales | Identidad estable, cifrado vinculado a UUID/versión, evidencia histórica inmutable y selección explícita de snapshot. | ADR-0019, append optimista, migración V1/V7, aislamiento, conflictos, restauración y evidencia byte por byte; resultados en el informe de verificación. |
| Clasificación documental | Tipo, clasificación y etiquetas organizativas con revisiones auditadas; búsqueda autorizada por metadatos. | ADR-0020, carga atómica, canon Unicode, revisiones esperadas, concurrencia, permisos y restauración sin alterar evidencia; resultados en el informe de verificación. |
| Identidad y firma personal | Resolver autenticación con certificado de socios, identidad del firmante y custodia de claves; altas, bajas, invitaciones y recuperación segura de credenciales. | La clave de firma configurada para el servidor no demuestra firma individual por usuario. Separar recuperación MFA de recuperación de contraseña. Definir contratos y probar revocación con sesiones existentes y fallos de entrega. |
| Expediente penal y participantes | Datos procesales del expediente; directorio de personas; roles procesales; transiciones con documentos habilitantes e historial. | Las asignaciones de acceso no representan participantes. Reglas probadas en dominio, transacciones y auditoría; vistas que conserven el diseño Qadra. |
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

| Caso de uso | Estado | Condición pendiente para cierre |
| --- | --- | --- |
| Registro de despacho y selección de plan | Parcial | Conservar bootstrap; conciliar selección comercial con instancia de un solo despacho y completar enrolamiento recuperable. |
| Ciclo de vida de miembros | Parcial | Invitaciones, directorio consultable, baja y protección del último Owner; selección por usuario en asignaciones. |
| Inicio de sesión y sesiones | Parcial | Certificado de socio, recuperación de contraseña e inactividad; contraseña/MFA y logout ya tienen implementación. |
| Control de acceso por perfil | Parcial | Extender la matriz a cada módulo pendiente y comprobar el registro de accesos exigido por el catálogo. |
| Registro y administración de expediente penal | Parcial | Añadir identificadores y atributos procesales, edición y cierre; título/referencia y asignaciones ya existen. |
| Directorio de participantes | Pendiente | Participantes, roles procesales, relaciones y operaciones con auditoría; independientes de cuentas de acceso. |
| Transición de etapa procesal | Pendiente | Reglas, documentos habilitantes, historial y rechazo de transiciones inválidas. |
| Audiencia y activación de plazos | Pendiente | Programación, cambios, cancelaciones y creación consistente de plazos vinculados. |
| Calendario judicial | Pendiente | Calendarios aplicables, inhábiles y excepciones, con pruebas normativas y de fechas. |
| Monitoreo de plazos y alertas | Pendiente | Vencimientos, entrega de alertas, reintentos y ausencia de duplicados. |
| Carga y clasificación documental | Parcial | Conciliar y probar la política de formatos admitidos y su rechazo. Carga cifrada, límites, clasificación atómica, filtros y versiones ya implementados; los bytes del archivo no se interpretan para validar su formato interno. |
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
- La autenticación implementada usa contraseña y MFA. Los certificados se usan
  para firma, y el servidor selecciona una credencial de firma al arrancar. La
  autenticación por certificado y la firma individual continúan abiertas hasta
  contar con implementación y pruebas correspondientes.
- Client mantiene acceso a metadatos de expedientes asignados y denegación
  documental. Ampliarlo requiere una política de recursos explícita y pruebas
  de aislamiento; ocultar botones no constituye autorización.
- Las alertas internas y la entrega por correo tienen contratos distintos. No se
  da por completada una notificación por persistir únicamente un vencimiento.
- La TSA local y la CA interna permiten verificar evidencia técnica. La campaña
  con un PSC externo permanece fuera de la entrega actual, según
  [la decisión sobre TSA local](adr/0009-local-timestamp-authority.md).

## Validación y dependencias externas

Antes de cerrar, la matriz de los casos de uso debe enlazar cada comportamiento
con su prueba y resultado. La cobertura de líneas y las pruebas criptográficas
existentes no prueban por sí mismas el catálogo procesal ni la usabilidad.
Los ensayos de navegador con respuestas simuladas se registran separados de
aquellos que ejecutan Rust, PostgreSQL, Redis y TSA.

La usabilidad requiere acordar participantes, tareas y condiciones, obtener
observaciones reales, corregir los problemas detectados y registrar los
resultados. Preparar el protocolo o simular un usuario no completa ese ensayo.
Mientras falte la participación externa, continúan las tareas de implementación
y validación técnica que puedan ejecutarse independientemente.

El despliegue público requiere además resolver los límites operativos enumerados
en [la revisión del backend](backend-review.md), revisar el modelo de amenaza de
firma HTTP y delimitar la ausencia de anclaje externo de auditoría. La
demostración local no se presenta como validación de producción.
