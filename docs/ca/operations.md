# Manual del operador y preparación del servicio CA

Este manual describe la implementación integrada. DNS y HTTPS del origen CA
ya están preparados; el servicio permanece cerrado hasta el enrolamiento real
del operador. La instalación y la aceptación humana del acceso Owner se
distinguen en el [estado de la entrega](README.md). Los ejemplos contienen
marcadores, no credenciales ni una configuración lista para producción.

## Responsables y acceso

El operador designado es `eddndev@gmail.com`. Su identidad pertenece al servicio
CA y se fija por UUID y correo exactos en la configuración. La cuenta Owner
del despacho, `hatziryvh@gmail.com`, conserva sus propios accesos y no puede
operar la CA por ser Owner. El registro CA usa internamente `Client` como formato
de credenciales; el servicio no le concede autoridad de despacho.

El acceso necesita contraseña y un segundo factor. En el portal, introducir
el correo del operador y su contraseña; después, el TOTP de seis dígitos o un
código de recuperación disponible. Cada código de recuperación aceptado se
consume. Si el factor se rechaza, volver a iniciar sesión. La recuperación
disponible sustituye ese segundo factor; no restablece una contraseña olvidada
ni crea otra cuenta. No hay alta o restablecimiento por correo en este portal.

Los plazos de sesión proceden del servidor. La interfaz muestra el tiempo
restante y dispone de **Cerrar sesión**. Contraseña, desafíos y bearer se
mantienen en memoria; recargar la página requiere otro acceso. Tras volver a
entrar, las operaciones persistidas se consultan en **Historial**.

## Esquema adoptado en el destino

La CA reutiliza PostgreSQL y Redis de la instalación. La identidad CA reside
en `qadra_ca.users` y `qadra_ca.audit_events`, dentro de la base explícitamente
seleccionada. Su contrato está en
[`schema.sql`](../../crates/infrastructure/src/ca_identity/schema.sql) y los
[validadores de catálogo](../../crates/infrastructure/src/ca_identity/catalog.rs).
El esquema y ambas tablas deben pertenecer al propietario de la base. La
conexión administrativa y la de ejecución usan exactamente
`search_path=qadra_ca,pg_temp`; se comprueban también base, `current_user` y
`session_user` esperados.

Antes del alta, un administrador debe preparar y revisar ese esquema y un rol
de ejecución propio de CA. El comando de enrolamiento no crea roles, esquemas
ni permisos. El rol operativo tiene lectura de ambas tablas, inserción en la
auditoría y actualización sólo de `recovery_codes`, `revision` y `updated_at`
en usuarios. No tiene inserción de cuentas ni actualización general. La apertura
rechaza autoridad excesiva, herencia privilegiada, propiedad del esquema y
acceso al esquema `public`, entre otras condiciones de
[`permissions.rs`](../../crates/infrastructure/src/ca_identity/permissions.rs).

La adopción usa `qadra_ca_runtime`, sin membresías, en la base existente `qadra`
y con propietario `qadra_admin`. El preflight del 12 de octubre observó ausencia
del esquema/rol CA, `USAGE` de `public` concedido a `PUBLIC` y también un grant
explícito a `qadra_runtime`. Antes de abrir CA, retirar el grant heredado de
`PUBLIC` conservando el de `qadra_runtime` y verificar acceso del despacho. No
retirar permisos de otros roles sin revisar su uso. El rol CA se crea sin
superusuario, creación de roles/bases, herencia, replicación ni bypass de RLS;
su contraseña se genera y conserva únicamente en configuración privada.
Crear el esquema desde `schema.sql` como propietario en una transacción; dar
al runtime CA los permisos mínimos descritos arriba. No instalar sobre un
esquema/rol previo sin conciliar su autoridad. La adopción en VPS3 se completó el 13 de octubre de 2026, después de activar
la release `v0.1.8`. Se comprobó ausencia de cuentas CA y conservación del acceso
explícito del despacho. El archivo privado de alta quedó preparado con modo
`0600`; repetir la provisión sobre ese estado exige conciliación previa. No se
deben trasladar los permisos amplios del despacho al rol CA para superar la
validación. El esquema ya existente y su propietario se contrastan antes de
aplicar cambios; el archivo SQL no es una migración idempotente.

## Configuración privada del alta

Preparar localmente un archivo regular privado, propiedad del usuario que
ejecutará el comando, con modo `0600`. No se aceptan enlaces simbólicos, campos
desconocidos ni archivos mayores de 64 KiB. La configuración de alta es distinta
de la del servicio y no exige un UUID previo:

```json
{
  "database_name": "BASE_APROBADA",
  "operator_email": "eddndev@gmail.com",
  "kek_base64": "BASE64_DE_LA_KEK_CA_DE_32_BYTES",
  "admin": {
    "database_url": "URL_PRIVADA_DEL_PROPIETARIO_CON_SEARCH_PATH_CA",
    "database_role": "ROL_PROPIETARIO_APROBADO"
  },
  "runtime": {
    "database_url": "URL_PRIVADA_DEL_ROL_CA_CON_SEARCH_PATH_CA",
    "database_role": "ROL_CA_RUNTIME_APROBADO",
    "redis_url": "URL_PRIVADA_DEL_REDIS_EXISTENTE"
  }
}
```

Los roles administrativo y operativo son distintos. La KEK CA de 32 bytes
protege la credencial TOTP almacenada y debe ser la misma en la configuración
posterior del servicio. Mantenerla, junto con las URLs privadas, fuera de Git,
chat, argumentos, logs y capturas. La contraseña del operador no forma parte
de ninguno de los dos archivos de configuración.

## Alta inicial por terminal

Con el esquema y las dependencias revisados, invocar el binario aprobado:

```bash
/RUTA/QADRA_CA_APROBADO enroll-operator \
  --setup-config /RUTA/PRIVADA/ca-setup.json \
  --enrollment-out /RUTA/PRIVADA/ca-enrollment-NUEVO.json
```

1. Introducir una contraseña de 12 a 1024 bytes y repetirla. La terminal desactiva
   el eco; no pasar estos valores por argumentos o variables de entorno.
2. El comando prepara las credenciales y guarda un archivo nuevo `0600` con
   `operator_id`, `operator_email`, `totp_secret_base32`, `otpauth_uri` y ocho
   `recovery_codes`. El archivo no incluye contraseña ni KEK.
3. Leer ese archivo mediante una herramienta local privada. Registrar el secreto
   o URI en el autenticador y conservar los códigos de recuperación de forma
   privada. No pegar el contenido en chat ni mostrarlo mediante logs o capturas.
4. Introducir el TOTP de seis dígitos en la terminal del comando. Se verifica
   contra el mismo secreto protegido que se guardará en la cuenta.
5. Sólo después de confirmar el factor y volver a comprobar el archivo, se
   insertan cuenta inicial y evento `ca.operator_enrolled` en una transacción.
   La salida estándar contiene únicamente el recibo con UUID y correo.

La entrega se crea sin sobrescribir archivos ni seguir un enlace existente;
se sincroniza y se relee antes de insertar. También se comprueban permisos,
propietario, identidad del archivo y sus bytes exactos tras la confirmación
TOTP. Un error de escritura, contraseña distinta, fin de entrada, cancelación,
factor incorrecto o archivo alterado antes de la inserción deja la cuenta sin
crear. La entrada por pipe permite tres líneas para pruebas automatizadas,
manteniendo los mismos límites; no usar comandos que expongan esas líneas.

Un archivo reservado puede quedar vacío o contener material de una tentativa
fallida. Su existencia no prueba el alta. Se conserva al fallar y no se reemplaza
automáticamente: un reintento sin cuenta utiliza una ruta nueva. Si se pierde la
respuesta durante el commit, conservar el archivo y hacer que el administrador
reconcilie el UUID/correo con la cuenta y su auditoría antes de repetir. Una cuenta
existente cierra el alta inicial antes de producir nuevas credenciales. Este
comando no elimina, reemplaza ni restablece cuentas.

## Configuración del servicio: valores de destino pendientes

Después del alta, crear otro archivo privado para el servicio. Copiar el UUID
del recibo, el correo, la misma KEK y las conexiones operativas revisadas. No
incluir la conexión administrativa ni el bundle de enrolamiento. El siguiente
ejemplo muestra los campos del código; el puerto y plazos son ilustrativos:

```json
{
  "bind": "127.0.0.1:18089",
  "database_url": "URL_PRIVADA_DEL_ROL_CA_CON_SEARCH_PATH_CA",
  "database_name": "BASE_APROBADA",
  "database_role": "ROL_CA_RUNTIME_APROBADO",
  "redis_url": "URL_PRIVADA_DEL_REDIS_EXISTENTE",
  "kek_base64": "BASE64_DE_LA_MISMA_KEK_CA",
  "operator_id": "UUID_DEL_RECIBO_DE_ALTA",
  "operator_email": "eddndev@gmail.com",
  "session_ttl_seconds": 3600,
  "idle_ttl_seconds": 900,
  "operations": {
    "python": "/RUTA/PYTHON_APROBADO",
    "worker": "/RUTA/OPS_APROBADO/ca/command.py",
    "root": "/home/qadra/qadra",
    "inspector": "/RUTA/QADRA_CA_APROBADO",
    "cli": "/RUTA/DESPACHO_CLI_APROBADO",
    "scripts": "/RUTA/PKI_APROBADA",
    "expected_root": "SHA256_HEXADECIMAL_MINUSCULO_DE_LA_RAIZ_ADOPTADA"
  }
}
```

