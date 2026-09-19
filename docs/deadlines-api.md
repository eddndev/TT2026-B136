# API de plazos registrados por expediente

Estado: el servicio humano y el adaptador HTTP escriben seguimiento V2 y
leen historia V1/V2. La vigencia actual se consulta por separado del cálculo
capturado. Sus comprobaciones focales y la aceptación integrada se identifican
por revisión en [el informe de verificación](verification-report.md); una campaña
histórica de V1 no acredita el contrato actual. La integración Qadra y la
composición del trabajador en `serve` tienen su propio estado de entrega.

El contrato utiliza [perfiles versionados](deadline-profiles-api.md). El resultado
conservado evalúa declaraciones explícitas; no determina por sí mismo la validez
de un acto ni sustituye su calificación jurídica.

## Frontera de versión

Las rutas conservan `/api/v1`; V1/V2 identifica el formato del recibo histórico.
Los nuevos comandos humanos preparan V2 con políticas explícitas y autor tomado
de la sesión autenticada. No reciben autor técnico, causa, observaciones ni
estados de revisión proporcionados por el cliente. Las revisiones técnicas se
consultan junto a las humanas y conservan su procedencia. Véase el contrato
completo de [seguimiento y vigencia](deadline-tracking-api.md).

## Colección y autorización

Base: `/api/v1/cases/{case_id}/deadlines`.

Owner y Litigator registran, corrigen, declaran atención y retiran. Owner,
Litigator y Paralegal consultan; las asignaciones vigentes al expediente se
comprueban para los roles que las necesitan. Client queda denegado. El expediente
cerrado conserva lecturas y bloquea escrituras. Asignar responsable no concede
acceso al expediente. El servidor vuelve a comprobar identidad, autorización,
responsable, administración y fuentes al confirmar.

| Método y sufijo | Operación |
| --- | --- |
| GET base | Lista ligera de cabezas actuales. |
| GET `/responsibles` | Selector paginado de responsables actualmente elegibles. |
| GET `/{id}` | Detalle actual con comprobación autorizada de vigencia. |
| GET `/{id}/revisions/{revision}` | Detalle histórico exacto. |
| GET `/{id}/history` | Revisiones ligeras descendentes. |
| POST `/prepare` | Prepara un comando sin reservar ni escribir. |
| POST base | Confirma registro. |
| PUT `/{id}` | Confirma corrección de definición completa. |
| POST `/{id}/attention` | Confirma una declaración de atención. |
| POST `/{id}/retirement` | Confirma retiro terminal. |

Lista: `limit`1..100, por defecto20; `after_id` UUID opcional; `status` es
`active` por defecto, `all` o `retired`. Historia: `limit`1..20, por defecto10,
y `before_revision` positivo opcional. No se admiten consultas en detalle,
revisión exacta ni mutaciones. Parámetros repetidos o desconocidos producen400.
UUID cero conserva su identidad y nunca significa ausencia.

## Selector de responsables

`GET /api/v1/cases/{case_id}/deadlines/responsibles` recibe `limit` entre 1 y
100 (por defecto 20) y `after_id` UUID opcional. Los parámetros desconocidos,
repetidos, vacíos o inválidos producen 400; el literal `null` no es un UUID.
Omitir `after_id` inicia la consulta y el UUID cero conserva su valor. La página
ordena por UUID ascendente y excluye el cursor; el cursor no necesita identificar
una cuenta existente y no concede acceso a otro expediente.

```json
{
  "case_id": "00000000-0000-0000-0000-000000000010",
  "responsibles": [
    {
      "id": "00000000-0000-0000-0000-000000000004",
      "email": "responsable@example.test",
      "role": "paralegal"
    }
  ],
  "has_more": false,
  "next_after_id": null
}
```

