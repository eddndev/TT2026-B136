# Cierre técnico del TT

## Frontera de trabajo

La continuación autorizada cubre los pendientes técnicos comprometidos en los
requisitos y casos de uso del manuscrito. Se conserva el producto existente y
se termina una aceptación observable antes de abrir la siguiente. Una frase
antigua de «pendiente» no basta: se contrasta con main, código y evidencia.

Este inventario complementa, sin ampliar su catálogo, el
[cierre de los cuatro frentes procesales](four-front-closure.md). La base
confirmada en main y producción es `3794181`, versión `v0.1.5`, con PR98 y PR99
integradas. La auditoría de accesos y la reparación del respaldo Redis están
resueltas e instaladas. Se conservan las entregas anteriores de cautelares,
informe de actividad, firma individual, invitaciones y estimación/correo de
informes. Resend y recuperación pública están activos con los valores humanos
registrados; no se reabren esos pendientes operativos.

El [registro de verificación](verification-report.md) conserva los cortes de
aceptación, integración y activación. Los gates se comprueban por revisión
exacta; los respaldos no son integraciones ni despliegues. El corpus jurídico
descrito abajo tiene cierre documental, implementación local y aceptación final
en curso; todavía no forma parte de esa base productiva.

## Lista de cierre

| Entrega y estado | Compromiso y límite | Criterio observable de término |
| --- | --- | --- |
| Corpus de cálculo | Ocho supuestos documentados de investigación complementaria y recursos para el caso controlado penal local de CDMX. Las fichas fijan fuentes, condiciones, calendario, canales y esperados. `AtAnchor` y cierre civil semanal están implementados localmente, sin integrar; HTTP/Qadra, restauración y regresión API aprobados localmente. | Cada supuesto contrasta su resultado independiente con el recorrido HTTP/Qadra existente, explicación y origen exacto, sin duplicados; datos o condiciones no cubiertos bloquean el vencimiento. Falta conciliar la evidencia final, manuscrito y gates de esta entrega. |
| Invitaciones y enrolamiento | RF-03 y aceptación de invitación de [análisis y diseño](../latex/chapters/03-analisis-diseno.tex). Se conserva alta directa; no se rediseña identidad ni se limita el número de usuarios del despacho. Integrada por PR96 en `0db0a9f9`, con PostgreSQL, Redis, MFA, restauracion y los tres gates aprobados; desplegada en v0.1.4 y correo publico activado con Resend. | Invitación temporal aceptada con el rol autorizado, vencimiento rechazado, finalización de MFA y recuperación de respuesta perdida sin duplicar cuentas. Entrega de correo comprobada con transporte de prueba; activación operativa separada. |
| Firma documental individual | CU-13 y CU-14 del mismo [análisis](../latex/chapters/03-analisis-diseno.tex). Integrada por PR95 y desplegada en `v0.1.3`; conserva separados el firmante global histórico y el acceso Owner por certificado. | Los siete criterios de firma individual siguientes: identidad vinculada, confirmación exacta, clave externa, autorización y confianza vigentes, firma y sello, persistencia recuperable y verificación por componentes. Sin exigir PSC externo. Cierre técnico aceptado; la demostración personal requiere un Litigante real con clave y CSR externos. |
| Estimación y correo de informes | CU-16 integrado por PR97, desplegado en v0.1.4 y correo Resend activado: precision 26/28 (92.857%), correo capturado/HTTP/restauracion y Qadra real aceptados. Conserva los informes de estado/carga y el catalogo finito de actividad ya integrados. | Prediccion inicial incluida cola; correo cuando sea estrictamente mayor de 2 segundos; al menos 90% de validacion independiente dentro de max(2 segundos, 30% real), con cobertura declarada. Aviso comprobado con transporte de prueba; transporte Resend operativo confirmado por recepción humana. Los avisos internos y la duración observada no sustituyen estos criterios. |
| Registro de accesos y datos de auditoría | RF-02, CU-04, CU-17 y RF-20 aceptados, integrados y desplegados en la base `v0.1.5`, con contexto, cadena histórica, consulta Owner y restauración. Véanse [matriz](access-audit-matrix.md) y [verificación](verification-report.md). | Cierre confirmado: matriz de operaciones, identidad estable y origen de eventos nuevos, preservación histórica y medición durable frente a 500 ms en el entorno declarado. No se repite esta aceptación por cerrar el corpus. |
| Conciliación y aceptación final | Documentación del comportamiento reproducido y gates de integración. | Contratos, operación y manuscrito coinciden; se reutiliza evidencia válida y se ejecuta una regresión completa al cierre de cada entrega funcional, gates para HEAD exacto, squash y main confirmado. |

## Corpus jurídico delimitado

