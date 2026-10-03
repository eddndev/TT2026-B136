# Ingresar con certificado en Qadra

Este recorrido permite al Owner usar su certificado interno vinculado como
primer factor, seguido del MFA habitual. El servidor debe habilitarlo
expresamente; permanece deshabilitado por defecto. No registra un certificado,
emite claves ni firma documentos del expediente. La CA interna de demostración
no acredita FIREL, e.firma ni servicios de un PSC.

La [guía de vínculo](owner-certificate-interface.md) explica cómo registrar el
certificado y descargar su recibo público. La
[guía de operación](owner-certificate-login-operations.md) describe la
habilitación del servidor. Tener un recibo no demuestra vigencia actual ni
garantiza que el servidor permita el acceso.

## Recorrido de acceso

1. En la pantalla de acceso, selecciona **Ingresar con certificado**. Qadra
   consulta su disponibilidad en ese servidor. Si no está disponible, utiliza
   **Volver a contraseña**; seleccionar el método no inicia una sesión ni
   consulta una cuenta concreta.
2. Selecciona el archivo JSON de **Descargar recibo**, obtenido en **Mi
   certificado**, en **Recibo público del vínculo**. Debe describir un vínculo
   sin retirar. La pantalla muestra la cuenta Owner, el UUID del vínculo, la
   huella del certificado y el nombre declarado en ese recibo. El archivo se
   interpreta localmente; la preparación envía sólo los dos UUID.
3. Pulsa **Preparar acceso** y luego **Descargar bytes de acceso**. La descarga
   `acceso-certificado.bin` contiene exactamente **182 bytes** de un intento
   nuevo y temporal. Conserva ese archivo binario sin convertirlo a texto.
4. Firma esos bytes fuera del navegador con la clave correspondiente al
   certificado. Selecciona sólo la firma binaria de **384 bytes** en **Firma
   separada de acceso**. La aplicación no solicita la clave privada, su
   contraseña, un archivo PFX o un dispositivo de custodia.
5. Pulsa **Comprobar firma** una sola vez. Si la prueba es admitida, aparece
   **Un paso más.**; introduce tu código TOTP o un código de recuperación aún
   sin usar y selecciona **Verificar y entrar**. La prueba de firma por sí sola
   no abre la sesión ni recupera un segundo factor perdido.

La declaración de registro de **150 bytes** y su firma pertenecen a otra
operación. No sirven para este acceso. Cada nueva preparación requiere firmar
sus nuevos 182 bytes, incluso si la cuenta y el certificado son los mismos.

## Firma externa

En el equipo que custodia la clave correspondiente, ejecuta:

```sh
openssl dgst -sha256 -sign clave-privada.pem \
  -sigopt rsa_padding_mode:pkcs1 \
  -out firma-acceso.sig acceso-certificado.bin
```

Sustituye los nombres por tus archivos y selecciona únicamente
`firma-acceso.sig` en Qadra. Una clave cifrada puede solicitar su contraseña
localmente. Ni la clave ni esa contraseña se adjuntan al navegador. El tamaño
de una firma no prueba su autenticidad: el servidor verifica RSA, la cuenta,
el vínculo y la confianza publicada actuales.

## Vencimiento, cambio de método y resultados inciertos

El intento tiene una duración limitada. Si vence, se rechaza la prueba o falla
su respuesta, Qadra descarta el desafío y la firma de ese intento. **Preparar
otro intento** requiere una nueva descarga y otra firma externa. No se repite
el envío anterior ni se considera una respuesta perdida como acceso concedido.
Un rechazo de MFA tampoco habilita reutilizar la prueba anterior.

Volver a contraseña, abrir recuperación de contraseña o seleccionar otro
recibo abandona el intento. Las respuestas tardías no restauran el desafío ni
abren otra cuenta. La identidad devuelta tras MFA debe ser el mismo Owner
seleccionado; una cuenta distinta o un rol diferente se rechazan antes de
instalar la sesión.

El intento sólo vive en memoria. No se incluye en la recuperación de borradores
de editores, no se guarda al recargar la página y no conserva el archivo
original del recibo. La selección pública puede permanecer para preparar otro
intento explícito; el token, la firma y el desafío MFA no se recuperan.

Retirar el vínculo o sustituir su confianza invalida las capacidades de acceso
derivadas de ese vínculo. La contraseña y MFA mantienen su recorrido
independiente. Si se pierde autorización después de confirmar un retiro, el
resultado puede requerir consultar el UUID exacto tras entrar de nuevo mediante
contraseña y MFA; no debe repetirse el retiro automáticamente.

## Fronteras del cliente y evidencia

El cliente comprueba JSON, Base64, tamaños y correspondencia de cuenta, vínculo
y huella. El máximo del archivo se deriva de los límites del recibo público,
incluidos raíz, certificado, CRL y texto escapado. No interpreta ASN.1 ni
verifica certificados o firmas RSA en JavaScript. El recibo conserva confianza
histórica: una raíz, revisión o generación distintas en un desafío nuevo no se
rechazan por su sola diferencia con aquel recibo. La autoridad actual sigue
perteneciendo al servidor.

El transporte reutiliza `createApi`, su control de intentos de autenticación y
el formulario MFA existente. La disponibilidad deriva del servidor, sin otra
opción de activación en la interfaz. El plazo local usa tiempo monotónico desde
el inicio de la solicitud y descuenta su latencia; no sustituye el vencimiento
que comprueba el servidor.

La verificación local aprobó **36 pruebas Node**: once del nuevo cliente y
25 de intentos de autenticación existentes. Aprobaron **cinco recorridos de
navegador con HTTP controlado en 20.9 s** y **siete regresiones previas de
autenticación**. Los recorridos controlados incluyen indisponibilidad,
contenedores privados rechazados, recibo histórico, descarga exacta, MFA,
respuestas tardías, vencimiento y rechazo de cuenta o rol discrepantes. Se
inspeccionaron cuatro capturas de escritorio y móvil, conservando el diseño
Qadra sin desbordamiento observado. Estos resultados no acreditan por sí solos
un recorrido de navegador con RSA y servicios reales ni una instalación.


El recorrido con servicios reales aprobó **1/1 en 14.0 s**; el comando completo
con preparación de fixtures duró **208.382 s**. Usó PostgreSQL 16.15,
Valkey 8.1.10 y RSA/OpenSSL externos: registró el vínculo, descargó y firmó los
182 bytes exactos, admitió sólo MFA, abrió la misma cuenta y leyó el recibo.
Luego cerró expresamente esa sesión y volvió por contraseña con otro código
de recuperación antes de retirar y volver a consultar la evidencia. Conservó
todas las aserciones anteriores. No atribuye al retiro la revocación de una
sesión que ya se había cerrado; esa invalidación tiene su aceptación backend.

El intento previo llegó al retiro bajo la sesión derivada del mismo vínculo y
recibió 401: la retirada invalida esa autoridad. Se corrigió la preparación de
la aceptación, sin cambiar permisos, tiempos límite ni aserciones de respuesta.
La versión privada instalada no habilita todavía esta funcionalidad.