La respuesta sólo contiene UUID, correo y rol de cuentas activas elegibles:
Owner puede atender cualquier expediente, aun sin membresía; Litigator y
Paralegal requieren asignación vigente al expediente solicitado. Client nunca
aparece. Un Owner asignado aparece una sola vez. La selección no expone
contraseñas, secretos MFA, códigos de recuperación ni datos de otros roles.
`has_more` verdadero indica una página completa y `next_after_id` es el último
UUID devuelto; cuando no quedan candidatos, el cursor es `null`.

Owner, Litigator y Paralegal pueden consultar según los permisos de lectura de
plazos. La cuenta lectora debe seguir activa y, para los dos últimos roles,
asignada al expediente. La falta de sesión o su revocación produce 401, Client
recibe 403 y un expediente inexistente o ajeno produce 404. Los expedientes
cerrados siguen siendo legibles. El servidor confirma la auditoría
`deadline.responsibles_read` antes de entregar candidatos; un fallo de auditoría
impide la respuesta de datos. La aplicación comprueba nuevamente la sesión antes
de devolver la página.

El selector representa elegibilidad actual, separada de la identidad histórica
conservada en un plazo. Una selección no reserva la cuenta ni modifica permisos.
Preparar y confirmar un registro o una corrección de definición vuelven a
comprobar actividad, rol y asignación; una revocación posterior al listado puede
rechazar esas operaciones con 409 `deadline_responsible_unavailable`. Declarar
atención o retirar conserva el responsable histórico, aunque ya esté revocado.
Consultar revisiones históricas también conserva esa identidad capturada,
aunque ya no aparezca entre los candidatos actuales.

## Preparación y confirmación

`POST /prepare` recibe un comando:

```json
{
  "operation_id": "00000000-0000-0000-0000-000000000000",
  "deadline_id": "00000000-0000-0000-0000-000000000001",
  "change": {
    "action": "register",
    "expected_revision": 0,
    "tracking": {"profile": "follow", "source": "fixed", "calendar": "undetermined"},
    "definition": {
      "title": "Respuesta declarada",
      "profile": {"id": "00000000-0000-0000-0000-000000000002", "revision": 1},
      "responsible_id": "00000000-0000-0000-0000-000000000004",
      "input": {
        "selection": {
          "case_id": "00000000-0000-0000-0000-000000000010",
          "source": {"kind": "known", "value": {
            "family": "resolution",
            "id": "00000000-0000-0000-0000-000000000003", "revision": 1
          }},
          "qualification": null
        },
        "calendar": null,
        "ordered_quantity": null,
        "qualification": {
          "statement": "Aplicabilidad declarada por la persona operadora",
          "locator": "Resolucion, pagina 1",
          "scope_applies": {"kind": "known", "value": true},
          "unresolved_incident": {"kind": "known", "value": false},
          "conditions": []
        }
      }
    }
  }
}
```

Este ejemplo requiere un perfil, responsable y fuente existentes y autorizados.
Las condiciones vacías no satisfacen condiciones requeridas por un perfil;
producen los bloqueos que correspondan. Las variantes de `change` son:

- `register`: `expected_revision:0`, `definition`, `tracking`.
- `correct`: revisión positiva esperada, `definition`, `reason`, `tracking`.
- `set_attention`: revisión positiva esperada, `attention`, `reason`.
- `retire`: revisión positiva esperada y `reason`, sin definición ni atención.

`tracking` contiene exactamente `profile`, `source` y `calendar`. Cada dependencia
presente exige `fixed` o `follow`; una fuente desconocida o calendario ausente
exige `undetermined`. No se infiere intención de la revisión seleccionada.
Atención y retiro prohíben `tracking`, incluso nulo: heredan la captura previa.

Confirmar utiliza `{command,expected_submission_digest}`. El comando debe
corresponder al expediente, UUID y acción de la ruta. El digest contiene64
caracteres hexadecimales y procede de la preparación. Preparar devuelve200;
las cuatro confirmaciones devuelven201. La preparación no reserva la cabeza,
la operación ni el estado observado.

