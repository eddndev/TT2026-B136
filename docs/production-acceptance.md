# Aceptación productiva y conciliación documental

Corte: 12 de octubre de 2026. Este registro cubre los puntos 1 y 4 autorizados:
informes y recuperación en producción, y conciliación de manuales e inventarios.
No añade funcionalidades. La base productiva comprobada es `v0.1.6`, commit
`671d356c428fd01d26a1fa80eb182bc1f526813e`.
El [checklist revisado](https://chatgpt.com/space/page_8292451649ac819190aa3cdd7a91c8e8)
conserva doce criterios finitos y su avance.

## Aceptación y procedencia de la evidencia

| Recorrido | Evidencia conservada | Estado de la comprobación productiva |
| --- | --- | --- |
| Recuperación por correo | El usuario informó el 12 de octubre que su pareja probó la recuperación con su cuenta el día anterior. Resend y los límites aprobados ya estaban activos. | Confirmada por reporte humano; no se repite el cambio. No se publican cuenta, enlace ni credenciales. |
| Controles de recuperación | Ensayo HTTP compuesto con PostgreSQL, Redis, Argon2id y MFA reales: 1/1 en 13.31 s. Consumo único, cambio auditado, credenciales anteriores invalidadas, ausencia de sesión automática y acceso posterior con MFA existente. Entrega capturada. | Se reutiliza la aceptación técnica. No se afirma que el reporte humano haya comprobado cada negativo, ni se le atribuye observación instrumental del agente. |
| Estado y carga | Captura autorizada y PDF/CSV aceptados técnicamente; PR49 integrada y conservada en la versión vigente. | PDF y CSV descargados y cotejados: tres expedientes ficticios, dos activos y uno cerrado, coincidentes con Qadra. Captura del 12 de octubre, periodo del 1 al 13 de octubre UTC (final excluido), alcance de oficina y sin filtro de litigante. |
| Actividad por litigante | PR92 aceptó autor original, periodo, tres contadores, restauración y PDF/CSV coincidentes. La cobertura documental histórica incompleta se declara. | PDF y CSV descargados y cotejados: cero documentos, actuaciones y plazos atendidos; `documents_complete=false` y advertencia visible. No hubo autores con actividad en esta captura; los casos positivos de autoría y tres contadores conservan la aceptación previa de PR92. |
| Estimación y correo del informe | PR97: 26/28 casos dentro de tolerancia (92.857%), con umbral inicial estrictamente mayor de 2 s. Transporte capturado; Resend operativo con recepción humana del correo de prueba genérico. | Ambos informes mostraron estimación no disponible, duración observada menor de un segundo y ningún aviso por correo registrado. La condición de correo no se ejercitó; se conserva su aceptación anterior sin generar carga artificial. |

El recibo privado `output/production-acceptance/acceptance-result.json` conserva
tipo, filtros, fecha de captura, identidad, tamaño y SHA-256 de los cuatro
archivos. Se cotejaron metadatos, filas y totales; ambos PDF de una página A4 se
inspeccionaron visualmente, sin recortes ni texto ilegible. Los archivos
productivos permanecen fuera de Git. El usuario descargó el primer par en Chrome
porque el navegador integrado no permitía descargas; el segundo par se obtuvo
desde su sesión ya abierta en Chrome. No se amplió ese permiso de navegador.

| Captura aceptada | Identidad y comprobación |
| --- | --- |
| Estado/carga | Informe `4252e574-637e-424c-abde-0797946c0138`; captura `f3e33b68ed5ca89ae992b71dc78a59f8da53cf079f7ea04a835017d0988553dd`; PDF 1173419 bytes, CSV 1870 bytes. |
| Actividad | Informe `ad874130-f445-425e-8a8c-2454d3cab38c`; captura `d8d786126b8c78d7643e3b9b3b49a93b89f5c0fc47d81ec3c9c934d199a92b2c`; PDF 1164148 bytes, CSV 555 bytes. |

La aceptación de descargas queda cerrada para la cuenta Administrador de demo y
el conjunto observado. No se crearon datos artificiales, roles o asignaciones
para producir cifras positivas, ni se acredita recepción de un aviso no emitido.

## Trazabilidad finita

| Compromiso | Comportamiento entregado | Contrato y aceptación |
| --- | --- | --- |
| HU-39, CU-16, RF-19, RNF-06 y RNF-07 | Estado/carga y actividad por autor; trabajo asíncrono, filtros, PDF/CSV, estimación inicial y aviso condicional. Actividad limita desempeño a tres cantidades, sin medir éxito jurídico. | [Análisis y diseño](../latex/chapters/03-analisis-diseno.tex), [contrato de informes](case-reports-api.md), [verificación](verification-report.md) y filas anteriores. |
| HU-08, CU-03, RF-01 | Solicitud neutra por correo, capacidad temporal de un uso, nueva contraseña y MFA existente; invalidación del acceso anterior. | [Análisis y diseño](../latex/chapters/03-analisis-diseno.tex), [transporte público](password-reset-public.md), [contrato interno](password-reset-internal.md) y reporte humano diferenciado. |
| Manuales y estado de entregas | Versión vigente, invitaciones, reingreso, firma individual, siete familias de audiencias, medidas, informes y contexto de auditoría. | [Manual de uso](user-guide.md), [cuatro frentes](four-front-closure.md), [inventario técnico](technical-closure.md), [estado del producto](product-completion.md). |

La recuperación mantiene 15 minutos de vigencia, un enlace pendiente por cuenta,
una solicitud por correo cada cinco minutos y cinco segundos de espera total al
correo. El restablecimiento no cambia roles ni asignaciones y no sustituye MFA.
La admisión de una solicitud y la aceptación del proveedor no son recibos de
entrega en bandeja.

## Revisión y límites

El checklist fue revisado contra UI, contratos y requisitos. La revisión
conservó la correspondencia entre captura y archivos, autoría original, alcance
por permisos y diferencia entre correo aceptado y recibido. Tras la confirmación
humana se retiró la repetición innecesaria de cambio de contraseña; los controles
negativos conservan su evidencia técnica anterior.

No se requieren otra regresión funcional completa, restauración, cambio de
controladores, despliegue o compilación del manuscrito para conciliar estos
textos. Los gates de publicación se aplican al cambio documental exacto.
La instalación de controladores no equivale a automatizar una restauración
integral: ésta conserva su procedimiento administrativo, barrera de acceso e
invalidaciones descritas en [operación](deployment-backups.md).

El PDF académico aceptado se conserva: `latex/main.pdf`, 419 páginas,
SHA-256 `31c83852dc6bb8ece48b11ebec0405b25cc40022919f0f92368684f475a1ff2d`.
Se detectó en `latex/chapters/03-informes.tex` una frase sobre activación pública
pendiente que debe conciliarse al cerrar el manuscrito. Se registra aquí para el
punto académico correspondiente; no se recompila ni se reemplaza el PDF ahora.

Permanecen separados: firma personal con el titular y su clave/CSR; cuenta Owner
operativa, inactividad, política documental Client y acceso Owner por certificado;
estudio con participantes reales; conclusiones y presentación.

## Firma bilateral en producción: 4 de noviembre de 2026

La firma bilateral de contratos de [ADR 0086](adr/0086-bilateral-contract-signing.md)
está instalada en producción desde `v0.1.16`, commit
`26eb7efbf4c512758f7987db222ab4ca59402cfa`, y su recorrido completo se ejecutó
allí con una cuenta Cliente controlada por el equipo. No es la aceptación de un
cliente real ni una evaluación con personas.

| Paso | Evidencia | Resultado |
| --- | --- | --- |
| Precondición de 0051 | Una cuenta Cliente heredada tenía una asignación de expediente; la migración se niega mientras exista. El Owner la retiró por `DELETE /cases/{case}/members/{user}` (`204`, auditado), sin SQL directo. | `client_memberships=0` antes de la ventana. |
| Ensayo | Respaldo del 30 de octubre restaurado en un clúster desechable del servidor: con la asignación la migración se niega sin cambiar la base y la versión anterior sigue arrancando; sin ella migra en 3 s, repetirla no cambia nada, la versión nueva valida la base y la anterior la rechaza. | Aprobado. |
| Ventana | Bajo `deploy.lock` y la generación aprobada del controlador: admisión, detención, respaldo `20261104T014518Z-47234d6c`, migraciones 0050 a 0052 con el rol administrador, `database check` con el rol de ejecución, marca de esquema `46d07ca0…57937a5`, `current` y `previous` y arranque. Sin `release.activate`. | 01:45:17 a 01:47:12 UTC, unos dos minutos sin servicio. |
| Alta de la Cliente | Invitación `client` del Owner enviada por Resend al buzón de la cuenta controlada; enlace recibido en ese buzón; preparación MFA y `finish`. | Cuenta `client` aceptada a las 02:20:27 UTC, sin pertenencia a expedientes. |
| Certificados | Claves RSA-3072 y CSR generados fuera de la CA; la CA interna emitió las series 1005 (Owner) y 1006 (Cliente) con propósito `document-signing`; cada parte vinculó el suyo con su prueba `DOCBIND1`. | Owner como único firmante elegible del despacho. |
| Firma | Contrato `275e98cb-6bfd-4ae9-b9d8-0b1d72e5da24` sobre el PDF `152bc743…f07e`; solicitud `0282ade9-cc91-4a93-b69f-f65fa8f3eba6` con referencia de hoja de huella. La Cliente leyó la versión exacta y firmó en la posición 1; el Owner contrafirmó en la posición 2. | Solicitud `completed`, contrato `firmado`, revisión 4. T1 02:46:34 ≤ Td 02:46:42 ≤ Tf 02:46:42 UTC. |
| Acuse parcial | `BilateralContractPartialV1` exportado por la Cliente tras firmar. | Validador: `overall: partial`, sin comprobaciones fallidas; OpenSSL `Verified OK` y cadena correcta. |
| Paquete final | `BilateralContractV1`, SHA-256 `c8b55911a46e5c26f7894f3debbfeebaf403e6a367e3a1ba9954bf6f83bea261`, idéntico exportado por el Owner y por la Cliente. | Validador nativo (`evidence-verify`, la misma biblioteca del validador del navegador): `overall: complete`. Cada orden de `INSTRUCCIONES.md`: cuatro firmas `Verified OK`, dos certificados con CRL correctos, raíz de los vínculos igual a `ca.pem`, contrafirma sobre la firma y el sello de la Cliente, tres sellos `Verification: OK`, 23 entradas del manifiesto coincidentes. |

El paquete final no se cargó en el validador del navegador de producción: esa
vista requiere una sesión de Qadra y esta comprobación no introdujo
contraseñas en un navegador. La validación nativa usa el mismo código que la
vista. Los avisos por correo de contratos siguen desactivados
(`TT_CONTRACT_EMAIL_ENABLED` no está definido en producción); ninguno se
comprobó. Las claves, contraseñas, factores, recibos y paquetes permanecen
fuera de Git, en el directorio privado de revisión.

La excepción de acceso del Cliente a sus propias solicitudes no abre la
política documental general del Client. Una confirmación de uso o prueba
automática no completa el estudio con participantes ni produce sus resultados.
