# Qadra: interfaz del despacho

Interfaz en Astro y Svelte para la API existente del prototipo. El contrato
esta en [`docs/http-api.md`](../docs/http-api.md). `web/` es la aplicacion de
navegador; `crates/web/` sigue siendo la capa HTTP de Rust. El directorio
`frontend/` conserva el ejemplo anterior.

La marca del prototipo es Qadra. Sus logos y favicon originales se sirven
desde `public/brand/qadra/`, sin depender de solicitudes a GitHub al abrir
la interfaz. La procedencia, revision y licencia se conservan en
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

El proxy local reenvia `/api` a `http://127.0.0.1:3000`. Para otra direccion,
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

La compilacion produce `web/dist`. El proxy tambien funciona en `preview`.
Al servir `dist` con otro servidor, configura su proxy inverso para `/api`
hacia el backend y conserva las cabeceras de autorizacion y descarga. El
proxy de Astro no forma parte de los archivos estaticos compilados.

Astro puede arrancar en segundo plano al detectar un agente. Usa
`npx astro dev status`, `npx astro dev logs` y `npx astro dev stop` para
consultar o detener ese proceso.

## Flujo disponible

1. Configurar el acceso inicial si la base no contiene usuarios. Guardar el
   secreto TOTP y los codigos de recuperacion antes de cerrar el enrolamiento.
2. Iniciar sesion con correo, contrasena y TOTP o codigo de recuperacion.
   Un rechazo MFA consume el desafio: la pantalla vuelve a pedir credenciales.
3. Consultar el inicio con el resumen de documentos abiertos, pendientes de
   sello y verificados en esta sesion. Cada contador abre su filtro documental.
4. Subir un archivo de hasta 16 MiB desde el selector o arrastrandolo al dialogo.
   Se sugiere un nombre ASCII de hasta 124 caracteres, compatible con la
   cabecera `X-Document-Name` y el ZIP de evidencia. Puede editarse antes de
   enviarlo. Se transmite el cuerpo binario, sin multipart.
5. Conservar el UUID recibido. Abrir por UUID consulta la verificacion del
   servidor; si responde `document_not_sealed`, abre una referencia pendiente
   para poder sellarla. La API no devuelve el nombre en esa respuesta.
6. Consultar las pestanas Resumen, Verificacion y Evidencia. Sellar con
   confirmacion, verificar los cuatro componentes y descargar el ZIP.
   Verificar y descargar se habilitan cuando el documento tiene sello.
7. Como administrador, crear integrantes y verificar la cadena de auditoria.

La barra lateral permite moverse entre Inicio, Documentos y Guia de uso;
Equipo y Auditoria aparecen para administradores. En movil se abre como un
menu plegable con cierre mediante Escape. Las rutas usan fragmentos de URL y
respetan los botones atras y adelante del navegador sin recargar la sesion.
La mesa documental permite buscar, filtrar por estado y alternar entre lista
y tarjetas. Los resultados de verificacion anteriores se descartan si una
nueva comprobacion falla.

Los controles respetan la matriz de roles del backend. El servidor conserva
la autoridad sobre permisos, reglas de negocio y criptografia. No se firma,
cifra ni verifica evidencia en el navegador.

## Estado y limites

- La sesion opaca, el desafio MFA, los secretos de enrolamiento y las
  referencias documentales permanecen solo en memoria. No se usa localStorage,
  sessionStorage, cookies del cliente ni persistencia de archivos en claro.
- Al recargar hay que iniciar sesion otra vez. La recarga no revoca el token
  anterior en el servidor: conserva su vencimiento de 24 horas. El boton
  **Cerrar sesion** llama al endpoint de revocacion; si falla, muestra el error.
- Las referencias de esta sesion no son un catalogo global. Se pierden al
  recargar o salir, mientras los documentos permanecen en el backend.
- La busqueda y los filtros solo abarcan esas referencias. No existen
  endpoints para listar, buscar globalmente ni consultar historial de versiones.
- Esta interfaz no ofrece gestion de expedientes, participantes ni
  asignaciones. La guia explica ese limite y el rol cliente sigue sin acceso
  documental conforme al backend de esta rama.
- La TSA local produce evidencia tecnica, no una constancia NOM-151 de un PSC.
- La API no ofrece cambio de contrasena, restablecimiento ni listado de usuarios.

## Verificacion

```sh
npm test
npx playwright install chromium
npm run test:e2e
npm run format:check
npm run build
npm audit
```

Las pruebas de navegador interceptan `/api/v1` con respuestas deterministas
basadas en los DTO de Rust. Comprueban formularios, solicitudes binarias,
cabeceras, MFA, permisos visibles, errores, descarga, navegacion, filtros,
estadisticas de sesion y adaptacion movil.
No demuestran una ejecucion con PostgreSQL, Redis, OpenSSL o la API real.
Para verificar el backend por separado usa `scripts/api-demo.sh` desde la
raiz con sus dependencias. Las capturas de navegador quedan en
`web/test-results/` y no se versionan.

La verificacion local mas reciente se registra por separado de las pruebas
historicas del backend en [`docs/verification-report.md`](../docs/verification-report.md).

El formato se mantiene con `npm run format`. Los componentes, modulos y hojas
de estilo se dividen en archivos pequenos; `package-lock.json` es generado
por npm y conserva el arbol completo de dependencias para `npm ci`.
