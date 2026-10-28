# Expediente ficticio de demostración: Olvera Castañeda vs. Ramírez Lugo y otros

Todo es ficticio: personas, despacho, autoridades, carpetas y documentos. No
copies un asunto real ni uses datos de clientes. Cada PDF lleva la marca
«DOCUMENTO FICTICIO — SIN VALOR LEGAL» (la pone `scripts/demo_case_documents.py`).

Revisado contra el CNPP, el Código Penal para el Distrito Federal (CPDF, vigente
en la CDMX) y las reglas reales de la API (tipos de participante, etapas,
soportes, recursos y plazos del corpus aceptado).

## 1. Premisas

- **El despacho asesora a la víctima** (CNPP 109-110). La cuenta Cliente es la víctima.
- **Delitos con pena máxima de 2 años** (categoría de 2 meses del art. 321 CNPP):
  - A, **Rebeca Ramírez Lugo**: lesiones dolosas simples, CPDF 130 fr. II (6 meses a 2 años), **de oficio**, sin calificativas.
  - B, **Sergio Peña Valdés**: amenazas, CPDF 209 párr. 1 (1 a 2 años o multa), **querella**; mal a bienes, sin arma ni explosivos.
  - C, **Tomás Ibáñez Corona**: daño a la propiedad doloso, CPDF 239 fr. II (6 meses a 2 años; daño de 9,500 pesos), **querella**.
  - Cada quien es autor de su propia conducta (CPDF 22 fr. I). **No se imputa pandilla (252) ni coparticipación.** Conexidad CNPP 30.
- **Hechos (miércoles 2026-01-07, estacionamiento de un condominio ficticio en la CDMX):**
  21:30 A empuja una vez a la víctima, que cae y sufre un esguince de muñeca (sana en ~22 días);
  21:40 B le dice que si vuelve a ocupar el cajón le «poncha las llantas y le raya el coche»;
  21:55 C rompe el espejo lateral y el parabrisas. Detención en flagrancia de A; denuncia (lesiones)
  y querella (amenazas y daño) esa noche; el MP los libera (CNPP 140) y pide citatorio (141 fr. I).
  La víctima no acepta acuerdos reparatorios (CNPP 189).
- **Dos capas de tiempo:** histórica fija (2026-01-07 a 2026-10-21, calendario oficial CDMX 2026 del
  corpus) y viva relativa a **T = momento de carga** (en horario hábil).
- **Sin perfil de cómputo (sólo declaraciones):** 48 h, 72/144 h, acusación 15 días, coadyuvancia 3 días,
  traslado/adhesión, ventanas de los arts. 341, 347, 349 y 351, plazos cautelares.

## 2. Despacho y cuentas

Despacho: **Ruiz Trejo y Asociados, S.C. (ficticio)**. Correos en `@despacho-demo.example.test`.

| Persona | Rol Qadra | Asignación | Uso |
| --- | --- | --- | --- |
| Elena Ruiz Trejo | Owner | todos | catálogos, cierre, auditoría (si ya existe un Owner real, usa ese) |
| Andrés Varela Soto | Litigante | principal y cerrado | asesor jurídico de la víctima; **autor de las escrituras procesales**; firma individual |
| Paula Medina Ortiz | Litigante | auxiliar | segundo litigante (carga de trabajo) |
| Diana Cruz Pineda | Paralegal | principal y auxiliar | lee, carga y verifica; no puede sellar |
| Mariana Olvera Castañeda | Cliente | principal | víctima; sólo ve la proyección básica |
| Jorge Ibarra León | Litigante | ninguna | cuenta para asignar/retirar y comprobar aislamiento |

## 3. Caso principal

- **Título:** «Olvera Castañeda vs. Ramírez Lugo y otros — lesiones, amenazas y daño». Referencia: `RTA-2026-014`.
- **Alta:** `POST /api/v1/penal-cases` (perfil completo + Investigación inicial), hecha por Andrés.
- **NUC:** `CI-DEMO/UI-99/00042/01-2026`. **Carpeta judicial:** `999/2026`. Ambos ficticios y únicos en la instancia.
- **Autoridades (ficticias):** Fiscalía de Investigación Territorial Demo, Unidad 99; Juzgado de Control Demo,
  Unidad de Gestión Judicial Demo 99; Tribunal de Enjuiciamiento Unitario Demo; Sala Penal Demo.