El listener debe ser loopback y tener puerto no nulo. La duración absoluta
admite de 1 a 86400 segundos; la inactividad es `null` o un entero positivo no
mayor que esa duración. `operations` es opcional: sin esa sección se ofrece el
acceso CA, pero no se compone la ruta de operaciones. Con ella, las rutas son
absolutas y la huella esperada tiene 64 dígitos hexadecimales minúsculos.

La raíz indicada en `operations.root` es la instalación existente, que contiene
`data/ca`, confianza publicada, configuración, bloqueo y mantenimiento. No es
un nuevo directorio vacío de CA. El worker necesita el árbol compatible de
`ops/ca` y `ops/deploy`, el inspector `qadra-ca`, la CLI y los scripts PKI
revisados. Ese conjunto está instalado en la release literal
`v0.1.8-95a9aacf975e4a9cde5c4f8b60544fab3baff84f`; no basta copiar
`command.py` de forma aislada. La configuración del servicio debe conservar
esa revisión y su inventario aprobados, junto con el controlador ya instalado.

La invocación del servicio es:

```bash
/RUTA/QADRA_CA_APROBADO --config /RUTA/PRIVADA/ca-service.json
```

El arranque verifica la cuenta activa y su UUID/correo exactos. Las rutas de
acceso `/login`, `/mfa/totp`, `/mfa/recovery`, `/logout`, `/me`, `/session` y
`/activity` tienen el prefijo `/api/v1/auth`. Las operaciones usan
`POST /api/v1/ca/operations`; `/healthz` indica que el proceso atiende. La salud
por sí sola no acredita una operación PKI ni un recorrido de usuario.

## Adopción del servicio y HTTPS

El origen `ca.qadra.com.mx` dispone de DNS y HTTPS desde el 13 de octubre de
2026. El vhost independiente conserva ACME por HTTP y devuelve `503` por HTTPS
hasta preparar el operador y las unidades. La renovación usa el mecanismo
Certbot existente; no se amplió ni reemplazó el certificado de Qadra.

El destino es VPS3 con la cuenta de despliegue `qadra`. Conservar PostgreSQL,
Redis, la CA, TSA y las unidades de usuario existentes. El servicio CA debe
mantenerse disponible durante la parada de API/web que realiza el mantenimiento
CRL, para que el operador pueda consultar y retomar la operación. El renderer puro
[`ops/ca/host.py`](../../ops/ca/host.py) prepara las dos unidades de usuario y
el proxy de loopback con rutas de una release literal aceptada; no instala
archivos ni cambia servicios. Su adopción, permisos y arranque siguen pendientes;
HTTPS preparado no significa que estas unidades ya estén instaladas.

El runtime CRL existente controla `qadra-api.service` y `qadra-web.service`
mediante `systemctl --user`. La adopción del servicio CA debe conservar ese
usuario y su bus de servicios, sin incluir el proceso CA en la parada de Qadra.

La web de CA se construye separadamente de Qadra. En desarrollo y preview usa
el puerto 18088 y un proxy `/api` a `127.0.0.1:18089`, configurable mediante
`CA_API_PROXY_TARGET`. En el despliegue, `ca.qadra.com.mx` debe servir esa web
por HTTPS y dirigir `/api` al servicio CA de loopback, conservando la separación
respecto de la API del despacho.

Antes de adoptar el servicio hay que fijar y contrastar la huella de la raíz
existente, configurar los archivos privados, revisar el alcance del respaldo
para esquema/configuración CA y comprobar el nuevo origen. La conservación y
copia externa siguen [respaldos](../deployment-backups.md); los servicios y
controladores siguen [despliegue](../deployment.md). No regenerar raíz, TSA,
índice ni contadores como parte de esta instalación.

## Solicitudes y emisión

En **Solicitudes**, pegar únicamente un CSR público PEM de hasta 16 KiB.
Elegir `Partner` para el perfil de acceso Owner o `Firma documental` para el
perfil `document-signing`. El titular genera y conserva su clave privada fuera
de la CA. El campo no recibe claves privadas, contraseñas de clave ni bundles
de credenciales.

Completar **Titular revisado**, **Referencia de la solicitud** y **Criterio de
emisión**. Pulsar **Revisar solicitud** y contrastar sujeto, propósito y huellas
de CSR, clave pública y CA. La inspección verifica la solicitud y su firma;
la raíz se valida separadamente y su huella configurada fija la autoridad que
emitirá. Un CSR no es por sí mismo una prueba de pertenencia a esa raíz ni de
identidad civil del titular.