Cada objeto tiene campos nombrados estrictos: no se aceptan claves desconocidas,
duplicadas ni arreglos posicionales en lugar de objetos. La política de cuerpo es
1MiB para todos los comandos; el exceso produce413 `deadline_body_too_large`.
Se exige exactamente un Content-Type JSON admitido por el lector HTTP compartido.
JSON o Content-Type inválidos producen400 `invalid_json`.

La definición referencia un perfil; no incorpora su corpus. DEVI1 limita las
declaraciones a98,897 bytes canónicos. Seis veces esa cota son593,382 bytes:
1MiB deja margen para claves y sobre JSON aun con escapes de caracteres. Esta
política limita los bytes recibidos; no promete aceptar espacio sintáctico
ilimitado ni cualquier representación equivalente con relleno.

## Definición y declaraciones

`definition` contiene exactamente:

- `title`: texto de1..200 caracteres, normalizado por el dominio.
- `profile:{id,revision}`: revisión exacta del perfil publicado seleccionado.
- `responsible_id`: UUID del responsable explícito.
- `input:{selection,calendar,ordered_quantity,qualification}`.

`calendar` es `null` o `{id,revision}`. `ordered_quantity` es `null` o un u32
positivo. No se sustituye por el máximo del perfil. Los máximos, las unidades,
el inicio, el corte y la aplicabilidad proceden de los datos explícitos y del
perfil; ningún dato se extrae de la narrativa.

`selection` contiene `case_id`, `source` y `qualification`. `source` es una
declaración `{kind:"unknown",reason}` o `{kind:"known",value}`. Sus valores son:

- `{family:"resolution",id,revision}`.
- `{family:"notification",id,revision,resolution:{id,revision}}`.
- `{family:"hearing_result",hearing_id,result_id,revision,agreement_id}`.

`agreement_id` es UUID o `null`; UUID cero sigue siendo un acuerdo concreto.
El padre de una notificación se identifica también por revisión exacta.

`selection.qualification` es `null` o
`{purpose,at,statement,locator}`. `purpose` es `hearing_end` u
`ordered_period_start`. Su tiempo pertenece a la misma fuente exacta. El
requisito del perfil determina si esa declaración es la entrada necesaria.

`input.qualification` describe aplicabilidad y contiene:

- `statement` y `locator`: declaración y localización explícitas.
- `scope_applies`: declaración de booleano.
- `unresolved_incident`: declaración de booleano.
- `conditions`:0..16 objetos `{id,applies,locator}`, con UUID únicos.

La declaración de booleano es `{kind:"known",value:true|false}` o
`{kind:"unknown",reason}`. No se convierte `unknown` en falso. `statement` y
`reason` admiten1..1000 caracteres; `locator`,1..200. El dominio normaliza texto
humano; no cambia identificadores ni precisión temporal.

## Tiempo declarado y atención

Los tiempos declarados conservan esta forma, compartida con
[los hechos procesales](procedural-facts-api.md):

- `{precision:"unknown"}`, sin componentes adicionales.
- `{precision:"date",year,month,day,offset_seconds}`.
- `{precision:"minute",year,month,day,hour,minute,offset_seconds}`.
- `{precision:"second",year,month,day,hour,minute,second,offset_seconds}`.

El desfase es `null` o segundos enteros admitidos por el dominio. Las respuestas
lo incluyen explícitamente en las precisiones conocidas. No se añade medianoche
ni segundos a una fecha o minuto. Las validaciones del tiempo declarado son las
del contrato de hechos; los instantes calculados tienen un formato distinto.

`attention` es `{status:"pending"}` o
`{status:"recorded",occurred_at,statement,locator}`. Registrar una declaración
no acredita presentación válida, atención oportuna ni extinción jurídica del
plazo. La atención y el retiro conservan el cálculo histórico íntegro; corregir
la definición crea una nueva revisión evaluada.

## Respuestas y conservación histórica

El borrador contiene `case_id`, `actor_id`, `command` normalizado,
`result_revision`, `definition`, `calculation`, `responsible`, `attention`,
`status`, `author`, `tracking`, `receipt_version`, `review_digest`,
`capture_digest` y `submission_digest`. No incluye vigencia operativa.