- **Delitos:** los tres de §1.

### 3.1 Participantes

| Ficha | Tipo | Perfil / sujeto | Soportes |
| --- | --- | --- | --- |
| Mariana Olvera Castañeda | tipificada | `victim`, persona física | identidad D05; rol D07 |
| Andrés Varela Soto | tipificada | `victim_counsel`, persona física | rol D06 |
| Rebeca Ramírez Lugo (A) | tipificada | `defendant`, libertad | identidad y rol D07 (localizador propio) |
| Sergio Peña Valdés (B) | tipificada | `defendant`, libertad | D07 (localizador propio) |
| Tomás Ibáñez Corona (C) | tipificada | `defendant`, libertad | D07 (localizador propio) |
| Lic. Julián Montes Arriaga | tipificada **sólo si hay PKI**; si no, manual | `defense_counsel` de A (exige declaración firmada) | D07 |
| Lic. Karla Benítez Robles | manual | defensora pública de B | — |
| Lic. Óscar Treviño Lara | manual | defensor de C | — |
| Lic. Fernanda Aguilar Ponce | tipificada | `prosecutor`, **persona física**, sin firma | D07 |
| Mtro. Ernesto Vidal Quiroga | manual | juez de control | — |
| Tribunal de Enjuiciamiento Unitario Demo | tipificada | `trial_court`, **órgano institucional** (distrito CDMX, unitario), sin firma | D14 |
| Mtra. Lucía Cárdenas Solís | manual | jueza de enjuiciamiento | — |
| Dra. Alejandra Fuentes Mora | tipificada | `expert` | D03 |
| Martín Robles Cano | tipificada | `police` | D04 |
| Lic. Rosa Méndez Ávila | tipificada | `precautionary_supervisor`, **persona física**, organización «Unidad de Supervisión de Medidas Cautelares Demo» | D07 |
| Laura Gómez Ríos | manual | testigo; se archiva tras el auto de apertura (no ofrecida) | — |

Cada identidad tipificada usa un localizador distinto aunque comparta documento; máximo 2 versiones de soporte por operación.

### 3.2 Cronología histórica (hora -06:00; escrituras con la sesión de Andrés)

