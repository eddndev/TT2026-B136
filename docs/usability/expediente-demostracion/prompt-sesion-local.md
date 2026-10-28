# Prompt para la sesión local (copiar y pegar)

> Necesito preparar en nuestra instancia desplegada de Qadra, para presentar a las
> 12:00, un expediente penal **ficticio** centrado en los **servicios
> criptográficos**. No cargues plazos, recursos, medidas cautelares ni el resto de
> la gestión procesal: sólo lo necesario para mostrar integridad, confidencialidad,
> no repudio y autenticación. Usa subagentes **Sonnet** en paralelo donde no haya
> dependencias. A las 11:45 debe estar listo todo lo marcado P0.
>
> **Contexto:** `docs/usability/expediente-demostracion/expediente.md` en la rama
> `feat/expediente-demostracion` (`git fetch origin feat/expediente-demostracion`
> y `git switch feat/expediente-demostracion`). Usa de ahí sólo el caso principal
> (§3, título, NUC, carpeta y delitos), las cuentas (§2) y los documentos D01, D05,
> D07, D13, D20, D24 y D25. Todo es ficticio y debe seguir así.
>
> **Antes de escribir en la instancia**, pregúntame una sola vez: URL base de la
> API, si la instancia es de demostración (los registros y su auditoría son
> permanentes), correo y contraseña del Owner y el código TOTP cuando haga falta.
> Guarda credenciales y secretos TOTP de cuentas nuevas sólo en
> `output/demo-case/credenciales.json` (fuera de Git) y no los imprimas.
>
> ## Fase 1, en paralelo
>
> **Agente A (Sonnet): documentos.** Redacta las fuentes Markdown de D01, D05, D07,
> D13, D20, D24 y D25 (v1 borrador y v2 final) en
> `docs/usability/expediente-demostracion/documentos/`, con el formato que lee
> `scripts/demo_case_documents.py` (léelo y su prueba
> `scripts/tests/test_demo_case_documents.py`): bloque `---` con `id`, `archivo`,
> `titulo`, `encabezado` (repetible), `referencia` y `lugar_fecha`; luego `# título`,
> párrafos, `## subtítulos`, `- viñetas` y `> líneas de firma`. Prosa jurídica
> formal, 1 a 2 páginas, autoridades «Demo», sin datos reales; sólo letras del
> español, ¿ ¡ « » – — “ ” ‘ ’ ° § º ª · … y ASCII. Compila:
> `python3 -B scripts/demo_case_documents.py --source docs/usability/expediente-demostracion/documentos --out output/demo-case/pdf`
> y revisa visualmente 2 PDF.
>
> **Agente B (Sonnet): cargador.** Escribe `output/demo-case/load.py` (fuera de Git,
> biblioteca estándar, una petición a la vez, archivo de estado para reanudar y
> nunca reenviar a ciegas). Copia los cuerpos JSON exactos de `scripts/api-demo.sh`
> (login y `totp_code`), `scripts/api-members-demo.py` (`POST /users`),
> `scripts/api-case-administration-demo.sh` (`/penal-cases`),
> `scripts/api_document_admission.py` y `api-metadata-demo.sh` (carga con
> clasificación), `api-version-demo.sh` (versiones, sello, verificación y evidencia
> por versión), `api-document-content-demo.py` (contenido exacto),
> `api-individual-signing-demo.py` y `api_individual_signing_support.py` (vínculo y
> firma individual), `api-owner-certificates-demo.py` (vínculo del Owner) y
> `api-audit-events-demo.py`.
>
> ## Fase 2: carga por prioridad
>
> - **P0 (antes de 11:45)**
>   1. Cuentas: Andrés (Litigante), Diana (Paralegal), Mariana (Cliente).
>   2. Caso principal por `/penal-cases`, creado por Andrés, con Owner, Diana y Mariana asignados.
>   3. Subir los PDF con clasificación; D01 de tipo Contrato y sin sellar.
>   4. Sellar D07, D13 y D24 (firma RSA-3072 del servicio + token RFC 3161 de la TSA local).
>   5. Verificar D24.
>   6. Exportar el ZIP de evidencia de D24 a `output/demo-case/evidencia/`.
>   7. D25: subir v1 y luego v2 como nueva versión; sellar v1 y comprobar que v2 tiene otro SHA-256.
>   8. Descargar el contenido exacto de D24 y comparar su SHA-256 con el de la ficha.
>   9. `GET /api/v1/audit/verify` y consulta de auditoría filtrada por el expediente.
> - **P1**
>   - **Verificación independiente:** descomprime el ZIP de D24 y ejecuta los cuatro pasos de su `INSTRUCCIONES.md` con `openssl`: digest, firma, certificado y CRL, sello de tiempo. Guarda la salida en `output/demo-case/evidencia/verificacion.txt`.
>   - **Alteración:** prepara una copia del ZIP con un byte cambiado (`scripts/flip-byte.sh`) y la salida de verificación que falla.
>   - **Comprobaciones de rol:** la Paralegal no puede sellar (403); la Cliente no ve documentos (403).
> - **P2** (requieren PKI vigente: CRL renovada y confianza publicada)
>   - **Firma individual de D25 v2 por Andrés:** vínculo `document-signing` y firma de la versión y del consentimiento; luego verificar y exportar su paquete. Si la llave debe generarse en el navegador, déjame indicado el paso en Qadra («Mi certificado»).
>   - **Vínculo de certificado del Owner** y acceso por certificado, sólo si la instancia lo tiene habilitado.
>   - **Portal CA:** emisión o revocación sólo si está desplegado; no lo toques si no.
>
> Si algo falla dos veces, sáltalo, anótalo y sigue. Repórtame cada 10 minutos.
>
> ## Entrega
>
> 1. Resumen de lo cargado y omitido, con el SHA-256 de cada documento y los
>    identificadores de los documentos sellados.
> 2. Credenciales en el archivo local.
> 3. Guion de 10 minutos para la presentación, en este orden:
>    1. Login con contraseña Argon2id + TOTP.
>    2. Expediente → Documentos: huella SHA-256 y almacenamiento cifrado AES-256-GCM.
>    3. Sellar en vivo un documento; mostrar firma y token de tiempo.
>    4. Verificar D24 (resultado integral).
>    5. Descargar el ZIP y verificarlo con `openssl` sin Qadra (comandos listos para copiar).
>    6. Verificar el ZIP alterado y mostrar el rechazo.
>    7. Versiones de D25 con huellas distintas.
>    8. Firma individual del titular (si quedó lista) y su verificación.
>    9. Auditoría: consulta y verificación de la cadena hash.
>    10. Vista de la Cliente y rechazo de la Paralegal al sellar.
>    11. Si está desplegado, portal CA y CRL.
