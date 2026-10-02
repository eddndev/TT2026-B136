# Registro y esquema de evidencia

Los cuatro CSV son plantillas UTF-8 con encabezado y **cero registros**. Cópialos
a una ubicación privada antes de completar datos. No hay fórmulas, calificaciones
precargadas ni ejemplos que puedan confundirse con observaciones. El separador es
coma; un campo con coma, comillas o salto de línea debe seguir el entrecomillado CSV.

Los encabezados ASCII facilitan intercambio. Las notas pueden escribirse en español.
Al abrir una copia en una hoja de cálculo, importa como texto los identificadores,
fechas y comentarios; no interpretes comentarios como fórmulas. Conserva el registro
original y documenta cualquier limpieza en una copia de análisis.

## Convenciones compartidas

- Identificadores de sesión y participante estables, sin nombre ni correo real.
- Fechas ISO 8601 con desfase explícito, por ejemplo la forma
  `AAAA-MM-DDThh:mm:ss-06:00`; no se presupone el desfase de la computadora.
- Números decimales con punto; duraciones en segundos, nunca mezcladas con minutos.
- Campo vacío significa no registrado/no aplicable y requiere motivo cuando era
  obligatorio. Cero significa cero observado; no se usa para sustituir ausencias.
- Listas de identificadores o códigos dentro de una celda se separan con `|`.
- Los únicos registros de producto son ficticios. Las referencias a evidencias
  apuntan al archivo privado autorizado; no incluyen contraseña, token o código.

## sessions.csv: una fila por sesión

| Campos | Definición |
| --- | --- |
| `session_id`, `participant_id` | Identificadores; un participante puede tener varias sesiones. |
| `protocol_revision`, `product_revision` | Revisión exacta del material y del producto evaluado. No sustituir por «última». |
| `session_kind` | `ensayo_tecnico`, `piloto_humano` o `evaluacion_humana`. Un ensayo sin persona puede dejar `participant_id` vacío. |
| `account_role` | `owner`, `litigator` o `paralegal` de la cuenta sintética. |
| `work_profile` | Perfil laboral declarado, general y voluntario; diferente del rol simulado. |
| `experience_group` | `sin_experiencia`, `uso_ocasional`, `uso_habitual` o vacío con motivo. Describe experiencia con sistemas de expedientes, no competencia jurídica. |
| `viewport_width_px`, `viewport_height_px` | Dimensiones observadas del área de página en píxeles. |
| `device`, `input_mode`, `emulation` | Descripción general de dispositivo; `raton_teclado`, `tactil` o `mixto`; emulación `si`/`no`. |
| `zoom_percent`, `browser`, `os` | Zoom observado; navegador y sistema con versión cuando esté disponible. |
| `mfa_method` | `totp` o `recovery`; no registrar el valor secreto. |
| `enabled_optional_tasks` | Tareas adicionales habilitadas, separadas por `|`; vacío significa ninguna. |
| `consent_notes`, `consent_screenshots`, `consent_quotes` | `si`/`no` según la hoja de consentimiento. Sin consentimiento de notas no se registra evaluación humana. |
| `started_at`, `planned_minutes` | Inicio y duración organizativa prevista de la sesión. |
| `notes` | Condiciones relevantes y datos faltantes, sin identificadores personales. |

## attempts.csv: una fila por tarea planificada e intento

| Campos | Definición |
| --- | --- |
| `session_id`, `participant_id` | Coinciden con la sesión; no cambian dentro de un intento. |
| `task_id` | Código de [tasks.md](tasks.md); `T04-S` y `T04-V` son tareas distintas. |
| `attempt_no` | Entero desde 1. Una repetición por aprendizaje o corrección se conserva separada. |
| `planned_order`, `task_limit_seconds` | Orden y límite acordados antes de la sesión. |
| `fixture_reference` | Identificador del expediente/documento ficticio y estado inicial documentado; no contenido real. |
| `status` | Uno de los siete estados definidos en [protocol.md](protocol.md). Obligatorio al cerrar el registro. |
| `started_at`, `ended_at` | Inicio y fin reales. Vacíos si no se inició; un retiro posterior al inicio conserva lo autorizado. |
| `elapsed_seconds` | Cronómetro total entre inicio y fin. Incluye espera del producto. |
| `external_pause_seconds` | Interrupciones ajenas a la tarea documentadas; no incluye lentitud del sistema. |
| `active_seconds` | `elapsed_seconds` menos `external_pause_seconds`, si ambos están registrados. |
| `assistance_count`, `assistance_kinds` | Número de ayudas; clases `manual`, `pista`, `navegacion` o `intervencion`. Una repetición neutral de la consigna no cuenta. |
| `error_count` | Acciones o interpretaciones incorrectas observadas, enlazadas a observaciones. Una misma equivocación continua no se cuenta por cada clic. |
| `completion_evidence` | Hecho que demostró finalización, por ejemplo ficha guardada o descarga terminada; obligatorio al marcar completada. |
| `observation_ids` | Referencias a observaciones de esta tarea e intento. |
| `stop_reason`, `notes` | Motivo de no completar/no iniciar/interrumpir y aclaraciones. |

