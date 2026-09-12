# Folio: interfaz del despacho

Interfaz en Astro y Svelte para la API existente del prototipo. El contrato
esta en [`docs/http-api.md`](../docs/http-api.md). `web/` es la aplicacion de
navegador; `crates/web/` sigue siendo la capa HTTP de Rust. El directorio
`frontend/` conserva el ejemplo anterior.

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
3. Cargar un archivo de hasta 16 MiB con nombre ASCII compatible con la
   cabecera `X-Document-Name`. Se transmite su cuerpo binario, sin multipart.
4. Conservar el UUID recibido. Abrir por UUID ejecuta una verificacion real.
5. Sellar con confirmacion, verificar los cuatro componentes y descargar el
   ZIP de evidencia. Los resultados proceden del servidor.
6. Como administrador, crear integrantes y verificar la cadena de auditoria.

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
- Expedientes muestra un estado pendiente: no simula casos, participantes ni
  asignaciones. El rol cliente sigue sin acceso documental.
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
cabeceras, MFA, permisos visibles, errores, descarga y adaptacion movil.
No demuestran una ejecucion con PostgreSQL, Redis, OpenSSL o la API real.
Para verificar el backend por separado usa `scripts/api-demo.sh` desde la
raiz con sus dependencias. Las capturas de navegador quedan en
`web/test-results/` y no se versionan.

El formato se mantiene con `npm run format`. Los componentes, modulos y hojas
de estilo se dividen en archivos pequenos; `package-lock.json` es generado
por npm y conserva el arbol completo de dependencias para `npm ci`.
