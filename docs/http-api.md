# API HTTP local de evidencia documental

Esta API expone una rebanada vertical reproducible del flujo documental. Carga
un documento, lo cifra antes de persistirlo, lo firma, obtiene un sello RFC 3161
de la TSA local, verifica la evidencia, exporta un ZIP interoperable y registra
cada operación en la bitácora encadenada.

La ejecución no usa Cincel ni realiza una sustitución silenciosa de proveedor.
La TSA local demuestra el comportamiento técnico, pero no equivale a una
constancia NOM-151 ni aporta independencia de un tercero.

## Preparación

Se requieren `cargo`, `openssl`, `curl`, `unzip` y una instalación local de
`python3` para ejecutar el guion de demostración. La ruta más corta y aislada
es:

```bash
bash scripts/api-demo.sh
```

El guion crea una CA, un firmante, una CRL y una TSA en un directorio temporal;
arranca el servidor sobre un puerto libre y borra todo al finalizar.

Para una ejecución manual, prepare primero la PKI y la TSA con los scripts de
`pki/`, defina una KEK de 32 bytes y arranque el proceso:

```bash
export KEK_BASE64="$(openssl rand -base64 32)"
cargo run --bin despacho-cli -- serve \
  --bind 127.0.0.1:3000 \
  --data-dir runtime-data \
  --signer-cert ruta/firmante.crt.pem \
  --signer-key ruta/firmante.key.pem \
  --ca-cert ruta/ca.crt.pem \
  --crl ruta/crl.pem \
  --tsa-config pki/tsa.cnf \
  --tsa-dir ruta/pki-tsa
```

El servidor escucha solamente en `127.0.0.1:3000` de forma predeterminada.
`--bind 127.0.0.1:0` permite que el sistema operativo elija un puerto libre.

## Contrato

Todas las rutas usan el prefijo `/api/v1`, salvo el diagnóstico `/healthz`.
Los documentos se reciben como cuerpo binario, no como `multipart/form-data`.
El límite de cuerpo es 16 MiB.

| Método y ruta | Cabeceras | Resultado |
| --- | --- | --- |
| `GET /healthz` | Ninguna | Estado del proceso. |
| `POST /api/v1/documents` | `X-Actor`, `X-Document-Name` | Crea la versión 1, cifra y persiste; responde `201`. |
| `POST /api/v1/documents/{uuid}/seal` | `X-Actor` | Firma el digest, emite y verifica el sello local; responde el resumen sellado. |
| `POST /api/v1/documents/{uuid}/verify` | `X-Actor` | Verifica integridad, firma, certificado/CRL y sello de tiempo. |
| `GET /api/v1/documents/{uuid}/evidence` | `X-Actor` | Descarga un ZIP; `X-Document-Digest` contiene el digest esperado. |
| `GET /api/v1/audit/verify` | Ninguna | Verifica la cadena de auditoría y reporta el primer índice roto. |

Ejemplo mínimo:

```bash
base=http://127.0.0.1:3000
curl --fail-with-body \
  -H 'X-Actor: usuario-demo' \
  -H 'X-Document-Name: contrato.txt' \
  --data-binary @contrato.txt \
  "$base/api/v1/documents"
```

La respuesta de creación y sellado tiene esta forma:

```json
{
  "id": "7f7985b2-7faa-4f99-b0ca-105482974617",
  "version": 1,
  "name": "contrato.txt",
  "digest": "<sha256-en-hexadecimal>",
  "sealed": false
}
```

`X-Actor` es una etiqueta de auditoría obligatoria. No autentica al llamador ni
otorga permisos. El servidor debe permanecer local o detrás de una frontera de
confianza hasta incorporar sesiones y RBAC.

## Persistencia y evidencia

Cada registro vive en `DATA_DIR/documents/<uuid>.json`. Sus campos binarios se
codifican en base64 y el documento se almacena dentro del paquete cifrado
`DVLT1`; el texto claro no se escribe al repositorio. Las escrituras usan un
archivo temporal y reemplazo atómico, y un lock advisory serializa los cambios.
La bitácora reside en `DATA_DIR/audit.jsonl`.

El adaptador es local y monoproceso. El registro documental y la bitácora son
archivos separados, sin una transacción común: si falla el append de auditoría
después de persistir una mutación, la petición devuelve error aunque el cambio
documental pueda haber quedado escrito. PostgreSQL, una cola transaccional y el
aislamiento del trabajo bloqueante de OpenSSL son requisitos del adaptador de
producción, no garantías de este repositorio demostrativo.

El ZIP exportado contiene el documento descifrado y los artefactos necesarios
para verificarlo fuera de la aplicación: firma, certificado del firmante,
autoridad, CRL, sello y cadena de la TSA, además de instrucciones reproducibles
con OpenSSL. Exportar es una operación explícita que materializa texto claro en
la respuesta HTTP; quien lo descarga controla su almacenamiento posterior.

## Errores y estados

Los errores de aplicación tienen una envoltura estable:

```json
{
  "error": {
    "code": "document_not_sealed",
    "message": "<detalle seguro>"
  }
}
```

- `400`: UUID inválido.
- `404`: documento inexistente.
- `409`: transición incompatible, como sellar dos veces o exportar antes de
  sellar.
- `422`: cabecera, nombre o entrada inválidos.
- `500`: fallo interno sin exponer llaves, evidencia sensible ni detalles del
  backend.

Las decisiones de arquitectura y límites de seguridad se registran en
[`docs/adr/0011-local-document-workflow.md`](adr/0011-local-document-workflow.md).
