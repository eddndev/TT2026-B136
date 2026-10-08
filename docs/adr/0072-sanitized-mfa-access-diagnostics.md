# 0072: Diagnósticos internos saneados para MFA

## Context

La respuesta pública `mfa_rejected` protege los detalles del acceso, pero no
permite distinguir operativamente un TOTP inválido de un reclamo de uso ya
ocupado. El consumo atómico del desafío elimina información: ausencia no
establece si venció, se consumió o nunca existió. Las operaciones pueden fallar
después de verificar el código, por cambios de cuenta o errores de auditoría.

## Decision

La aplicación devuelve una observación tipada con UUID opcional de cuenta,
motivo cerrado y resultado original. Los métodos anteriores conservan su
contrato y delegan a esta operación. La capa web registra solamente metadatos
permitidos; no formatea el resultado, los errores ni la petición. El resultado
contiene credenciales y no implementa Debug.

Cada petición MFA recibe un UUID generado en el servidor y devuelto como
`X-Request-Id`. La operación bloqueante emite el resultado con el contexto de
tracing capturado; el middleware cubre rechazos HTTP previos sin duplicarlo.
Los motivos que las interfaces existentes no distinguen se agrupan explícitamente.
No se añaden consultas, tombstones ni cambios a las decisiones de autorización.

## Status

Aceptada.

## Consequences

El operador correlaciona respuestas con logs sin revelar motivos al cliente.
Los eventos operativos no sustituyen la auditoría transaccional ni garantizan
retención ante fallos del proceso o del destino de logs. UUID de cuenta e historial
de acceso requieren acceso restringido al journal. Los adaptadores del puerto sin
diagnóstico detallado declaran `rejection_unspecified`. El catálogo y los comandos
de consulta están en `docs/mfa-access-logs.md`.