| Fecha | Etapa | Acto | Registro en Qadra | Plazo |
| --- | --- | --- | --- | --- |
| 01-07 / 01-08 | Inv. | Hechos, detención de A, denuncia y querella, dictámenes, contrato | D01-D05, D03b, D04b (PNG) | — |
| 01-09 | Inv. | Nombramiento de asesor jurídico | D06 | — |
| 01-12 10:00 | Inv. | **H1 `initial`**: imputación; los tres no se acogen al plazo constitucional (CNPP 313) y se vinculan; cautelares; plazo de investigación 1 mes fecha a fecha, cierre 18:00 | H1 + resultado (D07); **RF1** (procedencia H1); decisión cautelar con ancla `initial`: A `contact_restriction` + `periodic_appearance`, B `contact_restriction`, C `periodic_appearance`, supervisora Rosa | **P1** = 2026-02-12 18:00 (ficha 1) |
| 02-04 | Inv. | Juez niega anticipo de prueba (la testigo sólo viaja; CNPP 304 fr. II exige residir en el extranjero); notificación en audiencia, efectos 02-05 | **RF2** (procedencia `external_reference` D08) + notificación `in_hearing` | **P3** = 2026-02-09 15:00 (ficha 5) |
| 02-06 | Inv. | Asesoría apela; acuerdo que tiene por recibida la solicitud de prórroga del MP | **R1** `appeal` escrita + `interposition` (D09); atención de P3; **RF3** (D10) | **P11** bloqueado (solicitud sin concesión) — ejemplo; puede retirarse con motivo |
| 02-10 | Inv. | Auto concede 1 mes de prórroga desde 02-12 (acumulado 2) | **RF4** (D11); atención de P1 | **P2** = 2026-03-12 18:00 (ficha 2) |
| 03-10 | Inv. | Sala confirma la negativa | R1 `resolution` (D28) | — |
| 03-12 | Inv. | Cierre de investigación | atención de P2 | — |
| 04-06 | Inv.→Int. | Acusación del MP | **transición** a Intermedia (D12) | — |
| 04-09 | Int. | Coadyuvancia y cuantificación de la reparación (CNPP 338) | D12b | — |
| 05-07 → 05-13 | Int. | **H2 `intermediate`** programada 05-07 y diferida a 05-13 a petición de la defensa | programar + reemplazar (motivo) | — |
| 05-13 10:00 | Int. | H2: B en procedimiento abreviado; la asesoría objeta el monto de reparación (CNPP 204, 206); sentencia de B leída y notificada (efectos 05-14); acuerdos probatorios; auto de apertura para A y C; revisión cautelar de B | resultado (D13); **RF5** + notificación; convocatoria cautelar de revisión (objetivo: medida de B) + decisión `cease` anclada | **P4** = 2026-05-20 15:00 (ficha 6) |
| 05-18 | Int. | Asesoría apela la reparación de B | **R2** `appeal` + `interposition` (D16); atención de P4 | — |
| 05-19 | Int.→Juicio | Tribunal recibe el auto de apertura (emitido 05-13) | **transición** a Juicio (D14, D15) | — |
| 06-30 | Juicio | Sala modifica la reparación de B | R2 `resolution` (D29) | — |
| 07-08 | Juicio | Debate programado y **cancelado** (testigo con impedimento) | `oral_trial` programado y cancelado | — |
| 08-19 10:00 | Juicio | **H3d `oral_trial`**, antes de abrir el debate: MP se desiste respecto de C con autorización de su titular (CNPP 144); el tribunal lo aprueba y sobresee; notificado en audiencia (efectos 08-20) | resultado (D17); **RF6** + notificación | **P5** = 2026-08-24 15:00 (ficha 7) |
| 08-21 | Juicio | Asesoría apela el desistimiento | **R3** `appeal` + `interposition` (D18); atención de P5; vincular R3↔H3d y luego **desvincular** | — |
| 09-09 / 09-10 | Juicio | Acuerdo de trámite fuera de audiencia; notificación en el órgano el jueves 09-10 (efectos vie 09-11; 14, 15 y 16-09 inhábiles) | **RF9** (D19) + notificación `outside_hearing` | **P7** = 2026-09-17 15:00 — **sin atención** (alerta de vencido) |
| 09-24 | Juicio | Sala confirma el desistimiento | R3 `resolution` (D31) | — |
| 09-25 | Juicio | Cesan las medidas de C (sobreseimiento firme) | decisión `cease` para C **sin ancla** | — |
| 09-30 10:00 | Juicio | **H3 `oral_trial`**: apertura del debate; se suspende (CNPP 351 fr. III) | resultado S1 | — |
| 10-05 10:00 | Juicio | **H3b `oral_trial`**: continuación; fallo condenatorio de A | resultado S2 con `continuation` a S1; D20 | — |
| 10-09 10:05 | Juicio | **H4 `sentencing`** (antecedente de condena D20): acuerdo de trámite en audiencia; asesoría interpone revocación oral (10:20), resuelta en el acto; fin declarado 10:50:00 | **compuesto resultado + plazo derivado**; **RF7** (procedencia H4); **R4** `revocation` oral + `interposition` + `resolution` (D21); atención de P6 a 10:20 | **P6** = 2026-10-09 10:50:00 (ficha 3) |
| 10-13 / 10-14 | Juicio | Acuerdo de trámite (copias de registros) fuera de audiencia, notificado en el órgano 10-14 (efectos 10-15) | **RF10** (D22) + notificación | **P8** = 2026-10-16 14:00 (ficha 4) |
| 10-14 12:00 | Juicio | **H5 `sentencing`**: lectura y explicación de la sentencia de A con versión escrita (efectos 10-15) | resultado con `continuation`; **RF11** (D24) + notificación | **P9** = 2026-10-28 15:00 (ficha 8) |
| 10-15 | Juicio | Revocación escrita | **R5** `revocation` escrita + `interposition` (D23); atención de P8 | — |
| 10-19 | Juicio | Tribunal resuelve la revocación (ordena expedir copias) | R5 `resolution` (D30) | — |
| 10-20 | Juicio (alzada) | Asesoría apela la sentencia de A: omisión de condenar al daño moral y a la atención psicológica acreditados (agravio de derecho, CNPP 468); pide alegatos aclaratorios | D25 v1 y **v2**; v2 **firmada individualmente** por Andrés (si hay PKI); **R6** `appeal` + `interposition` (D25 v2); atención de P9; vincular R6↔H5 y R6↔P9 | — |
| 10-21 | Juicio (alzada) | Acuerdo de traslado de la apelación | D32 | — |

