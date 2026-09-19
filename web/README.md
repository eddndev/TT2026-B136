# Qadra: interfaz del despacho

Interfaz en Astro y Svelte para la API existente del prototipo. El contrato
está en [`docs/http-api.md`](../docs/http-api.md). `web/` es la aplicación de
navegador; `crates/web/` sigue siendo la capa HTTP de Rust. El directorio
`frontend/` conserva el ejemplo anterior.

La marca del prototipo es Qadra. Sus logos y favicon originales se sirven
desde `public/brand/qadra/`, sin depender de solicitudes a GitHub al abrir
la interfaz. La procedencia, revisión y licencia se conservan en
[`public/brand/qadra/README.md`](public/brand/qadra/README.md).

## Ejecutar

Requiere Node.js 24 LTS y npm. Desde este directorio:

```sh
npm ci
npm run dev
```

Abre `http://127.0.0.1:4321`. La pantalla de acceso se puede visualizar sin
backend. Para operar necesitas la API Rust, PostgreSQL, Redis y la PKI/TSA
descritos en [`docs/http-api.md`](../docs/http-api.md). No se crean cuentas ni
documentos de prueba al arrancar esta interfaz.

El proxy local reenvía `/api` a `http://127.0.0.1:3000`. Para otra dirección,
establece `API_PROXY_TARGET` en el entorno antes de iniciar Astro. En PowerShell:

```powershell
$env:API_PROXY_TARGET = 'http://127.0.0.1:3000'
npm run dev
```

La URL se configura en el servidor de desarrollo, nunca desde el navegador.
El cliente usa rutas del mismo origen; no necesita modificar CORS del backend.

```sh
npm run build
npm run preview
```

La compilación produce `web/dist`. El proxy también funciona en `preview`.
Al servir `dist` con otro servidor, configura su proxy inverso para `/api`
hacia el backend y conserva las cabeceras de autorización y descarga. El
proxy de Astro no forma parte de los archivos estáticos compilados.

Astro puede arrancar en segundo plano al detectar un agente. Usa
`npx astro dev status`, `npx astro dev logs` y `npx astro dev stop` para
consultar o detener ese proceso.

## Flujo disponible

1. Configurar el acceso inicial si la base no contiene usuarios. Guardar la
   clave TOTP y los códigos de recuperación antes de finalizar el alta.
2. Iniciar sesión con correo, contraseña y TOTP o código de recuperación.
   Un rechazo MFA consume el desafío y vuelve a pedir credenciales.
3. Abrir **Expedientes**. El personal usa un índice autorizado con estado
   administrativo, ficha completa/pendiente, NUC y carpeta; Client conserva
   únicamente título y referencia de sus expedientes asignados. El índice del
   personal pagina con cursor exclusivo, sin calcular totales. Sus filtros
   combinan parte literal del título, NUC y carpeta exactos, estado y ficha,
   distinguiendo mayúsculas y acentos. Elegir una tarjeta consulta el detalle
   antes de abrir **Resumen**. Owner y Litigator tienen un único formulario de
   **Nuevo expediente penal**, descrito abajo.
4. Consultar **Documentos** del expediente seleccionado. El servidor devuelve
   páginas de hasta 50 documentos y `has_more`; **Anterior** y **Siguiente**
   recorren los resultados. **Buscar** aplica una subcadena al nombre y el
   selector filtra por sellado. Cambiar los filtros vuelve a la primera página.
5. Subir un archivo de hasta 16 MiB. Se sugiere un nombre ASCII de hasta 124
   caracteres, compatible con `X-Document-Name` y el ZIP. El cuerpo se transmite
   junto con el tipo, la clasificación y las etiquetas opcionales mediante un
   único multipart. El servidor confirma archivo y clasificación en una sola
   transacción. Incluso una carga sin valores organizativos crea su primera
   revisión vacía. El listado se consulta después de la carga y del sellado.
6. Abrir una fila o usar el identificador de un documento del expediente consulta
   la versión actual por GET:
   nombre, versión, digest y estado. Esa lectura no dispara verificación.
   **Verificar integridad** comprueba explícitamente los cuatro componentes
   de la evidencia. La ficha conserva las pestañas Resumen, Verificación y
   Evidencia, con confirmación antes de sellar y descarga ZIP cuando hay sello.
7. Consultar el historial descendente y usar **Cargar versiones anteriores**
   para continuar desde el cursor devuelto por el servidor. Cada fila conserva
   su nombre y estado de sellado. La ficha distingue la versión actual de una
   histórica y dirige el sellado, la verificación y la descarga a ese número
   exacto. La descarga incluye la versión en el nombre del ZIP.
8. **Agregar versión** carga otro archivo al mismo documento con la versión
   actual conocida como condición. El archivo documental pasa a mostrar la
   versión nueva; las anteriores conservan contenido y evidencia. Si otra
   persona agrega una versión antes, el conflicto conserva el archivo y su
   nombre en el diálogo: **Consultar versión actual** actualiza la condición,
   pero se requiere otro clic en **Guardar nueva versión** para enviar de nuevo.
   Si se alcanza el límite de versiones, el archivo también se conserva y el
   envío queda bloqueado; se indica cargarlo como un documento nuevo.
9. **Editar clasificación** organiza el documento completo por tipo, clasificación
   y etiquetas. Los dos campos de texto son opcionales y admiten hasta 80
   caracteres cada uno; hasta 20 etiquetas individuales admiten 40 caracteres.
   Las comas y Unicode forman parte del valor, sin separar etiquetas ni cambiar
   mayúsculas o acentos. Una etiqueta pendiente se valida al guardar.
10. **Ver historial de clasificación** muestra las revisiones descendentes con
    fecha y correo/UUID del autor capturados en cada cambio. Se distingue de las
    versiones de contenido. Editar o limpiar la clasificación no cambia las
    firmas, sellos ni ZIP históricos. Un conflicto conserva el formulario;
    consultar los valores actuales y pulsar **Guardar mis cambios** confirma
    el reemplazo, sin reintentar automáticamente.
