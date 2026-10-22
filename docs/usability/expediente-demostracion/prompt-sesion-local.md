# Prompt para la sesión local (copiar y pegar)

> Necesito cargar un expediente penal ficticio completo en nuestra instancia
> desplegada de Qadra para presentarlo a las 12:00. Trabaja con subagentes
> **Sonnet** en paralelo donde no haya dependencias y prioriza: a las 11:45 debe
> estar cargado al menos todo lo marcado P0.
>
> **Fuente única de verdad:** `docs/usability/expediente-demostracion/expediente.md`
> en la rama `feat/expediente-demostracion` (`git fetch origin feat/expediente-demostracion`
> y `git switch feat/expediente-demostracion`). Léelo completo antes de empezar. No
> inventes datos fuera de él; todo es ficticio y debe seguir así.
>
> **Antes de escribir nada en la instancia**, pregúntame una sola vez: URL base de
> la API, si la instancia es de demostración (todo registro y su auditoría son
> permanentes), correo y contraseña del Owner, y el código TOTP en el momento del
> login. Guarda credenciales y secretos TOTP de las cuentas nuevas sólo en
> `output/demo-case/credenciales.json` (fuera de Git). Nunca los imprimas completos.
>
> ## Fase 1, en paralelo (4 agentes Sonnet): documentos
>
> Cada agente redacta un grupo de fuentes Markdown en
> `docs/usability/expediente-demostracion/documentos/` (grupo 1: D01-D07 y D03b;
> grupo 2: D08-D16 y D12b; grupo 3: D17-D25; grupo 4: D26-D34). Formato exacto que
> lee `scripts/demo_case_documents.py` (léelo y mira la prueba
> `scripts/tests/test_demo_case_documents.py`):
>
> ```
> ---
> id: D07
> archivo: D07-acta-audiencia-inicial.pdf
> titulo: Acta mínima de audiencia inicial
> encabezado: Poder Judicial Demo de la Ciudad de México
> encabezado: Unidad de Gestión Judicial Demo 99 — Juzgado de Control Demo
> referencia: Carpeta judicial 999/2026 · NUC CI-DEMO/UI-99/00042/01-2026
> lugar_fecha: Ciudad de México, a 12 de enero de 2026
> ---
> # ACTA MÍNIMA DE AUDIENCIA INICIAL
>
> Párrafos separados por línea en blanco.
>
> ## Subtítulo
>
> - viñeta
>
> > Nombre de quien firma
> > Cargo
> ```
>
> Reglas: prosa jurídica formal en español, 1 a 2 páginas, con fechas, artículos y
> nombres de la tabla 3.4 y la cronología 3.2; autoridades siempre «Demo»; sin
> datos reales. Sólo caracteres admitidos: letras del español (á é í ó ú ñ ü y
> mayúsculas), ¿ ¡ « » – — “ ” ‘ ’ ° § º ª · … y ASCII; nada de →, ≤, ã ni emojis.
> Los documentos vivos (D27, D33, D34) usan marcadores `{{fecha_d27}}`,
> `{{fecha_rf12}}`, `{{fecha_rf13}}`, etc. Después compila:
>
> ```
> python3 -B scripts/demo_case_documents.py \
>   --source docs/usability/expediente-demostracion/documentos \
>   --out output/demo-case/pdf --skip-live
> ```
>
> y revisa 3 PDF renderizados. D04b (PNG) es opcional.
>
> ## Fase 1, en paralelo (1 agente Sonnet): cargador
>
> Escribe `output/demo-case/load.py` (fuera de Git, sólo biblioteca estándar,
> `urllib`) que cargue el expediente **en orden, una petición a la vez** (el
> servidor admite 8 en curso). Copia patrones y cuerpos JSON exactos de:
> `scripts/api-demo.sh` (login y `totp_code`), `scripts/api-members-demo.py`
> (`POST /users`), `scripts/api-case-administration-demo.sh` (`/penal-cases`),
> `scripts/api-typed-participant-demo.py` y `api-participant-demo.sh`,
> `scripts/api_document_admission.py` y `api-metadata-demo.sh` (carga con
> clasificación), `api-version-demo.sh`, `api-case-stage-demo.sh`,
> `api-hearings-demo.py`, `api_hearing_results_demo.py`,
> `api-hearing-derived-deadline-demo.py`, `api-precautionary-demo.py`,
> `api-procedural-facts-demo.py` y `api_procedural_facts_support.py`,
> `api-judicial-calendars-demo.py`, `api-deadline-profiles-demo.py`,
> `api_deadlines_support.py`, `api-legal-corpus-demo.py` y
> `api_legal_corpus_fixtures.py` (perfiles y condiciones del corpus),
> `api-procedural-resources-demo.py`, `api-resource-activities-demo.py`,
> `api-resource-hearings-demo.py`, `api-contextual-deadline-demo.py`,
> `api-alerts-demo.py`, `api-case-reports-demo.py`, `api-audit-events-demo.py`.
>
> Reglas del cargador:
> - Patrón `prepare` → envío con `{command, expected_submission_digest}` usando el
>   `command` normalizado que devuelve prepare; el compuesto resultado + plazo usa
>   `expected_review_digest`; las tres familias cautelares exigen ambos digests.
> - `operation_id` propio por escritura y archivo de estado
>   `output/demo-case/estado.json`; ante respuesta incierta, consulta el recibo
>   exacto y **nunca reenvíes a ciegas**. Debe poder reanudarse.
> - Programa **todas las audiencias de una etapa antes de transitar**; actos de
>   etapa y resultados nunca en el futuro; notificaciones con `stated_effect`.
> - Escrituras procesales con la sesión de Andrés; catálogos con la del Owner.
> - Si el calendario CDMX 2026 o los perfiles ya existen, reutilízalos (consulta antes).
>
> ## Prioridades de carga
>
> - **P0 (antes de 11:45):** cuentas y preferencias de alertas; caso principal con
>   NUC y carpeta; asignaciones; todos los PDF con clasificación (D01 tipo Contrato
>   sin sellar); participantes (los que no requieren PKI); H1 con resultado y RF1;
>   transición a Intermedia, H2 y resultado; transición a Juicio, H3d, H3, H3b, H4
>   (compuesto con P6) y H5; hechos RF1-RF11 con notificaciones; plazos P1-P9 con el
>   corpus y sus atenciones (P7 queda **sin** atención); sellar D07, D13 y D24.
> - **P1:** recursos R1-R6 con actos y asociaciones; decisiones cautelares;
>   versiones de D25; capa viva 3.3 (P10, convocatoria de revisión, P12 con
>   corrección); caso auxiliar y caso cerrado; informes 3+1; marcar una alerta leída.
> - **P2:** defensor tipificado con declaración firmada, firma individual de D25 v2,
>   certificado Owner (requieren PKI vigente), D04b, incidente de integridad.
>
> Ve reportando cada 10 minutos qué fase está cargada. Si algo falla dos veces,
> sáltalo, anótalo y sigue con lo siguiente: prefiero un recorrido completo con
> huecos documentados que nada. Si la API no alcanza, dime qué hacer a mano en
> Qadra siguiendo la tabla 3.2.
>
> Al terminar entrega: resumen de lo cargado y de lo omitido, credenciales de las
> cuentas de demostración (en el archivo local) y un orden de recorrido de 10
> minutos para la presentación: login con MFA → tablero y alertas → expediente
> principal (resumen, participantes, documentos con sello y verificación, etapas,
> audiencias, resoluciones, plazos con P7 vencido y P10 próximo, recursos) → Agenda
> → informes → auditoría → vista del Cliente.