Esperados verificados (calendario CDMX 2026): P3 1770670800, P4 1779310800, P5 1787605200,
P7 1789678800 (unix); P1, P2, P6, P8 y P9 iguales a `docs/four-front-closure.md`.

### 3.3 Capa viva (relativa a T, en horario hábil)

| Instante | Registro | Efecto |
| --- | --- | --- |
| T − 26 h (día hábil anterior) | MP pide modificar la medida de A tras la condena (CNPP 161) | D27 |
| T − 3 h | **RF12**: el tribunal da vista a la asesoría; notificación a T − 2 h con precisión **de segundo** y desfase -21600 | fuente de P10 |
| T | **P10** contextual desde R6 con perfil interno `elapsed_hours` 48 h («regla interna del despacho, no plazo legal») | vence T + 46 h: alerta próxima y tablero 48 h |
| siguiente día hábil 10:00 (≥ T + 18 h) | **Convocatoria cautelar de revisión** de A ante el tribunal (objetivo: cabeza de la medida de A tras una rectificación administrativa) | Agenda y alerta próxima |
| T | **RF13** + notificación + **P12** (interno 72 h, `follow`); después se **corrige** la notificación de RF13 | revisión técnica `review_required` y alerta «revisión requerida» |
| T | P7 sigue sin atención | alerta `overdue_unattended` y tablero |
| T ≥ 2026-10-29 | R6 `admission` (D26) y audiencia `appeal_arguments` 2026-11-10 10:00 (soporte D26 mediante el acto) | si T es anterior, se omite |

### 3.4 Documentos (PDF salvo D04b)

