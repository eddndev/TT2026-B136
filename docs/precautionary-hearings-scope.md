# Alcance de audiencias y medidas cautelares declaradas

Estado: **propuesto**. Este contrato fija el alcance funcional; no acredita su
implementación completa ni aceptación. La decisión de arquitectura se conserva
en [ADR-0071](adr/0071-declared-precautionary-hearings-and-measures.md).

## Frontera de la implementación local

El primer checkpoint implementa localmente valores de convocatoria: identidad propia,
operación, revisión positiva, propósito `imposition` o `review`, hora exacta,
modalidad, lugar, nota opcional, participantes con identidad/revisión exactas,
únicos y ordenados, y soporte exacto
con declaración y localizador obligatorios. Una revisión selecciona de una a 32
medidas por identidad, revisión y digest; una imposición no acepta objetivos de
revisión. `PHEAR1` debe comprometer todos esos valores sin cambiar `HEAR1` ni
`RHEAR1`. La verificación focal de dominio está aprobada; las comprobaciones
remotas y la integración siguen pendientes.

La ampliación local de valores comprende `MeasureKind`, `MeasureTime` y
`MeasureValidity`, con el compromiso `MVAL1`. Diez pruebas focales y Clippy
aprobaron tras el fallo inicial de las pruebas. Esta evidencia es independiente
de las once pruebas anteriores de convocatoria; la integración sigue pendiente.

La capa de aplicación valida ahora un contexto histórico completo (`PCTX1`):
administración observada, etapa y administración exacta que originó esa etapa,
con valores, soportes y procedencia. `PHTXN1` vincula autor y rol capturados,
expediente, operación, convocatoria, acción, revisiones, digest previo, contexto,
valores y motivo. Son comprobaciones y compromisos de datos; no acreditan acceso
vigente, admisión documental nueva ni una operación guardada.

La resolución local de participantes verifica de cero a 32 capturas exactas,
manuales o tipadas, con sus valores y sujeto histórico vinculado. Rechaza fuentes
ajenas, faltantes, duplicadas o alteradas y deriva las etiquetas de esas mismas
revisiones. Permite reconstruir capturas archivadas sin autorizar su selección
para una nueva operación.

Aún faltan recibos de captura, decisiones, historia de medidas, servicios del
flujo, persistencia, HTTP, Agenda, alertas e interfaz. Estas capacidades y
la restauración de la familia cautelar siguen pendientes. Un catálogo o sus
pruebas unitarias no cumplen por sí solos el flujo completo. Los resultados
ejecutados se registran por separado en [el informe](verification-report.md).

## Fuente primaria y límites de interpretación