11. **Filtros de clasificación** busca tipo, clasificación y una etiqueta exactos,
    combinados con nombre y estado. Estos filtros distinguen mayúsculas y acentos
    y se conservan al paginar. Solo se filtran los valores actuales del documento.
12. Abrir **Participantes** en la navegación local del expediente. Owner gestiona
    todos; Litigator gestiona los asignados; Paralegal consulta los asignados;
    Client no consulta participantes. La identidad representada, la ficha/rol y
    la cuenta autora son recursos distintos. Registrar una persona no le da acceso.
13. **Agregar participante** abre el registro tipificado; **Registrar ficha
    pendiente** conserva el alta manual explícita. **Completar perfil** agrega
    una revisión tipificada a una ficha manual y conserva sus revisiones anteriores.
    **Editar participante** conserva el borrador ante conflictos. **Archivar** y
    **Reactivar** cambian solo el estado organizativo, sin nueva firma personal
    ni modificación de la identidad, rol, cuentas o membresías.
14. El directorio filtra nombre literal, rol manual exacto (solo fichas pendientes), tipo tipificado, perfil
    tipificado/pendiente y estado antes de paginar por cursor exclusivo.
    Las filas excluyen identificadores personales, contactos y certificados.
    Su historial conserva los valores y autor/fecha de cada revisión, manual
    o tipificada; no calcula totales a partir de una página.
15. Como Owner, crear integrantes y verificar la cadena de auditoría.

La navegación incluye Inicio, Expedientes, Documentos y Guía de uso; el personal
también accede a Agenda y Alertas. Dentro del
expediente, Resumen, Documentos, Participantes, Etapas, Audiencias, Resoluciones
y Plazos comparten contexto. Equipo
y Auditoría aparecen para Owner. El inicio ofrece accesos a operaciones y al
expediente seleccionado. No presenta recuentos de una página como totales del
despacho. En móvil, el menú se abre en un diálogo y permite cerrar con Escape.
Las rutas usan fragmentos de URL y respetan atrás/adelante sin recargar la sesión.

La identidad visual, los recursos de marca, la tipografía, los colores, las
proporciones de navegación y los componentes documentales se conservan. Las
pantallas de expedientes reutilizan las tarjetas, controles, iconos y estados
de Qadra. Los ajustes adicionales están en `src/styles/cases.css`, `src/styles/versions.css`,
`src/styles/metadata.css`, `src/styles/participants.css`, `src/styles/case-administration.css`
y `src/styles/stages.css`.

Los controles respetan los roles del backend. Owner ve todos los expedientes;
Litigator y Paralegal requieren asignación vigente para consultar documentos.
Client puede consultar los metadatos de sus expedientes asignados; la interfaz
no emite peticiones documentales para ese rol. El servidor conserva la autoridad
sobre permisos, reglas de negocio y criptografía.

## Resoluciones y notificaciones declaradas

**Resoluciones** abre las dos familias dentro del expediente. Owner y Litigator
pueden registrar, corregir y retirar según los permisos vigentes del servidor;
Paralegal consulta los expedientes asignados. Client no accede a esta sección
ni a sus fuentes. El contrato está en
[la API de hechos declarados](../docs/procedural-facts-api.md). La interfaz está
implementada; su campaña de navegador está aprobada localmente. Esto no sustituye
la evidencia HTTP y de restauración ya registrada para el backend.

1. Consultar el listado de resoluciones, filtrar por estado y recorrer sus
   páginas. Abrir una fila consulta el detalle; **Ver historial de resolución**
   permite cargar una revisión exacta y distinguirla de la actual.
2. **Registrar resolución** pide clase, emisor, tiempo, resumen y procedencia.
   Los datos desconocidos se eligen expresamente; no son valores automáticos
   de un formulario vacío. Clase y emisor requieren motivo cuando no constan.
   El subtipo es opcional.
3. Elegir **Solo fecha**, **Fecha y minuto**, **Fecha y segundo** o **No consta**
   conserva esa precisión. El desfase puede quedar no declarado o capturarse
   expresamente, incluido UTC. No se completan hora, segundo ni zona con los
   valores del navegador.
4. La procedencia admite nota del operador, referencia externa o resultado
   histórico de audiencia. El selector recorre audiencia, resultado, historia
   y detalle exacto; permite un resultado retirado y un acuerdo específico.
   Los soportes se eligen por lista, versiones y detalle, con su localizador.
   Una carga documental confirmada permanece guardada aunque no se confirme
   después el hecho; la preparación decide su admisión PDF/DOCX.
5. Desde una resolución, abrir sus notificaciones. Cada notificación conserva
   esa raíz como padre; el selector permite elegir otra revisión exacta del
   mismo padre, incluidas las retiradas. Registrar otra práctica requiere otra
   raíz; corregir conserva la identidad de la declaración.
6. **Registrar notificación** distingue carácter, medio, contexto y resultado
   declarados; destinatario previsto y receptor material se capturan por
   separado. Una persona puede desconocerse con motivo, describirse sin ficha
   o vincularse a una revisión histórica del directorio, incluso archivada.
   La representación tiene extremos y procedencia propios; no se infiere del
   perfil de defensor ni de una comparecencia.
7. El tiempo de recepción es opcional. Un efecto expresamente declarado tiene
   tiempo, afirmación y localizador en la procedencia principal; su captura
   no calcula ni acredita eficacia jurídica. Constancia y representación
   admiten hasta dos versiones documentales directas distintas, reutilizables
   con localizadores separados.
8. **Preparar registro** muestra valores normalizados, fuentes legibles y
   administración observada. **Confirmar registro** envía el comando y recibo
   preparados. Corregir o retirar requiere motivo; retirar conserva historia
   y fuentes, sin afirmar nulidad del acto.

Ante conflicto de revisión, el borrador se conserva y se exige comparar la
base actual antes de volver a preparar. Una respuesta incierta conserva el
comando y se concilia con **Consultar envío exacto**, sin reenvío automático.
La ausencia de la revisión objetivo no resuelve esa incertidumbre; los errores
de consulta tampoco prueban el resultado. La revocación de acceso no se trata
como una confirmación de fracaso o éxito.
El cierre administrativo bloquea cambios y conserva las lecturas autorizadas.

