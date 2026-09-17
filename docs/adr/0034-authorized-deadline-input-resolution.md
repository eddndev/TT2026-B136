# ADR 0034: Resolución autorizada de insumos temporales

## Status

Accepted.

## Context

La extracción y aritmética necesitan material cuya identidad, revisión y evidencia
estén verificadas. Sustituir fuentes históricas por sus cabezas impediría reproducir
el cálculo. Las operaciones públicas existentes abren transacciones auditadas;
llamarlas dentro de otro adaptador podría esperar su propio bloqueo y duplicar
consultas y eventos. Sus cargadores internos permiten reunir los datos bajo una
misma frontera de autorización y auditoría.

## Decision

Introducir servicio, puerto y comprobador en `application::deadline_inputs`, con
[contrato propio](../deadline-inputs.md). Recibir referencias exactas y operandos
explícitos; resolver material mediante PostgreSQL y verificarlo antes de calcular.

Conservar fuente y calendario elegidos junto a sus cabezas. Validar recibos,
identidad y revisiones; utilizar exclusivamente valores seleccionados para la
reproducción. El retiro posterior no sustituye automáticamente la evidencia.

Autenticar antes del puerto y antes de entregar. El adaptador revalida usuario,
rol y acceso al expediente en su transacción y confirma una auditoría de lectura.
La administración cerrada conserva lectura; el contexto sin revisiones no recibe
una revisión inventada. Separar errores de integridad de bloqueos válidos.

No interpretar estados o textos como aplicabilidad jurídica. Esta decisión no
altera los cánones de fuentes ni introduce evaluaciones, tablas o rutas HTTP.

## Consequences

La preparación reproduce operandos verificados sin duplicar la persistencia de
hechos. Cada consulta lee también las cabezas y registra un evento; los permisos
y recibos deben validarse incluso cuando no haya una fuente conocida.

Las pruebas cubren discrepancias, cabezas cambiadas, permisos, sesiones revocadas,
expedientes cerrados y ausencia de eventos en fallos internos del puerto. La
segunda autenticación puede impedir la entrega después de confirmar una lectura;
ese rechazo no revierte su auditoría. Redis no participa en la transacción SQL.

La preparación no reserva fuentes para una escritura futura. Perfiles jurídicos,
activación, evaluaciones persistentes, atención, reevaluación y entrega de alertas
requieren completar sus contratos y flujos conservando el alcance aprobado.
