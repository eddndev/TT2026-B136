# Criptografia local en el navegador

## Estado y alcance

El ayudante local esta implementado en la rama de trabajo y su aceptacion
tecnica esta en curso. Este documento describe su contrato y recorrido;
no afirma integracion ni despliegue. Las aceptaciones anteriores con OpenSSL siguen
siendo evidencia del flujo externo, no del nuevo ayudante.

La [decision arquitectonica](adr/0079-browser-local-cryptography.md) conserva
los contratos existentes de vinculo Owner, acceso con MFA y firma documental.
El ayudante prepara claves, solicitudes publicas y firmas en el equipo del
titular. La CA y la API del despacho no reciben la clave privada ni su contrasena.
La carga de la aplicacion y su modulo WASM puede usar la red; la operacion
criptografica local no envia esos secretos.

## Generar una clave y solicitar un certificado

En **Mi certificado**, para el Owner, o **Mi firma documental**, para el
Litigante, el componente **Generar clave y solicitud** ofrece este recorrido:

1. Introducir nombre comun, organizacion y pais. El nombre es declarado, no
   una acreditacion de identidad. Organizacion y pais deben corresponder a la
   CA que revisara el CSR; los valores iniciales son `Despacho Juridico Demo`
   y `MX` y pueden corregirse antes de generar.
2. Elegir una contrasena de al menos 12 caracteres, con un maximo de 1024 bytes
   UTF-8, y repetirla exactamente. Los espacios no se recortan.
3. Pulsar **Generar clave y solicitud**. El navegador obtiene la aleatoriedad
   de su fuente criptografica; si no esta disponible, la generacion falla.
4. Pulsar **Descargar clave cifrada** y conservar el archivo PEM y la
   contrasena. Qadra no puede recuperar ninguno de los dos.
5. Seleccionar el **Archivo de clave cifrada guardado**, volver a introducir
   la contrasena y pulsar **Comprobar copia guardada**. El worker debe abrirlo
   y comprobar que su clave publica corresponde al CSR de esta generacion.
6. Solo despues se habilita **Descargar solicitud CSR**. Compartir ese archivo
   publico con el responsable de la CA; no compartir la clave ni su contrasena.

La [CA existente](ca/operations.md) revisa el CSR y emite `partner` para acceso
Owner o `document-signing` para firma documental. Su politica de C/O sigue
vigente. Emitir el certificado no registra un vinculo en el despacho: el titular
debe realizar despues su prueba de posesion y confirmacion correspondiente.

## Firmar una operacion preparada

El bloque **Firmar en este dispositivo** se incorpora a estos puntos del flujo:

| Recorrido | Bytes que se firman | Paso posterior |
| --- | --- | --- |
| Vinculo Owner | Declaracion preparada de 150 bytes. | Confirmar el registro y consultar su recibo. |
| Acceso Owner | Desafio nuevo de 182 bytes. | Comprobar firma y completar MFA. |
| Vinculo documental | Declaracion exacta preparada para ese certificado y cuenta. | Confirmar el registro documental. |
| Documento | Bytes del PDF de la version seleccionada, tras revisar su contenido. | Preparar el consentimiento de esa firma. |
| Consentimiento documental | Declaracion exacta de cuenta, expediente, version y operacion. | Confirmar el envio y consultar su recibo. |

Seleccionar la clave cifrada y escribir su contrasena. Si el bloque pide un
certificado, seleccionar el certificado publico correspondiente; en otros
pasos ya se dispone del capturado en la preparacion. Revisar la huella esperada
y pulsar **Firmar localmente**. El worker comprueba la correspondencia entre
clave y certificado y devuelve la firma separada al formulario.

La firma preparada no registra, envia ni confirma por si sola la operacion.
El servidor vuelve a verificar permisos, contenido, proposito, confianza y
revocacion. En el acceso Owner sigue siendo obligatorio MFA. El documento y
su consentimiento son dos mensajes distintos: una firma no sustituye a la otra.

La alternativa externa permanece disponible: descargar los bytes exactos,
firmarlos con OpenSSL en otro proceso o equipo e importar solamente la firma.
Las guias de [vinculo Owner](owner-certificate-interface.md),
[acceso Owner](owner-certificate-login-interface.md) y
[firma documental](individual-signing-guide.md) conservan esos comandos.

## Formatos y limites

| Material | Formato y limite |
| --- | --- |
| Clave generada | RSA-3072, exponente 65537; PEM PKCS#8 `ENCRYPTED PRIVATE KEY`. |
| Proteccion de clave nueva | PBES2, PBKDF2-HMAC-SHA256 con 600000 iteraciones, AES-256-CBC, sal e IV aleatorios independientes de 16 bytes. |
| Clave importada | Mismo perfil cifrado; entre 1 y 2000000 iteraciones, hasta 64 KiB. Se admite el perfil OpenSSL existente sin recifrarlo. |
| Certificado publico | PEM o DER, hasta 16 KiB en el ayudante. Los selectores de registro mantienen sus propios formatos admitidos. |
| Solicitud publica | PKCS#10 PEM, hasta 16 KiB. CN/O de 1 a 128 bytes UTF-8, sin controles; C de dos letras ASCII mayusculas. |
| Firma separada | SHA-256 con RSA PKCS#1 v1.5; 384 bytes para RSA-3072. |

No se admiten PKCS12/PFX, claves sin cifrar ni otros algoritmos de clave en
este ayudante. El formato cifrado compatible usa CBC; no se presenta como
cifrado autenticado. La apertura incluye validacion del contenedor y de la
clave y su correspondencia con el certificado o CSR esperado.

La implementacion usa Rust compilado a WASM dentro de un Worker. `ring` realiza
la firma privada; `rsa` se limita a generacion y parseo de claves. La aceptacion
del riesgo CLI descrito en [ADR 0002](adr/0002-rsa-signing-crate-and-advisory.md)
no se extiende a la firma del navegador.

## Cancelacion, custodia y limites de confianza

Una operacion usa un worker cancelable. Al terminar o cancelarse se termina
ese worker; cambiar de cuenta, preparacion, certificado o version invalida
sus resultados tardios. Los campos de contrasena se vacian tras entregarlos
al worker y al terminar. **Limpiar datos locales** o **Limpiar firma local**
descarta el estado correspondiente. La clave y contrasena no se guardan en
los borradores recuperables, localStorage ni registros de la aplicacion.

Vaciar campos, liberar referencias y terminar el worker no garantiza borrar
fisicamente todas las copias en memoria, el archivo de intercambio o un volcado
del navegador. La pagina y sus recursos deben ser confiables: codigo alterado,
XSS, extensiones o un dispositivo comprometido pueden observar secretos durante
su uso. WASM no convierte el navegador en un dispositivo de custodia protegido.

El ayudante no cambia la CA, sus permisos, la politica de confianza ni los
procedimientos de perdida de clave. No incluye firmas de declaraciones de
participantes, verificador ZIP, emision automatica, nuevos propositos, FIREL,
PSC ni PAdES. La verificacion y aceptacion de este ayudante deben distinguirse
de las pruebas historicas de los flujos manuales.
