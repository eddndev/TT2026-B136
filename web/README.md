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
3. Abrir **Expedientes** para consultar los expedientes autorizados. Owner y
   Litigator pueden crear uno con título y referencia; el servidor asigna al
   creador. La lista se pagina en grupos de 50 y consulta un registro adicional
   para determinar si hay otra página. Seleccionar una tarjeta consulta el
   detalle del expediente antes de abrir su archivo documental.
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
12. Abrir **Participantes** en la navegación local del expediente. El directorio
    registra nombre, rol manual, organización y situación jurídica opcionales,
    independientes de cuentas y asignaciones. Owner gestiona todos los expedientes;
    Litigator gestiona los asignados; Paralegal consulta los asignados; Client no
    consulta participantes. Registrar a una persona no le concede acceso.
13. **Agregar participante** crea una revisión activa. **Editar participante**
    conserva el borrador ante conflictos y exige consultar/comparar los datos
    actuales antes de **Guardar mis cambios**. **Archivar participante** y
    **Reactivar participante** tienen confirmación y cambian solamente el estado
    organizativo, conservando la historia y cualquier edición concurrente del
    nombre o rol. No cambian la situación jurídica, cuentas ni membresías.
14. El directorio filtra nombre por subcadena literal, rol exacto y estado
    Activos/Archivados/Todos antes de paginar por UUID exclusivo. **Anterior** y
    **Siguiente** recorren páginas; no se calcula un total ficticio. Su historial
    descendente muestra valores y autor/fecha capturados por revisión.
15. Como Owner, crear integrantes y verificar la cadena de auditoría.

La navegación incluye Inicio, Expedientes, Documentos y Guía de uso; dentro del
expediente, Documentos y Participantes comparten contexto. Equipo
y Auditoría aparecen para Owner. El inicio ofrece accesos a operaciones y al
expediente seleccionado. No presenta recuentos de una página como totales del
despacho. En móvil, el menú se abre en un diálogo y permite cerrar con Escape.
Las rutas usan fragmentos de URL y respetan atrás/adelante sin recargar la sesión.

La identidad visual, los recursos de marca, la tipografía, los colores, las
proporciones de navegación y los componentes documentales se conservan. Las
pantallas de expedientes reutilizan las tarjetas, controles, iconos y estados
de Qadra. Los ajustes adicionales están en `src/styles/cases.css`, `src/styles/versions.css`,
`src/styles/metadata.css` y `src/styles/participants.css`.

Los controles respetan los roles del backend. Owner ve todos los expedientes;
Litigator y Paralegal requieren asignación vigente para consultar documentos.
Client puede consultar los metadatos de sus expedientes asignados; la interfaz
no emite peticiones documentales para ese rol. El servidor conserva la autoridad
sobre permisos, reglas de negocio y criptografía.

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
- Las asignaciones de acceso se gestionan mediante la API. La interfaz para
  elegir usuarios por nombre o correo requiere el directorio de usuarios y
  continúa pendiente. No se presenta un formulario de asignaciones por UUID.
- El directorio de participantes es organizativo y manual. Admite homónimos;
  no valida identidad legal, roles tipificados, identificadores oficiales,
  certificados, FIREL o condiciones procesales. Sus valores están en PostgreSQL,
  separados de los archivos documentales cifrados. Las audiencias, las etapas
  procesales y los plazos siguen pendientes. La API tampoco ofrece
  cambio/restablecimiento de contraseña.
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

## Verificación

```sh
npm test
npx playwright install chromium
npm run test:e2e
npm run format:check
npm run build
npm audit
```

Las pruebas de `tests/browser/` interceptan `/api/v1` con respuestas
reproducibles basadas en los DTO de Rust. Comprueban formularios, solicitudes
binarias, cabeceras, MFA, roles, consultas persistentes, búsqueda, paginación,
metadatos por GET, descarga, navegación y adaptación móvil. Incluyen respuestas
tardías de sesión, expediente y búsqueda, además de revocación de acceso.
El historial, las acciones sobre versiones históricas, la paginación por cursor
y los conflictos de carga tienen pruebas propias. La clasificación cubre
Unicode/comas, multipart, revisiones esperadas, historial, filtros, denegaciones
y respuestas tardías independientes del contenido. El directorio prueba Unicode,
permisos, filtros y cursores, historia, conflictos completos y de estado, errores
inciertos, revocaciones y formularios abandonados. Cada ejecución inicia un
servidor de desarrollo en un puerto libre, sin reutilizar otros servidores.
Ejecuta las pruebas simuladas y reales de forma secuencial: Astro admite una
sola instancia de desarrollo por proyecto, incluso con puertos diferentes.
Estas pruebas simuladas no demuestran ejecución con PostgreSQL, Redis o la TSA.
Las capturas quedan en `web/test-results/` y no se versionan.

La prueba separada `tests/live/` usa la API Rust con PostgreSQL, Redis y TSA
locales desechables. Se ejecuta desde la raíz con `scripts/web-demo.sh`, que
prepara su entorno y usa `playwright.live.config.mjs`. El recorrido agrega dos
versiones con nombres diferentes, sella ambas, descarga la evidencia histórica,
compara los bytes del ZIP original y vuelve a consultar tras iniciar otra sesión.
Un segundo recorrido prueba carga clasificada, conflicto entre dos sesiones,
limpieza, persistencia y ZIP idénticos antes/después de clasificar.
Las capturas quedan en `web/test-results-live/`. No uses bases de datos ni credenciales de usuarios
reales para esta prueba.

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