La consulta del 4 de octubre de 2026 al
[CNPP de la Cámara de Diputados](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf)
y al [índice oficial de reformas](https://www.diputados.gob.mx/LeyesBiblio/ref/cnpp.htm)
identificó como última reforma reportada la del 28-11-2025. El SHA-256 del PDF
consultado fue
`7ceb0682c948d7d945e7e68b8d8699ceb795fc9a491aa62325db91ccf2309904`.
La fecha y huella identifican esa consulta; no certifican la ausencia de otra
disposición o resolución aplicable.

| Artículos del CNPP | Base resumida | Consecuencia técnica propuesta |
| --- | --- | --- |
| 153, 154, 157 | Imposición judicial con supuestos y decisión en audiencia. | Separar solicitud, convocatoria y decisión; capturar fundamento y soporte. |
| 155, 156, 157 | Catorce medidas, proporcionalidad y restricciones de combinación. | Clasificación cerrada; sin recomendación o validación jurídica automática. |
| 159 | Justificación, lineamientos y vigencia en la resolución. | Conservar declaraciones y evidencia, incluidas sus carencias. |
| 161-163 | Revisión a petición y audiencia; el término de 48 horas tiene un supuesto. | La solicitud no modifica la medida; no generar `+48h` por seleccionar revisión. |
| 164 | Evaluación y supervisión por autoridades determinadas. | Registrar la fuente sin certificar cumplimiento o atribuciones. |
| 307 | La audiencia inicial comprende solicitudes cautelares. | Enlazar la audiencia existente sin duplicar la cita. |
| 347, 401, 405 | Medidas al abrir juicio y levantamiento en absolución. | Conservar historia y decisión expresa, sin terminar por cambio de etapa. |
| 137 | Medidas u órdenes de protección con régimen propio. | No incluirlas como sinónimos del catálogo cautelar. |

Fuente de la tabla: [CNPP, artículos indicados](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).
La última columna expresa decisiones de diseño, no reglas de procedencia. El
prototipo no calcula competencia, proporcionalidad, cumplimiento ni efectos de
impugnaciones. Tampoco convierte un máximo legal en una duración concedida.

## Catálogo finito de audiencias del producto

| Familia o relación | Contrato y frontera |
| --- | --- |
| Inicial | [Familia ordinaria](hearings-api.md); conserva programación y [resultados declarados](hearing-results-api.md). La materia cautelar puede enlazarse a esa misma cita. |
| Intermedia | Familia ordinaria existente; admisión o exclusión probatoria es contenido de actos, no otra etiqueta de cita. |
| Juicio oral | Familia ordinaria existente; sus sesiones/resultados no acreditan firma judicial por usar la firma interna del prototipo. |
| Individualización y reparación | Familia ordinaria existente; conserva el contexto condenatorio declarado y su soporte. |
| Medidas cautelares | Familia propia propuesta: convocatoria de imposición o revisión, decisiones y medidas con historia. |
| Alegatos de apelación | [Familia propia de recurso](resource-hearings.md), con su origen independiente. |
| Audiencia de revocación escrita | Familia propia de recurso; la revocación oral no crea una cita separada. |
| Continuación | Relación exacta con sesión/resultado anterior, no una octava especie de audiencia. |

La matriz acota el compromiso del prototipo, sin pretender enumerar todas las
audiencias del CNPP. No reemplaza otros criterios del
[alcance del producto](product-completion.md) ni declara terminado el documento
académico. Las descripciones de
[audiencias](../latex/chapters/03-audiencias.tex) y su
[anexo](../latex/chapters/anexo-e-audiencias.tex) conservan su alcance versionado.

## Tres registros con historia propia

1. **Convocatoria cautelar:** propósito, cita exacta, modalidad/lugar,
   participantes y soporte. Reprogramación y cancelación conservan historia.
   La revisión refiere medidas exactas sin afirmar que fueron modificadas.
2. **Decisión declarada:** identidad de resolución/autoridad, tiempo con su
   precisión, justificación y soporte exacto. Puede enlazar una audiencia inicial
   o convocatoria cautelar mediante referencia discriminada; no inventa una
   audiencia ausente ni usa un resultado concluido como prueba de imposición.
3. **Medida:** identidad estable, sujeto con ficha/revisión exacta, clase,
   condiciones, inicio y vigencia declarados, decisión de origen e historia.
   Persiste aunque cambie la cita, etapa o estado administrativo del expediente.

La convocatoria propia captura etapa y administración observadas, junto con el
contexto declarado. Puede registrar un señalamiento comunicado en cualquiera de
las tres etapas sin convertirlas en una regla de competencia. Toda referencia
contradictoria, ajena o cuyo contexto revisado cambió debe rechazarse.

### Clasificación cerrada de medidas

Los códigos técnicos corresponden, en orden, a las catorce fracciones del
[artículo 155 del CNPP](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf).
Son clasificaciones declaradas, no una elección automática de medida aplicable.

| Código | Clasificación declarada |
| --- | --- |
| `periodic_appearance` | Presentación periódica |
| `financial_guarantee` | Garantía económica |
| `asset_seizure` | Embargo de bienes |
| `account_freeze` | Inmovilización de cuentas y valores |
| `travel_restriction` | Restricción de salida del ámbito fijado |
| `custody_or_institution` | Cuidado, vigilancia o internamiento indicado |
| `place_restriction` | Restricción de reuniones o lugares |
| `contact_restriction` | Restricción de convivencia, acercamiento o comunicación |
| `home_separation` | Separación del domicilio |
| `public_office_suspension` | Suspensión en cargo público |
| `professional_suspension` | Suspensión de actividad profesional o laboral |
| `electronic_monitoring` | Localizador electrónico |
| `home_confinement` | Resguardo domiciliario |
| `pretrial_detention` | Prisión preventiva |

Conservar el texto de las condiciones sin inferir montos, zonas, periodicidad o
plazos. Si la fuente no permite clasificar, conservar documento/borrador; no
publicar una medida ficticia bajo `other`. Una persona supervisora se captura
con su revisión exacta o una declaración explícita de desconocimiento; un título
en el directorio no prueba nombramiento oficial.

### Tiempo y estado visible

Separar fecha declarada de decisión, inicio declarado, fin declarado y captura
real del servidor. Una fecha civil y un instante con desfase no son
intercambiables. `MeasureTime` conserva precisión, componentes y desfase opcional.
`Unknown` exige motivo; `Date`, `Minute` y `Second` prohíben el motivo de valor
desconocido. No inventar medianoche, UTC, duración o la hora de captura.

`MeasureValidity` compara inicio y término solo con estas reglas:

| Precisiones declaradas | Comparación permitida |
| --- | --- |
| `Date` / `Date` | Orden civil cuando los desfases opcionales son iguales, incluidos ambos ausentes; sin convertir la fecha en un instante. |
| `Minute` / `Minute` o `Second` / `Second` | Orden en UTC únicamente con ambos desfases explícitos; conserva componentes y desfases originales. |
| Precisión mezclada, hora sin desfase, fechas civiles con desfases opcionales distintos o un valor desconocido | Conserva las declaraciones y deja el orden sin resolver. |

Rechazar un término anterior al inicio si la pareja es comparable. Conservar una
pareja no comparable no acredita que su orden cronológico sea correcto.

La vigencia conserva texto expreso. El término ausente difiere de un término
declarado `Unknown` con motivo; ninguno significa perpetuidad ni conclusión.
`MVAL1` vincula precisión, componentes, desfase opcional y motivo del inicio,
la declaración de vigencia y la presencia y valor completo del término opcional.
Es un compromiso de valores, sin recibo, decisión o medida persistida.
La interfaz debe decir **última declaración registrada**; el reloj, cierre del
expediente y cambio de etapa no modifican el estado jurídico. Obligaciones
periódicas y términos requieren un contrato expreso adicional; no se extraen
automáticamente del texto de condiciones.

### Decisiones, grupos y rectificación

- `measure_changes` requiere cambios; `no_measure_change` exige cero cambios y
  resultado observado con razón y soporte. Desconocer el resultado sigue siendo
  incompleto. La no celebración pertenece a la cita.
- Una decisión puede afectar varios sujetos y hasta 32 identidades de medida,
  contando anteriores y nuevas. Es un límite técnico; no dividir silenciosamente
  una decisión mayor. Ordenar por UUID y rechazar objetivos repetidos.
- `impose`, `confirm`, `modify`, `revoke` y `cease` conservan decisiones y
  predecesores exactos. Confirmar agrega evidencia sin cambiar origen, sujeto,
  clase, condiciones, inicio o vigencia; modificar no reescribe el pasado.
- `substitute` enlaza uno o más predecesores y uno o más sucesores del mismo
  sujeto. Identidades disjuntas y únicas, sin ciclos ni parejas uno a uno
  inventadas; enlaces y revisiones se confirman juntos.
- `correct_record` rectifica texto/precisión con razón, soporte original y
  predecesor exacto. No cambia identidad, sujeto, clase, origen, efecto judicial
  ni decisión histórica. Solo actualiza la proyección desde su cabeza exacta.
- Identidad capturada por error requiere `entered_in_error`, conservando
  historia. Una entrada correcta tiene identidad nueva y enlace administrativo
  atómico; no se presenta como revocación o sustitución judicial. Las decisiones
  posteriores requieren conciliación expresa, sin cascadas inferidas.

## Autorización, evidencia y recuperación pendientes

Owner y Litigator con asignación vigente preparan/confirman; Paralegal asignado
consulta; Client permanece denegado. Mutar requiere administración activa.
Historia y replay requieren autorización actual. Reautenticar al principal
completo y conservar por separado ID, correo y rol históricos del autor.

La preparación resuelve documentos, participantes, sujetos y medidas por su
revisión/digest exactos; las cabezas vigentes se observan aparte. Admite soportes
fuera del bloqueo de auditoría y no crea objetos. Confirmar vuelve a comprobar
identidad, membresía, administración, contexto, soportes y revisiones esperadas
dentro de la transacción.

Una operación UUID compromete una instrucción inmutable. Replay idéntico y
autorizado devuelve el grupo original; otra instrucción entra en conflicto.
Encontrar registros aislados por ID no prueba origen conjunto. Decisión, todas
las revisiones, enlaces, origen y un evento de auditoría del grupo se confirman
todo o nada, incluso ante concurrencia o fallo. El evento enlaza la decisión y
cada revisión; la cita nueva es una intención separada.

Agenda necesita `precautionary_hearing`, orden y cursor estables con familia,
identidad y revisión, además de detalle autorizado propio. Una decisión sobre
inicial reutiliza su fila; no agrega otra cita ni filas de medidas. Las alertas
conservan el origen de la cita y no infieren incumplimientos o términos.

La interfaz debe conservar borradores solo para la misma identidad al reingresar,
retirar aprobación y reautorizar. Respuestas inciertas exigen conciliación exacta
antes de un reintento expreso; sin reenvío automático. Cambio de usuario, logout
o expediente invalida respuestas tardías y no recupera borradores ajenos.

Restauración exige migraciones nuevas, inventario estricto y decodificadores
acotados por versión. Conserva convocatorias, decisiones, revisiones, enlaces,
correcciones, recibos, autores, orígenes y auditoría; rechaza huérfanos, ambigüedad,
versiones desconocidas y discrepancias. No sustituye fuentes por cabezas actuales
ni recalcula efectos. Los canones históricos mantienen sus bytes y significado.

## Evidencia necesaria para cerrar el flujo

1. TDD de valores/canones y preservación de vectores históricos; casos positivos
   y negativos de alcance, precisión, propósitos y orden estable.
2. Aplicación y PostgreSQL: autorización vigente, aislamiento entre expedientes,
   cambios de contexto, objetivos obsoletos, rollback de grupos y replay concurrente.
3. HTTP antes/después de reinicio y restauración: mismos orígenes, lecturas y
   recibos; sin duplicados, auditoría separada o sustitución parcial.
4. Qadra en escritorio/móvil: programar, declarar, revisar, rectificar, consultar
   historia y recuperar respuesta perdida/nueva sesión; inicial sin cita duplicada.
5. Documentación de API, operación, alcance e implementación académica consistente
   con resultados reproducidos; [informe de verificación](verification-report.md)
   con evidencia fresca separada de la histórica y comprobaciones de cierre.

Estos criterios no declaran implementado un corpus completo de términos, firma
judicial, supervisión oficial o adjudicación jurídica. El
[flujo de plazos derivados](hearing-derived-deadlines.md) conserva su contrato
ordinario; admitir una fuente cautelar requerirá su propio contrato exacto.