Una tarea completada sin ayuda exige `assistance_count=0`; completada con ayuda
exige una ayuda registrada. Una tarea no completada puede tener ayudas o ninguna.
No incluyas en `completion_evidence` el resultado esperado copiado de la tarjeta:
escribe lo que efectivamente se comprobó.

## observations.csv: evidencia descriptiva

| Campos | Definición |
| --- | --- |
| `observation_id`, `session_id`, `participant_id`, `task_id`, `attempt_no` | Identidad de la observación y vínculo a un intento existente. |
| `kind` | `error`, `ayuda`, `duda`, `logro`, `entorno` o `comentario`. |
| `at_elapsed_seconds` | Momento del cronómetro total en que ocurrió, si se registró. |
| `observed_fact` | Acción, mensaje o resultado observado, sin atribuir una causa técnica no comprobada. |
| `participant_comment` | Comentario textual sólo si fue autorizado; en otro caso dejar vacío y resumir el hecho sin citas en `observed_fact`. |
| `interpretation` | Hipótesis del equipo, separada expresamente del hecho. Puede quedar vacía. |
| `impact` | `bloquea`, `requiere_ayuda`, `dificulta`, `sin_impacto_observado` o `no_determinado`. |
| `evidence_reference` | Captura o nota privada autorizada; vacío cuando no existe, nunca una captura inventada. |
| `followup_status` | `por_revisar`, `reproducido`, `corregido_pendiente_comprobar` o `comprobado`; cada cambio requiere evidencia. |
| `notes` | Aclaraciones o relación con una comprobación posterior. |

## summary.csv: agregados derivados, nunca entrada de resultados individuales

Cada fila agrupa la misma revisión de producto/protocolo, tipo de sesión, rol,
tarea, condición de viewport e intento inicial o repetido. No se mezclan condiciones
sólo para aumentar el número de observaciones.

| Campos | Definición |
| --- | --- |
| `product_revision`, `protocol_revision`, `session_kind`, `account_role`, `task_id` | Claves de agrupación tomadas de las sesiones y tareas. |
| `viewport_group`, `attempt_group` | Condición descrita, como ancho y tipo de entrada; `inicial` o `repeticion`. No mezclar 768 px con móvil o escritorio. |
| `n_planned`, `n_started` | Intentos planificados del grupo y aquellos que realmente comenzaron. |
| `n_completed_without_help`, `n_completed_with_help`, `n_not_completed` | Conteos por resultado, derivados de `status`. |
| `n_not_started`, `n_technical_interruptions`, `n_withdrawn`, `n_not_applicable`, `n_missing_status` | Resto de estados y ausencias; informar explícitamente, no convertirlos en éxito o fracaso. |
| `n_timed_completed`, `n_timed_not_completed` | Cantidad de duraciones válidas que sustentan cada resumen temporal. |
| `median_active_completed_seconds`, `min_active_completed_seconds`, `max_active_completed_seconds` | Mediana y rango de tareas completadas con o sin ayuda; indicar esa combinación. |
| `median_active_not_completed_seconds`, `min_active_not_completed_seconds`, `max_active_not_completed_seconds` | Mediana y rango de intentos no completados; no incluir tareas nunca iniciadas. |
| `total_errors`, `total_assistance` | Sumas de registros observados disponibles; señalar faltantes en limitaciones. |
| `source_session_ids`, `observation_ids` | Referencias que permiten revisar de dónde salió el agregado. |
| `limitations` | Perfiles/condiciones faltantes, exclusiones y otras restricciones. |

La suma de los ocho conteos de estado debe ser igual a `n_planned`. `n_started`
se calcula a partir de intentos con inicio real, no como resta simplificada de
estados, porque un retiro puede ocurrir antes o después de comenzar.

Para la mediana, ordena las duraciones válidas; toma la central o el promedio de
las dos centrales cuando sean pares. Sin duraciones válidas, deja mediana/rango
vacíos y el tamaño correspondiente en cero. No infieras satisfacción de tiempos.

Si se solicita eliminación, aplica el consentimiento antes del análisis. No
conserves vínculos personales ni comentarios contra esa decisión para cuadrar
las cifras. Explica las exclusiones de forma no identificable. Nunca publiques
las hojas privadas completas como anexo del repositorio.
