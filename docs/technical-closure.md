# Cierre técnico del TT

## Frontera de trabajo

La continuación autorizada cubre los pendientes técnicos comprometidos en los
requisitos y casos de uso del manuscrito. Se conserva el producto existente y
se termina una aceptación observable antes de abrir la siguiente. Una frase
antigua de «pendiente» no basta: se contrasta con main, código y evidencia.

Este inventario complementa, sin ampliar su catálogo, el
[cierre de los cuatro frentes procesales](four-front-closure.md). Este corte del
8 de octubre de 2026 describe el estado que entrega la integración de PR90
(cautelares completas), sobre PR91 (diagnóstico MFA, squash `46dc1c1`). Los gates
y la confirmación de main se comprueban por revisión exacta; los respaldos no
son integraciones ni despliegues.

## Lista de cierre

| Pendiente confirmado | Compromiso y límite | Criterio observable de término |
| --- | --- | --- |
| Corpus de cálculo | Ocho supuestos de investigación complementaria y recursos, sin los términos que sólo aparecen en teoría. | Cada supuesto tiene fuente primaria, ámbito, entradas y resultado independiente; fecha operativa explicada o falta de datos identificada; recorrido existente sin duplicados. |
| Invitaciones y enrolamiento | RF-03 y aceptación de invitación de [análisis y diseño](../latex/chapters/03-analisis-diseno.tex). Se conserva alta directa; no se rediseña identidad. | Invitación temporal aceptada con el rol autorizado, vencimiento rechazado, finalización de MFA y recuperación de respuesta perdida sin duplicar cuentas. Entrega de correo comprobada con transporte de prueba; activación operativa separada. |
| Firma documental individual | CU-13 y CU-14 del mismo [análisis](../latex/chapters/03-analisis-diseno.tex). El firmante global y el acceso Owner por certificado no completan la identidad del Litigante firmante. | Confirmación explícita del documento exacto; firma ligada al certificado del usuario autorizado; rechazo de certificado expirado/revocado; evidencia verificable de identidad, contenido y sello. Sin exigir PSC externo. |
| Informe comprometido restante | RF-19 y CU-16, conservando los informes de estado/carga ya integrados. | Informe de actividad registrada con tres cantidades por Litigante y periodo: documentos cargados, actuaciones procesales registradas y plazos marcados como atendidos. Atribución a la cuenta autora y fecha de la operación, sin cambiarla por reasignaciones posteriores. PDF/CSV coincidentes, progreso y estimación cuando aplica, aviso de terminación. Correo público sujeto a la decisión operativa pendiente. |
| Registro de accesos y datos de auditoría | RF-02, CU-04, CU-17 y RF-20, conservando cadena y registros históricos. | Accesos concedidos/denegados cubiertos por una matriz de operaciones; identidad estable y origen de red registrados para eventos nuevos sin atribuirlos retrospectivamente; confirmación durable medida contra el límite de 500 ms en un entorno declarado. |
| Conciliación y aceptación final | Documentación del comportamiento reproducido y gates de integración. | Contratos, operación y manuscrito coinciden; se reutiliza evidencia válida y se ejecuta una regresión completa al cierre de cada entrega funcional, gates para HEAD exacto, squash y main confirmado. |

El informe de desempeño queda delimitado a las tres cantidades
de actividad registrada anteriores. Sólo cuentan escrituras confirmadas: lecturas,
preparaciones y reintentos de la misma operación no agregan actividad. No se
añaden puntuaciones, comparaciones de éxito jurídico ni métricas distintas.
Los informes de estado y carga existentes se conservan.

El informe de actividad está implementado localmente, con aceptación focal y
recorridos reales. Su [decisión de diseño](adr/0073-author-attributed-activity-reports.md)
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
La integración exige los gates de la revisión publicada. Los avisos internos
y la duración observada no resuelven el correo ni la estimación validada de
CU-16; esos pendientes se conservan explícitos y no se atribuyen a los contadores.
La fila jurídica conserva la necesidad de identificar entidad, fuero y canal de
recepción. Los datos faltantes se presentan como decisiones concretas, no como
permiso para investigar o implementar indefinidamente.

## Trabajo entregado que no se reabre

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
uniforme ni la medición durable de 500 ms; esa fila permanece pendiente. Su
integración tampoco acredita activación o retención del journal en VPS3.

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

La cuenta/correo Owner, remitente y activación de correo, duración de inactividad
y activación en VPS3 siguen pendientes humanos. No se cambian por el avance de
código. Una evaluación con participantes reales también requiere personas y
resultados reales; las pruebas automáticas no la sustituyen.

El acceso documental Client requiere resolver su política explícita. No se abre
por analogía con los roles internos. Los planes comerciales se concilian con el
alcance de una instancia por despacho antes de cualquier desarrollo. No se añade
multitenencia, facturación, PSC externo, anclaje externo ni programación operativa
de CRL como consecuencia de una propuesta técnica. Las siete familias de
audiencias y catorce clases de medidas permanecen cerradas.

Una aceptación terminada no se reabre para una mejora opcional. Un defecto nuevo
se agrega sólo con comportamiento reproducible, requisito afectado y criterio
concreto de reparación. El avance se informa por estas aceptaciones, no por
cantidad de código, commits o comprobaciones.
