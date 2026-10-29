# 19. Versiones documentales inmutables y selección explícita

## Status

Aceptada. Amplía la persistencia por expediente de
[ADR-0016](0016-case-document-transactions.md) y las consultas de
[ADR-0018](0018-authorized-document-queries.md). Enmendada el 2026-10-29: el
veredicto histórico juzga el certificado en el instante de firma (véase la
enmienda al final).

## Context

Un documento puede recibir contenido corregido sin perder su evidencia anterior.
`DocumentRecord` conserva UUID y versión junto al contenido cifrado;
`document_aad` en `crates/domain/src/crypto/cipher.rs` autentica ambos en AES-GCM.
El formato `DVLT1` guarda la DEK envuelta y el payload; UUID y versión se
suministran como contexto al cifrar y descifrar, no se serializan dentro del
vault. Ese formato puede conservarse sin cambios.
La tabla anterior admitía una sola fila por UUID. Reemplazarla destruiría la
capacidad de volver al contenido y evidencia que se habían recibido y sellado.

Los archivos offline importados pueden contener una versión positiva distinta
de uno. Renumerar ese contenido rompe su contexto de cifrado. La firma y el
sello RFC 3161 cubren el digest del contenido; no firman el UUID, el expediente,
el número de versión ni el nombre. El actor autenticado que registra la
auditoría tampoco es automáticamente el titular de la clave del servidor.

## Decision

### Identidad y almacenamiento

`document_series` conserva un UUID permanente, su expediente inmutable y
`first_available_version`. `documents` conserva sus filas existentes con clave
primaria `(id, version)`. Su clave foránea compuesta impide mezclar expedientes.
Otra clave foránea diferida obliga a que exista la primera versión de cada raíz
al confirmar la transacción. La versión actual se obtiene como `MAX(version)`;
no existe un puntero mutable que pueda retroceder separadamente del contenido.

Las versiones posteriores se añaden de forma contigua. Cada carga cifra de
nuevo con DEK propia, el mismo UUID y el número siguiente; incluso contenidos
idénticos reciben un nuevo cifrado. Nombre, digest, contexto y vault quedan
inmutables. La evidencia puede pasar de ausente a capturada una sola vez por
versión. Añadir una revisión no copia ni reemplaza la firma, el sello, los
certificados o la CRL de otra.

El rol operativo tiene SELECT/INSERT sobre raíces y versiones, y únicamente
UPDATE(evidence) sobre versiones. Los triggers de secuencia toman el bloqueo
transaccional común de auditoría antes de consultar la cabeza. No necesitan
conceder UPDATE sobre la raíz para bloquearla. El orden y la serialización
global existentes se mantienen; este cambio no aumenta la concurrencia de
confirmaciones documentales.

### Preparación, autorización y concurrencia

`append` exige `expected_version`. La aplicación autentica y autoriza, compara
la versión actual, prepara la siguiente y autentica de nuevo. El almacén
revalida rol activo, pertenencia y cabeza en su transacción y confirma la fila
y `document.version_added` juntos. Una cabeza distinta produce conflicto; un
fallo de auditoría no deja una versión huérfana. El agotamiento de `u32` se
reporta explícitamente. Un reintento con la misma versión esperada no añade
otra revisión; el cliente consulta de nuevo y solicita una decisión del usuario.

Owner, Litigator y Paralegal pueden añadir versiones cuando su ámbito lo permite.
Historial y detalle usan el permiso de lectura documental. Sellado conserva
Owner/Litigator; Client continúa denegado. Las etiquetas organizativas no son
una regla de autorización y su implementación se aborda por separado.

Las operaciones de contenido mantienen una `DocumentVersionRef` exacta durante
preparación, comparación, auditoría y confirmación. Añadir una versión mientras
se sella otra no redirige el resultado. Las rutas antiguas sin versión resuelven
únicamente documentos con una sola versión disponible; una selección ambigua
responde `409 document_version_required`. Una operación que ya resolvió una
versión única puede confirmar esa misma versión aunque después se añada otra.

### Consultas y presentación

La colección documental y su detalle muestran la versión actual. Se selecciona
la cabeza antes de aplicar filtros por nombre o sellado: una revisión antigua
sellada no convierte a la actual pendiente en un resultado sellado.

El historial usa orden descendente y cursor exclusivo `before_version`, con
páginas de 1 a 100 elementos. Una nueva cabeza no desplaza los elementos ya
recorridos; aparece al refrescar la primera página. El historial informa la
primera versión realmente disponible, sin inventar revisiones anteriores a una
importación. Historial y detalle consultan metadatos y confirman su auditoría
antes de responder.

Qadra conserva sus componentes, estilos y marca. Las acciones usan UUID y
versión seleccionada, distinguen el contenido actual del histórico y descartan
respuestas de sesiones, expedientes o selecciones anteriores. Un conflicto de
carga conserva el archivo para revisión y no reintenta automáticamente.

### Migración y evidencia histórica

`migrations/0004_document_versions.sql` transforma claves y agrega raíces sin
actualizar los vaults ni la evidencia existente. Es reejecutable junto con las
migraciones administrativas anteriores. Deben detenerse todos los escritores
antes del corte; no se puede volver a un servidor que asume UUID único después
de habilitar múltiples versiones.

La importación conserva el número original y la reconciliación busca ese UUID
y versión, aunque posteriormente aparezcan otras. No se reescriben recibos ni
eventos históricos. El ZIP de una versión conserva su plantilla e insumos; la
selección exacta se informa mediante la ruta y cabeceras, sin modificar bytes
históricos para incorporar contexto que no estaba firmado.