Marcar la revisión y pulsar **Confirmar y emitir**. Conservar el identificador
de operación y el recibo con serie, huella y referencia. Descargar el certificado
público cuando la emisión quede confirmada. El vínculo y la prueba de posesión
posteriores en Qadra son pasos separados; la emisión no crea una cuenta ni un
vínculo automático.

Si no llega el resultado, usar **Retomar esta emisión** o la misma entrada de
**Historial** después de otro acceso. La reanudación conserva la solicitud,
operador, perfil, raíz y serie reservada, y busca el resultado exacto. No enviar
la solicitud como una operación nueva para superar una respuesta perdida.

## Inventario público

En **Certificados**, consultar sujeto, propósito, serie, huella del certificado
y de la CA. La descarga selecciona la serie y huella exactas. Se muestran por
separado el estado del índice local, el intervalo de fechas del certificado y
su inclusión en la CRL registrada en Qadra:

| Estado mostrado | Interpretación |
| --- | --- |
| Emitido / Revocado / Vencido en el registro local | Estado del índice de la CA. |
| Dentro de vigencia / Vencido / Aún no vigente | Comparación de fechas firmadas con la consulta; no sustituye la validación de uso y revocación. |
| No incluido en la CRL de Qadra | La serie no aparece en la CRL publicada consultada. |
| Pendiente de publicar en Qadra | El índice local está revocado, pero esa serie aún no aparece en la CRL publicada. |
| Revocado en la CRL de Qadra | La serie consta en la CRL persistida que se consultó. |

El TSA existente, emitido por la misma CA, aparece como **Sellado de tiempo**
(`time-stamping`) y se puede descargar. Esa inspección es sólo de lectura: no
añade TSA a los dos propósitos de emisión, a la revocación del portal, al acceso
Owner ni a la firma documental. Los certificados anteriores no se borran por
vencimiento o por emitir un sustituto.

## Revocación local y publicación

Para un certificado admitido por el portal, seleccionar **Revocar certificado**,
revisar serie y huella exactas e introducir el motivo. Confirmar explícitamente
y pulsar **Confirmar revocación local**. El recibo confirma el cambio del índice;
el estado sigue pendiente de publicación. Ante respuesta incierta, retomar la
misma revocación y conservar su serie, huella y motivo originales.

En **Publicación**, pulsar **Actualizar estado** para observar revisión, número
de CRL, fechas, huellas y mantenimiento pendiente. La fuente es la confianza
persistida en Qadra, no sólo un archivo local. Si hay mantenimiento pendiente,
ir a **Historial** y retomar su operación antes de solicitar otra renovación.

Para una publicación nueva, revisar la revisión actual y confirmar la ventana
de mantenimiento. **Renovar y publicar CRL** reutiliza el
[procedimiento CRL existente](../deployment-crl.md): bloqueo compartido con
despliegue, candidato, detención comprobada de API/web, respaldo, marcador,
publicación con revisión esperada y relectura de la sucesora exacta antes de
reabrir Qadra. Se conserva la release y la CA. PostgreSQL y Redis siguen siendo
los servicios existentes de esa instalación.

No iniciar emisión, revocación manual, despliegue u otra renovación para eludir
una operación pendiente. Si se pierde la respuesta, **Retomar esta publicación**
conserva el UUID y la revisión original. Una sucesora exacta ya publicada se
reconoce sin crear otra CRL. Una discrepancia o estado parcial conserva el
bloqueo y requiere investigación; no se bajan contadores ni se elimina el
marcador para forzar el ingreso.

La publicación termina con recibo de revisión, número de CRL, huella de CA y
operación. Volver al inventario para comprobar el estado publicado. La aceptación
del rechazo de nuevos usos y de la evidencia histórica se completa en sus
recorridos de Qadra; este recibo no la sustituye.

## Historial y recuperación

**Historial** consulta emisiones, revocaciones y publicaciones paginadas.
Muestra operación, operador que registró la solicitud, fecha, estado y datos
del recibo. **Retomar** consulta o completa la misma operación pendiente; no
cambia sus parámetros ni el operador original. Una operación pendiente bloquea
otras mutaciones que podrían ocultar su estado previo.

Los registros privados se conservan bajo `data/ca/operations`,
`data/ca/revocations` y `data/ca/publications`. La publicación conserva además
el registro de mantenimiento en `maintenance/crl` y su marcador en `config`.
La auditoría de identidad CA está en `qadra_ca.audit_events`; la publicación de
confianza conserva la auditoría SQL del mecanismo existente. Preservar todos
estos datos con los respaldos. No editar recibos, copiar otra raíz ni borrar
operaciones para hacer que una reanudación acepte material distinto.
