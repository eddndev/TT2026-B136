# Documentacion

Indice por tema de los contratos, manuales y registros existentes. Los informes
conservan la fecha y el alcance de sus comprobaciones; su presencia no acredita
una ejecucion nueva ni convierte una capacidad prevista en implementada.

## Estado y uso

- [Estado tecnico y limites de la entrega](technical-closure.md).
- [Aceptacion productiva](production-acceptance.md).
- [Alcance y estado del producto](product-completion.md).
- [Manual de uso de Qadra](user-guide.md).
- [Contrato HTTP general](http-api.md).
- [Workspace de interfaces](../web/README.md).
- [Interfaz del despacho](../web/apps/qadra/README.md).
- [Alcance del portal de la CA](ca/README.md).

## Identidad, certificados y firma

- [Miembros y permisos](members-api.md), [invitaciones](member-invitations-api.md)
  y [alta y recuperacion](member-enrollment-recovery.md).
- [Recuperacion de contrasena: interfaz publica](password-reset-public.md)
  y [contrato interno](password-reset-internal.md).
- [Sesion y recuperacion del editor](session-editor-recovery.md)
  y [diagnostico de acceso con MFA](mfa-access-logs.md).
- [Vinculo de certificado Owner: contrato HTTP](http-owner-certificates.md)
  y [recorrido de interfaz](owner-certificate-interface.md).
- [Acceso Owner: contrato interno](owner-certificate-authentication.md),
  [HTTP](owner-certificate-login-http.md),
  [interfaz](owner-certificate-login-interface.md)
  y [operacion](owner-certificate-login-operations.md).
- [Firma individual: guia](individual-signing-guide.md)
  y [contrato HTTP](http-individual-signing.md).
- [Scripts y configuracion de la CA existente](../pki/README.md).

## Expedientes, participantes y documentos

- [Administracion del expediente](case-administration-api.md)
  y [etapas](case-stages-api.md).
- [Participantes tipificados](typed-participants-api.md).
- [Admision de documentos](document-upload-admission-api.md)
  y [entrega del contenido original](document-content-api.md).
- [Operacion de formatos documentales](document-format-operations.md)
  e [instalacion del decodificador multimedia](media-decoder-setup.md).
- [Informes del expediente](case-reports-api.md),
  [bitacora](audit-events-api.md) y [tablero](dashboard-api.md).

## Audiencias, agenda y recursos

- [Audiencias](hearings-api.md), [sesiones y resultados](hearing-results-api.md)
  y [plazos derivados de resultados](hearing-derived-deadlines.md).
- [Audiencias cautelares: alcance](precautionary-hearings-scope.md)
  y [contrato HTTP](precautionary-hearings-api.md).
- [Registros de medidas](measure-records-api.md)
  y [decisiones sobre medidas](measure-decisions-api.md).
- [Recursos procesales: alcance](procedural-resources-scope.md)
  y [contrato HTTP](procedural-resources-api.md).
- [Audiencias de recursos](resource-hearings.md)
  y [sus alertas](resource-hearing-alerts.md).
- [Actividades de recursos](resource-activities-api.md),
  [asociaciones de actividades](activity-resource-links-api.md)
  y [plazos de recursos](resource-deadlines-api.md).
- [Agenda](agenda-api.md) y [alertas](alerts-api.md).

## Hechos, calendarios y plazos

- [Hechos procesales](procedural-facts.md),
  [canonizacion](procedural-facts-canonical.md),
  [recibos](procedural-facts-receipts.md),
  [persistencia](procedural-facts-persistence.md),
  [aplicacion](procedural-facts-application.md)
  y [contrato HTTP](procedural-facts-api.md).
- [Tiempo procesal](procedural-time.md)
  y [calendarios judiciales](judicial-calendars-api.md).
- [Ciclo de vida de plazos](deadline-lifecycle.md),
  [aritmetica](deadline-arithmetic.md),
  [conteo de dias](deadline-day-counting.md),
  [insumos](deadline-inputs.md) y [desencadenantes](deadline-triggers.md).
- [Investigacion de reglas](deadline-rule-research.md),
  [perfiles](deadline-profiles-api.md)
  y [limites JSON de perfiles](deadline-profile-json-budget.md).
- [Registros de plazo](deadline-records.md), [API de plazos](deadlines-api.md),
  [API de seguimiento](deadline-tracking-api.md)
  y [recibos de seguimiento](deadline-tracking-receipts.md).
- [Despacho de reevaluaciones](deadline-dispatch.md)
  y [consumo durable](deadline-worker.md).
- [Aceptacion de los cuatro frentes](four-front-closure.md)
  y corpus de [investigacion](legal-corpus-investigation.md),
  [calendarios](legal-corpus-calendar.md) y [recursos](legal-corpus-resources.md).

## Despliegue y recuperacion

- [Despliegue](deployment.md), [base de datos](database-operations.md)
  y [mantenimiento recuperable de CRL](deployment-crl.md).
- [Respaldos](deployment-backups.md),
  [manifiesto de captura](backup-capture-manifest.md)
  y [captura para restauracion](deployment-restore-capture.md).
- [Comandos de restauracion](database-restore-commands.md),
  [compatibilidad](deployment-restore-compatibility.md),
  [exclusion de restauracion](deployment-restore-fence.md),
  [detencion coordinada](deployment-restore-quiesce.md)
  y [observacion de procesos](restore-process-observation.md).
- [Plazo de disponibilidad](deployment-readiness-deadline.md).
- Controlador: [instalacion](deployment-controller-installation.md),
  [lanzador](deployment-controller-launcher.md),
  [arranque inicial](deployment-controller-bootstrap.md),
  [entrada](deployment-controller-entry-gate.md),
  [aprobacion](deployment-controller-approval.md)
  y [publicacion](deployment-controller-publication.md).

## Verificacion y decisiones

- [Informe de verificacion de software](verification-report.md)
  e [informe del documento academico](academic-report-verification.md).
- [Matriz de autorizacion y auditoria](access-audit-matrix.md)
  y [medicion de auditoria de acceso](access-audit-measurement.md).
- [Preparacion de pruebas de navegador](browser-preparation-checks.md)
  y [operacion de runners de CI](ci-runner-operations.md).
- [Protocolo y materiales de usabilidad](usability/README.md).
- [Decisiones de arquitectura](adr/).
- Antecedentes: [revision del backend](backend-review.md),
  [transcripcion de demostracion](demo-transcript.md)
  y [programacion historica de entrega](next-goal.md).
- [Fuentes y compilacion del documento academico](../latex/README.md).