El arranque operativo valida el esquema esperado y la coherencia de las raíces
y secuencias con consultas de metadatos. Un respaldo debe contener todas las
raíces, versiones, evidencia, auditoría y recibos. La recuperación compara ambas
versiones y sus exportaciones, además de reconciliar el origen preservado.

## Consequences

- El almacenamiento crece con cada versión; no se ofrece eliminación o edición
  destructiva de la historia ni reasignación de expediente.
- Las aplicaciones antiguas deben adoptar rutas explícitas para documentos
  con varias versiones. La API continúa aceptando su flujo con una sola versión.
- Verificar evidencia histórica usa la política del verificador actual y juzga el
  certificado del firmante en el instante de firma, según la enmienda de
  2026-10-29. Conservar bytes no garantiza un veredicto válido permanente; no se
  reemplaza material histórico con certificados o CRL actuales.
- Los nuevos eventos incluyen versión; el prefijo anterior no se reescribe.
  AES-GCM impide sustituir un vault entre contextos diferentes, pero una copia
  antigua íntegra puede seguir siendo internamente válida. El anclaje externo
  pendiente de [ADR-0007](0007-audit-chain-anchoring.md) sigue siendo necesario
  para distinguir esa restauración de un estado actual.
- La comprobación inicial de coherencia recorre metadatos de versiones; debe
  considerarse al medir el arranque con conjuntos grandes. No se ejecuta como
  un escaneo adicional por petición.
- Clasificación, participantes procesales, firma individual e identidad por
  certificado requieren sus propios contratos; el historial no los sustituye.

## Enmienda 2026-10-29: veredicto histórico en el instante de firma

### Context

La verificación de una versión sellada evaluaba el certificado del firmante en
la fecha de ejecución con la CRL capturada al sellar. Esa CRL tiene un
`nextUpdate` siete días posterior a su emisión, así que pasado ese plazo toda
verificación terminaba en un error interno, aunque la evidencia fuera íntegra.
La validación de registros históricos y la de firmas individuales ya evaluaban
el certificado en el instante de firma, igual que el validador del navegador de
[ADR-0085](0085-in-browser-evidence-validation.md).

### Decision

El veredicto histórico juzga el certificado del firmante en el instante de firma
con el material capturado: el `genTime` del sello de tiempo verificado para un
sello del servidor y la aceptación registrada para una firma individual, que
sólo se usa cuando el sello también verifica. Es la práctica de validación a
largo plazo: el artículo 32, apartado 1, letra b, del Reglamento (UE) 910/2014
(eIDAS) exige que el certificado fuera válido en el momento de la firma, y
ETSI EN 319 102-1 valida en un instante pasado demostrado por un sello de
tiempo.

El estado en la fecha de ejecución se informa en el detalle del componente
`certificate` como nota y nunca cambia el veredicto. Una CRL que ya no cubre la
fecha de ejecución no es un error. Una CRL vencida en el instante de firma, un
certificado fuera de vigencia, un emisor no confiable o una revocación listada
en la CRL capturada hacen fallar el componente. Sin un sello verificado no hay
instante de firma confiable: el certificado se evalúa en la fecha de ejecución
y una CRL que ya no la cubre deja el componente omitido con su motivo.

### Status

Aceptada. Reemplaza la evaluación en la fecha de ejecución descrita antes en
este documento.

### Consequences

- El contrato JSON del reporte no cambia; cambia el texto de `detail` del
  componente `certificate`.
- La exportación de paquetes por línea de comandos sigue exigiendo un estado
  válido en la fecha de ejecución, porque sus instrucciones ejecutan
  `openssl verify -crl_check` sin fijar un instante.
- La comprobación de la cadena de la TSA conserva la política del verificador
  OpenSSL actual, que la evalúa en la fecha de ejecución. ETSI EN 319 102-1
  (cláusulas 5.3 y 5.4.4) también valida el sello como firma básica en la fecha
  actual mientras no exista otra prueba de existencia; fijar la verificación en el `genTime`
  que el propio sello declara no se adopta.
- El sello se juzga en su `genTime` exacto; la precisión del sello no se
  aplica en este verificador.
- El sellado del servidor exige que el certificado del firmante sea válido en
  el `genTime` del sello nuevo con la CRL que se capturará. Si el proceso
  conserva una CRL ya vencida porque su renovación falló, el sellado se rechaza
  en lugar de crear una versión que nunca podría verificarse.
- Una serie listada en la CRL suministrada hace fallar el componente sin
  comparar su fecha de revocación con el instante de firma. Con la CRL capturada
  al sellar toda revocación listada es anterior a la firma. En la línea de
  comandos, `--crl` con una lista posterior puede nombrar una revocación
  ocurrida después de la firma: el detalle indica que la lista nombra la serie,
  sin fecharla en el instante de firma. ETSI EN 319 102-1 compara la fecha de
  revocación con el instante de firma, igual que el validador del navegador;
  adoptar esa comparación aquí queda como decisión pendiente.
- Sin un sello verificado, un componente `certificate` en `skipped` por una CRL
  que ya no cubre la fecha de ejecución no hace fallar el veredicto, como
  cualquier componente omitido. La línea de comandos sin `--tsr` puede así
  informar `valid` sin haber comprobado la revocación; las versiones selladas
  del servidor siempre incluyen sello.