Las fichas de [investigación y prórroga](legal-corpus-investigation.md),
[seis recursos](legal-corpus-resources.md) y
[calendario y canales](legal-corpus-calendar.md) completan documentalmente los
ocho supuestos de la lista aprobada. El ámbito ya está fijado: órgano penal
local de CDMX, con calendario controlado de 2026 y modalidades de notificación
expresas. No queda una solicitud genérica de territorio ni se exige un revisor
externo o un expediente publicado idéntico para aceptar estos ejemplos.

Investigación y prórroga usan una orden ficticia que declara meses, método de
fecha a fecha, inicio y cierre a las 18:00 -06:00. Esa hora procede de la orden
del supuesto, no del CNPP. La concesión y el acumulado de la prórroga se califican
expresamente; el motor no los infiere de una solicitud ni suma historia ausente.
La oportunidad oral usa el fin exacto de la audiencia mediante `AtAnchor`.
Los cinco supuestos por días incluyen la fecha declarada de efectos y aplican
el calendario y cierre semanal de la ventanilla ordinaria UGJ documentados.

El soporte de `AtAnchor` y del cierre semanal está implementado localmente. La
aceptación con perfiles publicados, fuentes ficticias identificadas,
recibos, Agenda, alertas y restauración aprobó, junto con Qadra real y la
regresión API completa. Este estado no acredita todavía integración ni despliegue. Las condiciones
no cubiertas, desconocidas o rechazadas impiden un vencimiento operativo; no
abren otra investigación ni amplían el catálogo aprobado.

## Criterios de firma individual

1. El certificado público se vincula a la cuenta del Litigante mediante prueba
   de posesión externa, con registro, consulta y retiro auditados. No se deduce
   identidad de un correo, del subject del certificado ni de un participante.
2. El usuario ve y confirma el PDF, expediente, documento, versión, huella y
   certificado exactos. Se rechazan firmas de otra cuenta, versión o propósito.
3. La clave privada permanece bajo custodia del firmante fuera del servidor.
   El flujo recibe certificado y firma separados, nunca `.key`, contraseñas de
   clave ni PKCS#12; conserva la decisión de custodia del análisis de CU-13.
4. Sesión, rol, acceso, expediente activo, vinculación y confianza se comprueban
   al confirmar la operación. Certificado expirado/revocado, retiro, CRL no
   vigente o pérdida de autorización impiden completar la firma.
5. La firma valida contra el certificado y contenido exactos; el sello RFC3161
   valida contra el digest y la autoridad configurados. Un timeout o token
   inválido no deja una firma completa ni provoca cambio silencioso de TSA.
6. Versión, identidad, firma, certificado, confianza capturada y sello se
   conservan con auditoría atómica y evidencia exportable. La recuperación de
   una respuesta incierta y los reintentos no duplican firmas.
7. La verificación autorizada distingue integridad, firma, certificado y sello,
   así como validez histórica y estado actual. Un byte alterado invalida su
   comprobación; se declara qué acredita el tiempo sellado y que la TSA local
   demuestra el flujo técnico, sin atribuirle una constancia de PSC autorizado.

El cierre exige los siete criterios. Una vinculación de certificado entregada
por separado es un prerrequisito, no el cierre de CU-13 y CU-14.

Al 10 de octubre de 2026 existe implementación local de emisión desde CSR,
vinculación propia, firma doble del PDF y su consentimiento, persistencia
transaccional, transporte HTTP y cliente Qadra. La
[decisión de diseño](adr/0074-individual-document-signatures.md), el
[contrato HTTP](http-individual-signing.md) y la
[guía del titular](individual-signing-guide.md) delimitan esa implementación.
La clave individual no entra al servidor. El consentimiento identifica cuenta y
contadores, despliegue, operación, vínculo, confianza y versión, e incluye la
huella de la firma del PDF para impedir su reutilización en otro contexto.

Las migraciones `0044_` y `0045_` preservan vínculo, retiro, recibo y evidencia
con la auditoría común. La verificación conserva pruebas históricas bajo la
confianza capturada y presenta aparte la situación actual. El sello cubre el
digest del contenido y no prueba la existencia de la firma en su fecha. Los
sellos anteriores siguen identificados como sellos del servidor.

La aceptación nativa, HTTP con restauración y navegador real aprobó los siete
criterios funcionales. La [verificación](verification-report.md) conserva las
pruebas negativas, el rechazo atómico, la concurrencia, los recibos originales,
la validación independiente del ZIP y la separación entre revocación posterior
y validez histórica. Los controles de integración requieren evidencia para la
revisión exacta; una aceptación local no acredita por sí sola publicación ni
despliegue. La firma individual se activó en `v0.1.3`, sobre
`784a0cfb064058a358e71316da595d4768de5b96`, y se conserva en la base operativa
actual `v0.1.5`.


## Estado del informe de actividad

El informe de desempeño queda delimitado a las tres cantidades
de actividad registrada por Litigante y periodo: documentos cargados, actuaciones
procesales registradas y plazos marcados como atendidos. Sólo cuentan escrituras
confirmadas: lecturas, preparaciones y reintentos de la misma operación no
agregan actividad. No se añaden puntuaciones, comparaciones de éxito jurídico ni
métricas distintas.
Los informes de estado y carga existentes se conservan.