Detalle contiene `id`, `case_id`, `revision`, `definition`, `calculation`,
`responsible`, `attention`, `status`, `reason`, `receipt`, `recorded_at` y
`recorded_by`, `tracking` y `operational`. Autoría lleva `kind:"user"` con
`id,email`, o `kind:"technical"` con `service,policy_version`. Responsable conserva
`{id,email,role}` históricos. `receipt.version` distingue V1/V2 y conserva
predecesor y causa técnica cuando corresponden.
El recibo contiene `operation_id`, `action`, `expected_revision` y los tres
digests del borrador. `review_digest` vincula el contenido confirmado;
`capture_digest` conserva además la administración finalmente capturada.

`calculation` contiene:

- `profile`: `{id,revision,algorithm,title,scope,status,definition_digest,
  submission_digest,href}`. El enlace recupera la revisión exacta, con condiciones,
  referencias y corpus; no apunta a la cabeza actual.
- `material`: `{case_id,administration,source,source_head,calendar,calendar_head}`.
- `result`: salida histórica estructurada descrita abajo.

Cada fuente no nula lleva `case_id`, `reference` de la familia seleccionada,
`values_digest`, `sources_digest` nullable, `submission_digest`, `status` y
`href` exacto. `source` conserva el acuerdo seleccionado; `source_head` conserva
el resultado observado sin seleccionar un acuerdo. En una notificación cada
referencia conserva la revisión del padre que le corresponde. Las cabezas
observadas pueden haber sido corregidas o retiradas después: la lectura histórica
no las sustituye.

Calendario no nulo contiene `{id,revision,values_digest,submission_digest,status,
title,href}`. La administración `unrevised` lleva título, referencia y estado
originales, sin inventar revisión, digest, autor ni fecha. La administración
`recorded` añade `case_id`, `revision`, `values_digest`, `changed_at` y
`changed_by`. No incluye el perfil penal.

Los instantes `recorded_at`, `changed_at`, `due_at` y los instantes del resultado
se proyectan como `{unix_seconds,nanosecond,offset_seconds}`. Conservan desfase y
nanosegundos íntegros, incluso representaciones no admitidas por RFC3339. No son
el objeto de tiempo declarado anterior. La evaluación actual normaliza el
vencimiento completo a UTC; conserva las representaciones originales en los
operandos y la traza histórica.

## Resultado y traza

`result` contiene `{requirement,trigger_outcome,rule,arithmetic,due_at,blocks}`.
`rule`, `arithmetic` y `due_at` pueden ser nulos según lo que pudo evaluarse.
`trigger_outcome` es `{kind:"extracted",at}` o `{kind:"blocked",block}`.
`requirement` usa la misma proyección del requisito del perfil.

Las reglas son `days`, `civil_months` y `elapsed_hours`, con su cantidad y
operandos declarados. La aritmética conserva `{rule,anchor,outcome,trace}`.
`outcome` es `civil_candidate` con `date`, `instant_candidate` con `instant`, o
`blocked` con `block`. Las fechas civiles son `YYYY-MM-DD`.

La traza conserva todas las etapas, en orden:

- `natural_days`: `first_included`, `quantity`, `candidate` nullable.
- `civil_months`: `anchor`, `quantity`, `target_year`, `target_month`,
  `requested_day`, `candidate` nullable.
- `elapsed_hours`: `start`, `quantity`, `candidate` nullable, como instantes.
- `counted_days` y `final_day`: `count` con `first_included`, `quantity`,
  `accumulated`, `outcome` y la traza diaria.

Cada elemento diario conserva `{day,accumulated}`. `day` lleva `date`, `origin`,
`classification`, `explanation`, `source_ids`; origen es nulo o
`weekly_pattern` con `weekday`, o `exception` con `id`. Clasificación pendiente
y fuera de cobertura conservan su distinción.