| Id | Archivo | Contenido mínimo |
| --- | --- | --- |
| D01 | `D01-contrato-servicios-asesoria.pdf` | Contrato despacho–víctima (2026-01-08). **Tipo «Contrato», nunca sellar** (tablero) |
| D02 | `D02-denuncia-querella.pdf` | Denuncia por lesiones y querella por amenazas y daño (2026-01-07, 23:50) |
| D03 | `D03-dictamen-medico.pdf` | Lesiones que no ponen en peligro la vida y sanan en más de 15 y menos de 60 días. Reclasificar una vez |
| D03b | `D03b-dictamen-valuacion.pdf` | Valuación del daño: 9,500 pesos |
| D04 | `D04-informe-policial.pdf` | Informe de la detención en flagrancia de A |
| D04b | `D04b-croquis.png` | Croquis del estacionamiento (familia imagen) |
| D05 | `D05-constancia-identidad-victima.pdf` | Constancia ficticia de identidad, sin datos reales |
| D06 | `D06-nombramiento-asesor.pdf` | Nombramiento de Andrés como asesor jurídico (01-09) |
| D07 | `D07-acta-audiencia-inicial.pdf` | Acta mínima de H1: imputaciones con artículos, CNPP 313, vinculación, cautelares, plazo 1 mes fecha a fecha cierre 18:00. **Sellar** |
| D08 | `D08-auto-niega-anticipo.pdf` | Negativa del anticipo (CNPP 304 fr. II) |
| D09 | `D09-apelacion-anticipo.pdf` | Apelación de la asesoría (02-06) |
| D10 | `D10-acuerdo-solicitud-prorroga.pdf` | Tiene por recibida la solicitud de prórroga (sin concederla) |
| D11 | `D11-auto-concede-prorroga.pdf` | Concede 1 mes desde 02-12; acumulado 2 de 2 |
| D12 | `D12-escrito-acusacion.pdf` | Acusación del MP (04-06) |
| D12b | `D12b-coadyuvancia-reparacion.pdf` | Coadyuvancia y cuantificación de la reparación (04-09) |
| D13 | `D13-acta-intermedia-abreviado.pdf` | Acta de H2 y sentencia abreviada de B, con la objeción de la asesoría. **Sellar** |
| D14 | `D14-auto-apertura.pdf` | Auto de apertura para A y C (05-13) |
| D15 | `D15-constancia-recepcion.pdf` | Recepción por el tribunal (05-19) |
| D16 | `D16-apelacion-reparacion-b.pdf` | Apelación (05-18) |
| D17 | `D17-resolucion-desistimiento.pdf` | Autorización del titular, aprobación y sobreseimiento respecto de C (08-19) |
| D18 | `D18-apelacion-desistimiento.pdf` | Apelación (08-21) |
| D19 | `D19-acuerdo-tramite-septiembre.pdf` | Acuerdo 09-09 y constancia de notificación 09-10 |
| D20 | `D20-fallo-condenatorio.pdf` | Fallo condenatorio de A (10-05) |
| D21 | `D21-acta-individualizacion.pdf` | Acta de H4 con la revocación oral y su resolución |
| D22 | `D22-acuerdo-tramite-octubre.pdf` | Acuerdo 10-13 y constancia de notificación 10-14 |
| D23 | `D23-escrito-revocacion.pdf` | Revocación escrita (10-15) |
| D24 | `D24-sentencia-a.pdf` | Sentencia de A, versión escrita (10-14). **Sellar, verificar y exportar ZIP** |
| D25 | `D25-apelacion-sentencia-a.pdf` | Apelación de A (10-20): v1 borrador y v2 final; v2 firmada por Andrés |
| D26 | `D26-admision-apelacion.pdf` | Admisión y señalamiento de alegatos para 2026-11-10 (sólo si T ≥ 10-29) |
| D27 | `D27-solicitud-revision-medida.pdf` | **Viva**: solicitud del MP de modificar la medida de A |
| D28 | `D28-resolucion-sala-anticipo.pdf` | Sala confirma (03-10) |
| D29 | `D29-resolucion-sala-reparacion-b.pdf` | Sala modifica la reparación (06-30) |
| D30 | `D30-auto-resuelve-revocacion.pdf` | Tribunal resuelve la revocación (10-19) |
| D31 | `D31-resolucion-sala-desistimiento.pdf` | Sala confirma el desistimiento (09-24) |
| D32 | `D32-acuerdo-traslado-apelacion.pdf` | Traslado de la apelación (10-21) |
| D33 | `D33-acuerdo-vista.pdf` | **Viva**: vista sobre la solicitud de D27 (RF12) |
| D34 | `D34-acuerdo-comprobantes.pdf` | **Viva**: requiere comprobantes de gastos médicos (RF13) |

## 4. Casos auxiliares

- **Auxiliar en Investigación:** «Salgado Ruiz — abuso de confianza» (querella, CPDF 227). Alta básica,
  completar perfil y **adopción de Investigación** con la querella; audiencia `initial` el siguiente día hábil
  ≥ T + 3 d. Asignado a Paula y Diana. Sirve para tareas de usabilidad y como distractor.
- **Cerrado:** «Fuentes Lara — fraude, acuerdo reparatorio cumplido». Alta penal completa, un documento y
  **cierre administrativo** en T. Asignado a Andrés.

## 5. Supervisión al final

- Preferencias de alertas por cuenta **antes** de crear lo próximo; luego esperar alertas y marcar una leída.
- Tablero de Owner y de Andrés; Agenda de la semana.
- Informes: 3 de estado y luego un 4.º (muestra estimación); 3 de actividad y uno de Andrés.
- Auditoría filtrada por el expediente y verificación de la cadena.
- Sellar D07, D13 y D24; verificar y exportar el ZIP de D24; descargar el contenido exacto de D24.

## 6. Requisitos de la instancia

- **PKI vigente** (CRL renovada y confianza publicada) para: defensor tipificado con declaración, firma
  individual de D25 y certificado Owner. Si no hay tiempo, omítelos y muéstralos con lo ya aceptado.
- **Correo (Resend)** sólo para invitaciones, recuperación y avisos por correo.
- Calendario CDMX 2026 y perfiles del corpus: **revisar si ya están publicados** antes de publicarlos
  (`scripts/legal_corpus_calendar.py`, `legal_corpus_investigation.py`, `legal_corpus_resources.py`,
  `api_legal_corpus_fixtures.py`).
- **Cargar en una instancia de demostración.** Todo registro y su auditoría son permanentes: en una
  instalación con datos reales, el caso ficticio no se podrá borrar.