El informe de actividad está integrado mediante PR92 y su catálogo finito de
tres contadores queda cerrado, con aceptación focal y recorridos reales.
Su [decisión de diseño](adr/0073-author-attributed-activity-reports.md)
conserva exactamente las tres cantidades y declara la cobertura documental
histórica incompleta. El catálogo definitivo comprende registros originales de
resoluciones/notificaciones, cinco actos de recursos, sesiones/resultados y
decisiones judiciales cautelares. Cada original cuenta una vez; sus acuerdos o
medidas contenidos no aportan actuaciones adicionales.

La aceptación conserva autoría tras reasignaciones, captura y archivos después
de restaurar, avisos tras renovar sesión y PDF/CSV coincidentes. El recorrido
positivo del catálogo completo obtuvo 1 documento, 9 actuaciones y 1 plazo
atendido. HTTP/restauración y manuscrito separado también aprobaron. La
[verificación](verification-report.md) distingue estos cortes y sus límites.
El informe ya está desplegado en `v0.1.2`; falta generar y descargar PDF/CSV
en la sesión real de producción, sin sustituir esa comprobación por el ensayo.
La estimación y el correo de CU-16 se cerraron por separado en PR97; no se
atribuyen a los contadores ni se sustituyen por duración observada.
La fila jurídica ya identifica entidad, fuero y canales de recepción en las
fichas del corpus. Las condiciones de otro supuesto quedan fuera de esa
aceptación; no autorizan investigar o implementar indefinidamente.

## Trabajo entregado que no se reabre

- Informe de actividad de PR92: tres contadores por autor y periodo, con el
  catálogo finito completo, sin inflación por correcciones, reintentos o medidas
  contenidas. Estimación y correo de CU-16 quedaron cerrados por PR97.
- Flujo cautelar completo de PR90: HTTP, Qadra, Agenda, alertas, decisiones y
  rectificaciones con historia y recuperación; navegador real, reinicios,
  restauración y manuscrito separado aceptados. Se conservan siete familias
  de audiencias y catorce clases de medidas; no hay otra ampliación del catálogo.
- Diagnóstico operativo MFA de PR91: intentos TOTP y recuperación con hora,
  `X-Request-Id`, resultado, motivo saneado y UUID de cuenta cuando está disponible.
  Conserva el rechazo público genérico y excluye secretos. Véase
  [el contrato de logs](mfa-access-logs.md).

Los logs MFA son operativos y están separados de la auditoría transaccional.
No completan la matriz general de accesos, el origen de red, la identidad estable
uniforme ni la medición durable de 500 ms. La extensión de acceso y contexto
cierra esa fila con su evidencia independiente, integrada y activada en
`v0.1.5`; no se atribuye a los logs MFA.

- Audiencias propias de recursos, creación explícita de resultado/plazo derivado,
  Agenda, alertas y las familias ordinarias aceptadas de recuperación de editores.
- Informes de estado/carga y consulta de actividad: PR49 y PR50, con sus
  confirmaciones conservadas. Sus contratos aún tenían frases antiguas de
  integración pendiente.
- Recuperación de contraseña y primer factor Owner por certificado, con la
  activación operativa separada de su implementación.
- Instalación recuperable de generaciones de controladores: PR86, squash
  `90b27f20`, presente en main. La frase de integración pendiente en el resumen
  de producto era antigua; no requiere otra implementación ni otra PR.

## Fronteras humanas y trabajo excluido

La cuenta/correo Owner y duración de inactividad siguen pendientes humanos.
El remitente y la activación de correo ya quedaron resueltos con Resend; los
valores aprobados de recuperación pública están activos. La continuación autoriza despliegues
con la configuración vigente preservada; no autoriza inferir esos valores.
La discrepancia del directorio Redis del recibo automático de compatibilidad
quedó corregida, integrada e instalada en la base `v0.1.5`; el
[registro de verificación](verification-report.md) conserva su evidencia. No
constituye un pendiente de esta continuación.
Una evaluación con participantes reales también
requiere personas y resultados reales; las pruebas automáticas no la sustituyen.

El acceso documental Client requiere resolver su política explícita. No se abre
por analogía con los roles internos. La instancia atiende un solo despacho sin límite de usuarios; no se aplica
un cupo por plan comercial. No se añade
multitenencia, facturación, PSC externo, anclaje externo ni programación operativa
de CRL como consecuencia de una propuesta técnica. Las siete familias de
audiencias y catorce clases de medidas permanecen cerradas.

Una aceptación terminada no se reabre para una mejora opcional. Un defecto nuevo
se agrega sólo con comportamiento reproducible, requisito afectado y criterio
concreto de reparación. El avance se informa por estas aceptaciones, no por
cantidad de código, commits o comprobaciones.