`blocks` conserva el orden y todas las causas. Las de aplicabilidad son
`scope_unknown`, `scope_rejected`, `incident_unknown`, `unresolved_incident`,
`condition_missing`, `condition_unknown` y `condition_rejected`; las tres últimas
llevan el UUID de condición. Otras son `civil_cutoff_missing` y
`cutoff_outside_coverage` con candidata. Los grupos `rule`, `trigger` y
`arithmetic` incluyen un objeto `block` con `kind` y sus operandos. La API no
convierte enumeraciones en textos Debug ni recalcula un resultado al consultarlo.

## Páginas, conflictos y errores

Lista devuelve `{case_id,deadlines,has_more,next_after_id}`. Cada resumen contiene
`id`, `case_id`, `revision`, `title`, `status`, `responsible`,
`attention_recorded`, `receipt_kind`, `review_state`, `calculation_due_at`,
`calculation_blocked` y `operational`. No incorpora cálculo completo ni corpus.
La fecha utilizable para seguimiento está exclusivamente en `operational.due_at`;
la fecha `calculation_due_at` conserva el resultado histórico.
Historia devuelve `{case_id,id,revisions,has_more,next_before_revision}`; cada
revisión lleva identificación, estado, motivo, recibo, `state_digest`, fecha y
autor, sin definición ni cálculo completo.

Los cursores son nulos sin página siguiente. UUID ascendentes en lista y
revisiones descendentes en historia. Los filtros no amplían autorización.

Errores de plazo:404 `deadline_not_found`;409 `deadline_revision_conflict`,
`deadline_operation_conflict`, `deadline_revision_exhausted`, `deadline_retired`,
`deadline_profile_unavailable`, `deadline_responsible_unavailable` y
`deadline_submission_mismatch`;422 `invalid_deadline` para errores de aplicación.
Los campos sintácticos inválidos y desacuerdos entre ruta y comando producen400.
Sesión, permisos, expediente ausente o cerrado conservan sus códigos compartidos.
Inconsistencia de almacenamiento o respuesta interna produce500 `internal_error`
sin exponer su diagnóstico.

Una respuesta perdida se concilia leyendo la revisión exacta esperada y
comparando UUID de operación, acción, revisión y recibo; no se reenvía
automáticamente el comando. Un conflicto exige consultar de nuevo y preparar
explícitamente lo que se desea confirmar. La actualización automática durable,
las alertas y su entrega no se declaran completadas por estos endpoints.

## Ejemplo de continuidad de la operación

Para el expediente del ejemplo, se prepara con
`POST /api/v1/cases/00000000-0000-0000-0000-000000000010/deadlines/prepare`.
Se revisan el comando normalizado y el resultado recibido. La confirmación envía
ese mismo comando y su `submission_digest` como `expected_submission_digest` al
recurso base. No se calcula un digest en el cliente.

Después se consulta `GET /api/v1/cases/{case_id}/deadlines/{id}/history?limit=10`
y, al elegir una fila, `GET /api/v1/cases/{case_id}/deadlines/{id}/revisions/1`.
Una declaración de atención prepara un nuevo UUID de operación con el mismo
`deadline_id` y este `change`:

```json
{
  "action": "set_attention",
  "expected_revision": 1,
  "attention": {
    "status": "recorded",
    "occurred_at": {"precision": "date", "year": 2026, "month": 1,
                    "day": 9, "offset_seconds": null},
    "statement": "Presentacion declarada por la persona operadora",
    "locator": "Acuse, pagina 1"
  },
  "reason": "Incorporacion de la declaracion de atencion"
}
```

Tras revisar, se confirma mediante `POST /{id}/attention`, con el sobre habitual.
Para retirar esa revisión se prepara otro UUID de operación y
`change:{action:"retire",expected_revision:2,reason:"Retiro administrativo"}`;
se confirma en `POST /{id}/retirement`. Las revisiones previas siguen consultables.
Los UUID y revisiones de estos ejemplos deben corresponder a los datos reales;
no constituyen una regla jurídica ni autorizan automáticamente una operación.