Los detalles separan tiempo del acto, autor/fecha de registro y administración
histórica. La captura administrativa original sin revisiones no inventa R1,
autor ni fecha. Los nombres, estados y huellas de las fuentes pertenecen a las
revisiones seleccionadas; no se refrescan a sus cabezas actuales. La interfaz
reutiliza los controles de Qadra y añade `src/styles/procedural-facts.css`.
Este módulo de hechos no activa automáticamente un plazo ni determina efectos
jurídicos. Sus revisiones exactas pueden seleccionarse en **Plazos** y sus cambios
alimentan la reevaluación durable según las políticas capturadas. Los recursos
y la activación automática conservan su alcance pendiente. La interfaz de
[alertas personales](#alertas-personales) y su contrato HTTP están implementados;
su aceptación integrada con persistencia y runtime sigue pendiente.

## Plazos del expediente

**Plazos** consulta la colección persistente del expediente seleccionado. Owner
puede gestionar todos los expedientes; Litigator gestiona los asignados y
Paralegal los consulta. Client no abre esta sección ni emite sus solicitudes.
Un expediente cerrado conserva lecturas e historia y bloquea las mutaciones.
Los contratos son [plazos](../docs/deadlines-api.md) y
[perfiles versionados](../docs/deadline-profiles-api.md). El backend persistente
ya está integrado en `main`; la interfaz y el selector de responsables aprobaron
sus campañas locales completas, incluido el navegador con servicios reales.
Consulta [el informe](../docs/verification-report.md) para sus resultados
reproducidos y su estado de publicación.

1. El listado ofrece estado **Activos**, **Todos** o **Retirados** y páginas
   ordenadas por identificador. No representa un orden cronológico ni calcula
   totales a partir del tamaño de página. Abrir una fila recupera su detalle
   actual, responsable capturado, resultado y atención.
2. **Registrar plazo** pide un título y un perfil por revisión exacta. El
   selector recorre el catálogo disponible en el expediente, incluidos perfiles
   globales y privados, o exclusivamente el global. Se consulta historia y
   definición antes de elegir. Una revisión retirada sigue siendo consultable,
   pero no seleccionable para una nueva captura. La interfaz consulta perfiles;
   su publicación, reemplazo y retiro siguen disponibles mediante la API Owner.
3. **Elegir responsable** pagina cuentas actualmente elegibles por correo y
   rol. Incluye Owner activos y Litigator/Paralegal activos con asignación al
   expediente, sin introducir UUID a mano. Elegir a alguien no concede acceso
   ni reserva la cuenta: registrar y corregir vuelven a comprobar elegibilidad.
   El responsable histórico se conserva aunque después pierda el acceso.
4. **Tipo de fuente** permite resolución, notificación, resultado de audiencia
   o fuente no identificada con motivo. Los selectores recuperan una revisión
   exacta. La notificación conserva la revisión de su resolución padre, incluso
   si existe una posterior. Un resultado permite seleccionar acuerdo o declarar
   que se usa el resultado completo; no confunde un UUID cero con ausencia.
5. **Declarar un inicio calificado** conserva finalidad, tiempo, declaración y
   localizador en la misma fuente cuando corresponda. Fecha, minuto, segundo y
   desconocimiento conservan su precisión; no se completan componentes desde el
   reloj o zona del navegador.
6. El calendario se elige por revisión exacta o queda explícitamente ausente.
   **Cantidad ordenada** distingue ausencia de cantidad declarada. El máximo
   del perfil se muestra separado y nunca rellena la duración concedida.
   Ámbito, incidencia y condiciones requieren elecciones explícitas de sí, no
   o desconocimiento con motivo. Las condiciones omitidas de una definición
   histórica pueden agregarse al corregirla; no aparecen satisfechas por defecto.
7. **Preparar plazo** muestra declaraciones, perfil, responsable, fuente y
   calendario exactos, resultado, bloqueos y pasos del cálculo. La casilla de
   revisión habilita **Confirmar plazo**; cuando existen bloqueos, la
   confirmación declara expresamente que se guardará ese resultado incompleto.
   Una fecha candidata no se muestra como vencimiento si falta el corte o la
   precisión que el perfil requiere.
8. **Corregir plazo** conserva identidad y exige motivo y revisión base.
   **Declarar atención** captura estado pendiente o declaración con precisión
   temporal, afirmación y localizador. La atención conserva el cálculo y no
   acredita presentación válida ni cumplimiento oportuno. **Retirar plazo** es
   terminal, exige motivo y conserva cálculo, atención e historia.
9. **Ver historial de plazo** pagina revisiones descendentes. Elegir una
   recupera su contenido exacto y la distingue de la actual; las acciones de
   modificación requieren volver a consultar la cabeza. El detalle separa
   insumos seleccionados, cabezas observadas, administración capturada, perfil
   y recibo. Consultar historia no vuelve a ejecutar la aritmética ni sustituye
   las referencias por sus versiones más recientes.

Un conflicto conserva el borrador y exige **Consultar base actual**, revisar
la comparación y **Usar esta base y conservar borrador** antes de preparar otro
cambio. Un resultado incierto permite **Consultar envío exacto**, sin reenvío
automático. Solo el código específico de revisión inexistente se interpreta
como ausencia de esa revisión; aun así, una ausencia temporal no acredita que
la escritura haya fallado. Caducidad, revocación y otros errores de consulta no
son una confirmación de fracaso o éxito. Cambiar de expediente o sesión y cerrar
el formulario descarta su estado local; no cancela una escritura ya enviada.

Registrar o corregir puede capturar una administración observada más reciente
sin cambiar el contenido revisado; la conciliación valida los recibos y las
fuentes correspondientes. Atención y retiro conservan el cálculo capturado.
El cliente no crea huellas ni realiza la aritmética: las reglas, autorización,
persistencia y auditoría siguen en el backend.

El servicio humano y Qadra V2 capturan políticas explícitas de seguimiento para
perfil, fuente y calendario. El consumidor durable compuesto en `serve` procesa
sus cambios y conserva revisiones técnicas, causa e historia. Los recorridos de
reevaluación API con reinicios y restauración, y de Qadra con backend real,
aprobaron en su entrega; sus resultados se conservan separados en el informe de
verificación. El [contrato de seguimiento](../docs/deadline-tracking-api.md)
distingue el cálculo histórico del vencimiento operativo: una aceptación
capturada no sustituye la comprobación actual de las dependencias.

La agenda combinada diaria, semanal y mensual está implementada y su aceptación
sigue en curso. La activación automática permanece pendiente. Las alertas
personales permiten configurar anticipaciones de 48/24 horas y otros valores,
con bandeja y lectura explícita; su aceptación con PostgreSQL y el runtime
integrado sigue pendiente. Los ejemplos sintéticos no acreditan perfiles jurídicos aplicables;
su calificación y fundamento primario requieren el trabajo separado descrito
en [las fronteras de reglas](../docs/deadline-rule-research.md).

## Calendarios jurisdiccionales

Owner abre **Calendarios jurisdiccionales** desde Administración; el personal
puede llegar desde Agenda sin seleccionar expediente. Owner publica, reemplaza
y retira; Litigator y Paralegal consultan; Client queda denegado. El contrato
está en [la API de calendarios](../docs/judicial-calendars-api.md).

La publicación solicita ámbito, entidades, cobertura, fuentes, siete reglas
semanales y excepciones expresos. El título y el ámbito quedan fijos desde la
primera revisión y aparecen en la confirmación. Reemplazar requiere motivo;
retirar conserva la historia y no admite nuevas revisiones de esa raíz.

El detalle ofrece mes y lista de días, con clasificación computable, excluida,
sin resolver o fuera de cobertura. Consultar historia selecciona una revisión
exacta. Los enlaces de fuentes se abren por acción explícita; el servidor no
archiva su contenido ni acredita su aplicabilidad jurídica.

Los conflictos conservan el borrador y requieren comparar la base actual. Un
resultado incierto se concilia consultando el recibo exacto, sin reenvío automático.
La vista global y el formulario mantienen el diseño existente en escritorio y
móvil, con estilos adicionales en `src/styles/judicial-calendars.css`. Este
catálogo no completa el cómputo automático de plazos ni sus alertas.

## Ficha penal y administración

**Nuevo expediente penal** requiere título (200 caracteres), referencia interna
(100), NUC y carpeta judicial (100 cada uno), autoridades registradas (200 cada
una) y de 1 a 8 descripciones manuales de delito (120 cada una). Las descripciones
conservan orden, comas, Unicode y espacios interiores; los duplicados literales
tras recortar extremos se rechazan. El texto pendiente se incorpora al guardar.
Información general es opcional, admite 1000 caracteres y saltos de línea;
normaliza CRLF a LF. Identificadores complementarios es opcional y admite 300.
Los límites cuentan escalares Unicode, no unidades UTF-16. Se rechazan controles
antes de recortar extremos, excepto LF en información general.

La alta completa usa un único POST JSON: crea expediente activo, ficha y registro
inicial de Investigación en una transacción confirmada por el servidor. La
referencia interna no se convierte en NUC. Los identificadores y autoridades
son datos manuales; la interfaz no acredita su validez oficial ni actos judiciales.
NUC y carpeta son únicos de forma literal entre las cabezas actuales de la
instancia, incluidos cerrados; un conflicto no revela otro expediente.

Un expediente con **Ficha penal pendiente** permite **Editar datos básicos** o
**Completar ficha penal**. Los anteriores sin revisiones muestran datos originales,
sin actor ni fecha ficticios; los creados por el POST básico de compatibilidad
tienen R1 real y también pueden estar pendientes. La revisión no determina si la
ficha está completa. Completarla no crea una etapa retrospectiva: conserva
**Etapa sin registrar**. Una ficha completa no puede eliminarse.

La edición conserva el borrador ante conflictos y requiere consultar/comparar
los valores guardados antes de **Guardar mis cambios**. Estado y etapa no forman
parte del reemplazo del formulario. **Ver historial administrativo** pagina
revisiones descendentes con valores, digest, fecha y actor/correo capturados. El
registro inicial de etapa referencia su revisión administrativa original, incluso
tras ediciones y cierre; no representa la fecha de un acto judicial.

**Cerrar administrativamente** y **Reactivar expediente** cambian solo el estado
con revisión esperada y confirmación. Owner o Litigator asignado pueden hacerlo;
Paralegal asignado consulta ficha e historia. Client no solicita esas proyecciones.
El cierre bloquea nuevas cargas, versiones, clasificación, sellado, cambios del
directorio y de la ficha. Conserva consultas, historia, verificación y ZIP con los
permisos vigentes; tampoco elimina membresías ni impide que Owner revoque acceso.
Reactivar no modifica participantes archivados ni el registro de etapa.

Una escritura en vuelo puede detectar un cierre de otra sesión: la interfaz
consulta el estado, bloquea guardar y conserva el archivo/formulario. Consultar
el estado después de una reactivación permite revisar y enviar explícitamente;
no se reenvía automáticamente ni se promete sincronización instantánea entre
ventanas. Un fallo de acceso elimina los datos protegidos del contexto afectado.
Un fallo de red no confirma rechazo ni éxito: consulta el índice/detalle antes
de repetir una alta o edición. Esta interfaz no usa claves de idempotencia.

## Estado y límites

- Sesión opaca, desafío MFA y secretos de enrolamiento permanecen solo en
  memoria. No se usa localStorage, sessionStorage ni cookies del cliente.
- Recargar requiere iniciar sesión y elegir el expediente otra vez. Los
  expedientes y documentos permanecen en PostgreSQL y reaparecen al consultar.
  La recarga no revoca la sesión anterior; **Cerrar sesión** solicita revocarla.
- Las respuestas de una sesión o expediente abandonados se descartan. Una
  respuesta de búsqueda anterior no sustituye resultados más recientes; los
  fallos de acceso limpian las filas y el detalle de documentos.
- Las páginas no forman una instantánea conjunta: pueden cambiar si otro
  usuario agrega documentos o modifica asignaciones entre consultas.
- La búsqueda abarca los nombres del expediente, no el contenido cifrado.
  El estado de sellado no equivale a una verificación vigente de la evidencia.
- Las asignaciones de acceso se gestionan mediante la API; su administración
  visual y el directorio general de miembros continúan pendientes. El selector
  de responsable de un plazo muestra cuentas ya autorizadas y no asigna acceso
  al expediente. No se presenta un formulario de asignaciones por UUID.
- El directorio admite fichas pendientes y once perfiles tipificados. Los
  datos declarados y el perfil de CA interna no acreditan identidad jurídica,
  FIREL oficial ni efectos procesales automáticos. No fusiona homónimos.
- Si un documento importado empieza en una versión posterior a 1, el historial
  muestra explícitamente su primera versión disponible; no inventa versiones
  anteriores. Las cargas posteriores no reemplazan evidencia histórica.
- Cambiar de versión descarta los resultados y archivos pendientes de la
  selección anterior. La ficha comprueba que el informe de verificación y las
  cabeceras del ZIP correspondan al documento y versión seleccionados.
- La clasificación actual tiene una revisión independiente de la versión del
  archivo. Los documentos existentes sin cambios de clasificación muestran
  revisión 0, sin inventar actor ni fecha. Limpiar o guardar los mismos valores
  crea otra revisión; no vuelve a 0. La procedencia se consulta en su historial.
- Si una respuesta de carga o guardado se pierde, el formulario se conserva y
  se pide consultar los datos guardados antes de repetir: un fallo de conexión
  no demuestra que el servidor haya rechazado la escritura.
- La TSA local produce evidencia técnica, no una constancia NOM-151 de un PSC.

## Identidades, perfiles y firma personal

El formulario permite registrar una identidad nueva o consultar y elegir una
existente en el expediente. Persona y órgano institucional tienen campos
separados. Los datos desconocidos requieren estado y motivo; no se inventan CURP,
nombre ni cédula. La cédula conserva ceros iniciales. Cada identidad y cada rol
seleccionan soporte por documento, versión, SHA-256 y página o sección, mediante
lista, historia y detalle exacto; no hay una consulta por fila. El registro admite
como máximo dos versiones documentales distintas, reutilizables en secciones.
Una carga confirmada se conserva si luego falla la ficha.

Los perfiles son imputado, víctima/ofendido, defensor, Ministerio Público,
asesor jurídico, juez de control, tribunal de enjuiciamiento, perito, policía,
supervisión de medidas cautelares y otro. Contacto/protección remiten a soportes
cifrados exactos: no se publican como contacto plano ni se envían mensajes.
La lectura explícita de detalle, historia y evidencia mantiene los permisos de
Owner/Litigator/Paralegal; la lista compacta no crea una reserva frente a Paralegal.

**Revisar identidad y coincidencias** no guarda ni reserva identificadores.
Cada candidato se consulta para elegirlo y volver a revisar, o se declara distinto
con motivo y soporte exacto. Una coincidencia de nombre no fusiona identidades;
la capacidad de 16 candidatos no permite omitir coincidencias o cambiar los datos
para evadirlas. Una ficha tipificada conserva su identidad; otra ficha del mismo
sujeto/tipo, incluida una archivada, se revisa en el directorio completo.

**Preparar registro** fija la revisión propuesta. Defensor y juez de control
requieren firma personal; otros perfiles personales la permiten y los órganos
no la admiten. El certificado público PEM/DER ocupa hasta 16 KiB. La descarga
binaria preserva los 218 bytes de la declaración; el recibo es legible. La firma
RSA-3072/PKCS#1 v1.5/SHA-256 se realiza fuera del navegador y se carga como archivo
separado de 384 bytes. No se admiten claves privadas, PFX ni contraseñas de claves.
Certificado, declaración, firma y borradores solo permanecen en memoria.

Cambiar valores, revisión base, decisiones de identidad o certificado invalida la
preparación. Un conflicto exige una consulta y comparación explícitas. Ante un
resultado incierto, la ficha conserva el envío y consulta su revisión exacta:
la conciliación unsigned usa el digest/revisión de origen; la signed compara
la declaración y evidencia exactas. No se atribuye un envío por nombre ni se
crean nuevos identificadores automáticamente. La identidad separada muestra la
revisión consultada sin atribuirla al envío y exige otra decisión explícita.

El detalle conserva la identidad vinculada a la revisión de la ficha.
**Consultar identidad actual** y su historia son consultas independientes.
**Editar identidad** crea una revisión propia; las fichas previas no se reescriben.
La evidencia personal se consulta expresamente y exporta JSON público, declaración
y firma binarias. Su leyenda describe la comprobación capturada con la CA interna
y la revisión atestada; no declara vigencia actual ni equivale al ZIP documental.
El cierre administrativo bloquea mutaciones y conserva lecturas autorizadas.

## Etapas y soportes exactos

**Resumen** conserva el registro inicial histórico. **Etapas** consulta un recurso
independiente: etapa actual, revisión y origen (inicial, adopción o transición).
El historial se abre bajo demanda, pagina por revisión exclusiva y muestra los
actos declarados separados de fecha y autor capturados por el sistema. El detalle
de soportes históricos llega en cada entrada, sin consultas por fila.
El contrato está en [`docs/case-stages-api.md`](../docs/case-stages-api.md).

Un expediente completo sin etapa admite **Registrar etapa actual**, con etapa
conocida, fecha, motivo y soporte obligatorios; no reconstruye transiciones.
Desde Investigación se declara la acusación para pasar a Intermedia. Desde
Intermedia se registran por separado emisión del auto, recepción y tribunal para
pasar a Juicio; referencia y constancia adicional de recepción son opcionales.
El mismo documento y versión pueden seleccionarse expresamente para ambos actos.
Motivo/nota admiten 1000 caracteres; tribunal/referencia, 200. No hay siguiente
avance desde Juicio, corrección o deshacer en este recurso. Recursos procesales y plazos requieren otros flujos; la programación de
audiencias se consulta en su propia sección.

Cada fecha elige precisión de día o instante y desfase UTC explícito. No se
inventan horas desconocidas ni se usa el desfase actual del navegador para una
fecha histórica. El servidor comprueba futuro y orden considerando intervalos
cuando falta la hora. Un registro no acredita por sí mismo validez jurídica.

**Elegir documento** consulta lista, versiones y detalle exacto en orden; confirma
nombre, versión y digest. No consulta clasificación ni verifica o sella de forma
automática. **Cargar soporte** reutiliza multipart y clasificación opcional; una
carga confirmada permanece guardada si después se rechaza la etapa. Que el archivo
se cargue no demuestra que pase la validación de formato PDF/DOCX del soporte.

El formulario presenta confirmación antes del envío, conserva borrador y último
comando ante conflicto y exige consultar/comparar la etapa. Si la arista dejó de
aplicar, no transforma el comando en otro avance. Un soporte cambiado exige
seleccionarlo y consultarlo de nuevo. Un resultado incierto consulta cabeza e
historia secuencialmente; una coincidencia no prueba qué envío se confirmó y no
provoca reenvío automático. Los controles de la misma sección esperan el trabajo
real de selectores, carga, registro y recargas visibles.

Owner y Litigator asignado gestionan; Paralegal asignado consulta; Client no
solicita etapas. Perfil incompleto y cierre administrativo impiden registros.
Un cierre en vuelo actualiza el estado administrativo conservando borrador y
soportes; una denegación elimina datos protegidos del contexto. El cierre no se
interpreta como una etapa ni como suspensión de términos judiciales.

## Audiencias y Agenda

Audiencias conserva citas declaradas y sus revisiones dentro de cada expediente.
El contrato es [`docs/hearings-api.md`](../docs/hearings-api.md). Owner y Litigator
asignado gestionan; Paralegal asignado consulta; Client no tiene acceso. El cierre
administrativo conserva lecturas y suspende también las mutaciones de audiencia.

Los cuatro tipos corresponden a la etapa consultada. Individualización exige
antecedente declarado de condena y versión documental exacta; Juicio no acredita
por sí solo una condena. Fecha, hora con segundos y desfase UTC se conservan,
incluso para fechas futuras. No se calcula un resultado por el paso del tiempo.

Alta y reemplazo se preparan y confirman por separado. Reprogramar conserva el
tipo y exige un motivo. Cancelar agrega una revisión y mantiene programación,
participantes y soporte históricos. Las fichas elegidas se vinculan por UUID y
revisión: editar o archivar después el directorio no sustituye esas referencias.
Un conflicto conserva el borrador y exige consultar y comparar explícitamente.
Una respuesta perdida consulta el recibo de la revisión exacta, incluyendo actor,
operación y digest. Un 404 provisional sigue incierto y nunca causa reenvío.

### Agenda combinada

**Agenda del despacho** reúne audiencias y vencimientos operativos mediante
una sola consulta autorizada a `/api/v1/agenda`, independiente del expediente
abierto. Owner consulta el despacho; Litigator y Paralegal sólo sus expedientes
asignados; Client no accede. El contrato está en
[la API de agenda](../docs/agenda-api.md) y la decisión en
[ADR-0038](../docs/adr/0038-authorized-combined-agenda.md).

**Vista de agenda** ofrece Día, Semana, Mes y Rango personalizado. La fecha de
referencia y **Periodo anterior** o **Periodo siguiente** desplazan las vistas;
las semanas comienzan el lunes y navegar meses conserva el día cuando existe.
El rango personalizado incluye **Desde** y excluye **Hasta**, con un máximo de
366 días. El desfase se declara expresamente; **Tipo de actividad** filtra
ambas familias o una sola. **Estado de audiencia** sólo afecta a las audiencias
y se deshabilita al consultar únicamente plazos. En móvil, los calendarios
semanal y mensual se desplazan horizontalmente dentro de su contenedor.

La consulta pide hasta 20 actividades por página y **Cargar más actividades**
acumula resultados. Una página vacía con continuación conserva el aviso de
consulta parcial. Las tarjetas se identifican por familia e identificador y
conservan la revisión mayor recibida; un mismo UUID en ambas familias representa
dos actividades. El orden mantiene segundo UTC, nanosegundo, audiencias antes
de plazos en un empate e identificador. Cambiar filtros descarta la continuación
y las respuestas tardías. **Actualizar Agenda** reinicia la consulta: las páginas
no forman una instantánea reservada ni permiten calcular un total del despacho.

Un plazo sólo aparece si el servidor comprueba seguimiento V2 aceptado, estado
activo, dependencias vigentes y vencimiento operativo. El cálculo histórico de
un plazo pendiente, cambiado, bloqueado, legado o retirado permanece en su
expediente. Atención declarada y cierre administrativo no eliminan por sí solos
una fecha vigente autorizada. Las tarjetas de audiencia conservan su fecha
original sin exponer sede, notas o participantes. Elegir cualquier actividad
revalida el expediente y abre la revisión exacta mediante una intención de un
solo uso; una denegación elimina sus actividades del contexto visible.

La implementación de esta agenda tiene su aceptación en curso, separada de los
recorridos aprobados de reevaluación. No activa plazos ni envía alertas.

## Alertas personales

**Alertas** abre **Mis alertas** desde la navegación principal; Inicio también
ofrece un acceso sin inventar un contador. Owner, Litigator y Paralegal consultan
su propia bandeja y preferencias. Client no ve la entrada ni emite solicitudes
de este módulo. El [contrato HTTP](../docs/alerts-api.md) y
[ADR-0039](../docs/adr/0039-durable-activity-alerts.md) separan lectura, resolución
del aviso y entrega al proveedor de correo.

La política de destinatarios exige membresía staff actual para las audiencias,
incluido Owner; los plazos se dirigen a su responsable autorizado. Consultar
todos los expedientes no suscribe todas las audiencias ni permite ver bandejas
ajenas. Esta interfaz no crea suscripciones o destinatarios arbitrarios.

1. **Lectura** filtra Todas o Sin leer; **Estado de alerta**, Activas o Activas
   y resueltas. **Consultar alertas** aplica los filtros y **Actualizar alertas**
   vuelve al principio. **Cargar más alertas** acumula por identidad de aviso.
   Una página vacía con continuación conserva el estado parcial; los filtros
   invalidan respuestas tardías y no se calculan totales a partir de una página.
2. Las tarjetas distinguen proximidad de audiencia o plazo, vencimiento sin
   atención declarada, revisión requerida y cambio de fecha próxima. Muestran
   contexto y revisión de origen capturados, generación y fechas exactas cuando
   existen. Esas fechas no acreditan un vencimiento operativo actual.
3. **Abrir audiencia** o **Abrir plazo** vuelve a consultar la alerta y autoriza
   el expediente antes de abrir su revisión exacta. **Volver a Alertas** conserva
   los filtros; la intención de apertura se consume una sola vez. Una revocación
   retira los avisos del expediente del contexto visible.
4. **Marcar como leída** es explícito, idempotente y separado de la atención del
   plazo. Abrir el recurso no cambia la lectura. Una respuesta incierta permite
   **Comprobar lectura** por GET sin repetir automáticamente la escritura.
5. **Preferencias de alertas** muestra valores del servidor para la cuenta.
   Las dos familias permiten hasta ocho anticipaciones de 1 a 720 horas,
   separadas por comas; vacío desactiva sus anticipaciones. Los cinco grupos
   tienen canales interno y correo independientes. Los valores iniciales del
   servidor son 48/24 y ambos canales activos, sin guardado automático al abrir.
6. Un conflicto conserva el formulario. **Consultar preferencias actuales**
   muestra la revisión y los valores guardados; **Guardar mis preferencias**
   exige una decisión explícita sobre esa nueva base. Un resultado incierto
   conserva el comando y permite **Comprobar guardado**, sin reenvío automático.

**Correo deshabilitado** indica falta de transporte configurado, aunque la
preferencia de correo esté activa. **Aceptado por proveedor** no significa
entregado al buzón ni leído. No se ofrece una acción de reintento de correo.
El aviso externo previsto es genérico y no expone datos del expediente.

La interfaz conserva tarjetas, estados, colores y navegación de Qadra; añade
`src/styles/alerts.css` y apila controles y acciones en móvil. Salir de la vista
invalida sus solicitudes; cerrar sesión borra filtros privados e intenciones.
Las pruebas focales del cliente y nueve recorridos con HTTP controlado
aprobaron, incluidos escritorio de 1440 y móvil de 390 píxeles. Esas pruebas
no son aceptación con servicios reales: la composición PostgreSQL/runtime,
generación durable y entrega externa conservan su aceptación integrada pendiente.
Los resultados ejecutados se registran en el informe de verificación.

## Verificación

Ejecutar una sola suite local a la vez. Un único coordinador administra las
compilaciones, las pruebas y sus servicios de demostración. No solapar
compilaciones ni campañas. Mantener un trabajo de compilación Cargo, un hilo
de pruebas Rust y un worker de navegador.

Si `/tmp` usa tmpfs, configurar los temporales en disco antes de la campaña.
Desde la raíz del checkout activo, comprobar que su sistema de archivos está
en disco y preparar un directorio privado:

```sh
mkdir -p output/tmp
chmod 700 output/tmp
export TMPDIR="$PWD/output/tmp"
export CARGO_BUILD_JOBS=1
export RUST_TEST_THREADS=1
```

Conservar estas variables en el coordinador y sus procesos hijos, incluido
`scripts/web-demo.sh`. No borrar archivos ajenos en `/tmp`, el checkout ni el
directorio temporal. Registrar ese entorno junto con los resultados; un error
de carga de recursos no identifica por sí solo su causa exacta.

Después, desde `web/`, ejecutar los controles secuencialmente:

```sh
npm test
npx playwright install chromium
npm run test:e2e -- --workers=1
npm run format:check
npm run build
npm audit
```

Las pruebas de `tests/browser/` interceptan `/api/v1` con respuestas
reproducibles basadas en los DTO de Rust. Comprueban formularios, solicitudes
binarias, cabeceras, MFA, roles, consultas persistentes, búsqueda, paginación,
metadatos por GET, descarga, navegación y adaptación móvil. Incluyen respuestas
tardías de sesión, expediente y búsqueda, además de revocación de acceso.
Las pruebas retienen respuestas para comprobar que las acciones de una misma
ficha esperan sus operaciones y recargas automáticas. Cubren carga, nueva versión,
sello y edición de clasificación, incluso con historial abierto. Un fallo
transitorio de consulta conserva los datos ya confirmados; una denegación 403/404
limpia el contenido protegido. La edición espera una consulta de clasificación
en curso antes de permitir otro cambio.
El historial, las acciones sobre versiones históricas, la paginación por cursor
y los conflictos de carga tienen pruebas propias. La clasificación cubre
Unicode/comas, multipart, revisiones esperadas, historial, filtros, denegaciones
y respuestas tardías independientes del contenido. El directorio prueba Unicode,
permisos, filtros y cursores, historia, conflictos completos y de estado, errores
inciertos, revocaciones y formularios abandonados. La administración distingue
índice staff/Client, alta completa, perfil pendiente R0/R1, validación Unicode y
multilínea, conflictos de revisión/identificadores, historia, filtros y cierre con
borradores documentales y de participantes. Las pruebas de etapas cubren variantes,
fechas/desfases, referencias históricas exactas, conflictos, conciliación incierta,
cierre, denegación, páginas y consultas retenidas. Cada ejecución inicia un
servidor de desarrollo en un puerto libre, sin reutilizar otros servidores.
Las configuraciones aisladas usan puertos libres y `--ignore-lock` para coexistir
con el servidor de demostración. Ejecuta las suites simulada y real por separado
para que los resultados y sus diagnósticos sean independientes.
Estas pruebas simuladas no demuestran ejecución con PostgreSQL, Redis o la TSA.
Las capturas quedan en `web/test-results/` y no se versionan.

La prueba separada `tests/live/` usa la API Rust con PostgreSQL, Redis y TSA
locales desechables. Se ejecuta desde la raíz con `scripts/web-demo.sh`, que
prepara su entorno y usa `playwright.live.config.mjs`. El recorrido agrega dos
versiones con nombres diferentes, sella ambas, descarga la evidencia histórica,
compara los bytes del ZIP original y vuelve a consultar tras iniciar otra sesión.
Un segundo recorrido prueba carga clasificada, conflicto entre dos sesiones,
limpieza, persistencia y ZIP idénticos antes/después de clasificar.
Las capturas quedan en `web/test-results-live/`. Cada escenario que recibe una
respuesta API de error conserva `api-failures.json` con método, ruta anonimizada,
estado HTTP y código de error. El diagnóstico excluye consultas, cabeceras y
cuerpos completos; sustituye los UUID de la ruta. No uses bases de datos ni
credenciales de usuarios reales para esta prueba.

Las pruebas focales de plazos separan validación de contratos, renderizado de
campos y recorridos con HTTP simulado. Cubren declaraciones sin valores
inventados, confirmación explícita de bloqueos, selección paginada de
responsables, referencia histórica del padre de una notificación, corrección
con conflicto, atención, retiro y conciliación de un envío incierto sin
repetirlo. Estos resultados no sustituyen la campaña completa ni las pruebas
contra servicios reales; su evidencia se registra en el informe.

La verificación local más reciente se registra por separado de las pruebas
históricas del backend en [`docs/verification-report.md`](../docs/verification-report.md).

El formato se mantiene con `npm run format`. Los componentes, módulos y hojas
de estilo se dividen en archivos pequeños; `package-lock.json` es generado
por npm y conserva el árbol completo de dependencias para `npm ci`.

El recorrido real del directorio usa cuentas de fixture exclusivas para Owner,
Litigator, Paralegal y Client, con códigos de recuperación independientes. Prueba
edición y archivo desde dos contextos, persistencia al reingresar, consulta de
solo lectura, denegación a Client, revocación de una asignación con la misma
sesión y conservación byte por byte de la evidencia documental. Los registros
son de prueba en servicios aislados; no constituyen un ensayo de usabilidad con
personal real.

El recorrido de administración usa `fixture.caseAdministration` con cuentas
exclusivas Owner/Litigator/Paralegal/Client y casos básico R1, original R0 y oculto.
El seeder y sus credenciales pertenecen al entorno temporal protegido de
`scripts/web-demo.sh`. El escenario cubre alta penal, completar casos pendientes,
conflicto entre sesiones, cierre durante una carga, lectura de versiones y ZIP
idéntico, reactivación, persistencia y revocación. Los códigos de cada cuenta
son independientes de los recorridos anteriores. Las pruebas describen escenarios;
los resultados ejecutados se registran en el informe de verificación del proyecto.

Los escenarios reales de etapas usan `fixture.caseStages`: cuentas exclusivas
Owner/Litigator/Paralegal/Client, un expediente completo sin registro inicial y
uno oculto. Usan un PDF válido versionado como soporte técnico de prueba, sin
contenido jurídico real. El recorrido distingue registro inicial/adopción,
ambos avances, conflicto entre sesiones, V1 conservada tras V2, precisión de fecha,
historia y ZIP estable tras cierre. Otro recorrido verifica adopción por Litigator,
lectura por Paralegal, ausencia de solicitudes Client y revocación con sesión vigente.
Owner usa códigos 0/1/2 para sesiones de navegación y 3 para revocación; cada otra
cuenta usa su código 0. Los resultados se incorporan al informe únicamente después
de ejecutar `scripts/web-demo.sh` contra la API integrada.

Las regresiones tipificadas cubren los once perfiles en helpers, selección de
identidades/candidatos, preparación exacta, firma separada, base e identidad como
contextos distintos, conciliación unsigned/signed, edición separada, historia
mixta y denegación. Los recorridos reales adicionales usan cuentas exclusivas de
`typedParticipants`, certificado público de fixture y una clave individual solo
en `TT_LIVE_PARTICIPANT_PRIVATE_KEY` del proceso Node. OpenSSL firma fuera del
navegador; solo certificado y firma pública se entregan a la página. Estos guiones
son independientes: la política de cierre/revocación crea su propio expediente.
La ejecución de esos guiones se registra cuando el backend integrado está listo.

Los recorridos reales de audiencias usan `fixture.hearings`, cuentas exclusivas
y expedientes distintos para programación, permisos e individualización. El
seeder reserva el código 7 de `caseStages.owner` para crear esas cuentas; no
reutiliza los códigos de sus recorridos. La prueba de respuesta perdida descarta
únicamente la respuesta de un POST que ya confirmó la API real y consulta su
recibo; no fabrica respuestas de negocio. Otro recorrido compara byte por byte
el ZIP de la versión sellada exacta antes y después de programar y cancelar.
Los resultados ejecutados se incorporan al informe de verificación del proyecto.

## Recursos procesales del expediente

La sección Recursos permite registrar revocación y apelación con una resolución
histórica exacta, su soporte documental y personas recurrentes declaradas, con
fichas del directorio opcionales. La modalidad y precisión temporal requieren
una selección expresa; los datos desconocidos conservan ese estado.

Owner y Litigator asignado preparan y confirman altas, correcciones, actos,
archivo y reactivación. Paralegal asignado consulta; Client permanece denegado.
El cierre del expediente conserva la historia y bloquea escrituras. Archivar un
recurso sólo cambia su organización; el desistimiento se captura como acto
independiente.

Cada acto conserva uno o dos soportes exactos PDF/DOCX. Su revisión se encuentra
en la historia del recurso y puede corregirse sin sustituir capturas anteriores.
Una revisión histórica se abre expresamente; las respuestas inciertas se
concilian por recibo y nunca se reenvían automáticamente. Los conflictos
conservan el borrador y exigen aceptar una nueva base.

La captura no inicia plazos ni programa audiencias; esas asociaciones siguen
pendientes. El contrato está en [la API de recursos](../docs/procedural-resources-api.md)
y las verificaciones ejecutadas en [el informe](../docs/verification-report.md).
