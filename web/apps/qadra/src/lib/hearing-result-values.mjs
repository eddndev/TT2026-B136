import { caseText } from './case-administration.mjs';
import { hearingResultTime, hearingResultTimeDraft } from './hearing-result-time.mjs';

const maximumRevision = 4294967295;
function invalid(message) {
  throw new Error(message);
}
function uuid(value, label) {
  if (typeof value !== 'string' || !/^[a-f\d]{8}(-[a-f\d]{4}){3}-[a-f\d]{12}$/i.test(value))
    invalid(`Consulta la referencia exacta de ${label}.`);
  return value.toLowerCase();
}
function revision(value) {
  if (!Number.isInteger(value) || value < 1 || value > maximumRevision)
    invalid('Consulta una revisi\u00f3n exacta v\u00e1lida.');
  return value;
}
function text(raw, label, limit, required = false, multiline = false) {
  if (typeof raw === 'string' && /[\ud800-\udfff]/u.test(raw))
    invalid(`${label}: revisa los caracteres del texto.`);
  return caseText(raw, { key: label, label, limit, required, multiline });
}
function exactSupport(value) {
  if (value == null) return null;
  if (typeof value.digest !== 'string' || !/^[a-f\d]{64}$/i.test(value.digest))
    invalid('Consulta la huella exacta del soporte.');
  return {
    document_id: uuid(value.document_id, 'soporte'),
    version: revision(value.version),
    digest: value.digest.toLowerCase(),
  };
}
function uniqueList(raw, limit, key, label, convert) {
  if (!Array.isArray(raw) || raw.length > limit) invalid(`Agrega hasta ${limit} ${label}.`);
  const result = raw.map(convert);
  if (new Set(result.map((row) => row[key])).size !== result.length)
    invalid(`Hay ${label} con la misma referencia. Revisa la selecci\u00f3n.`);
  return result;
}
export function hearingResultDraft(record) {
  const values = record?.values;
  return {
    occurrence: values?.occurrence || '',
    extent: values?.extent || '',
    time: hearingResultTimeDraft(values?.event_time),
    summary: values?.summary || '',
    attendees: structuredClone(values?.attendees || []),
    agreements: structuredClone(values?.agreements || []),
    provenance: structuredClone(values?.provenance || { kind: '', reference: '', support: null }),
    reason: '',
  };
}
export function hearingResultValues(draft, now = Date.now()) {
  if (!['occurred', 'not_started'].includes(draft.occurrence))
    invalid('Selecciona si se inici\u00f3 el acto informado.');
  if (!['partial', 'concluded', 'unspecified'].includes(draft.extent))
    invalid('Selecciona el alcance declarado del acto.');
  if (draft.occurrence === 'not_started' && draft.extent !== 'unspecified')
    invalid('Un acto no iniciado requiere alcance no especificado.');
  const attendees = uniqueList(draft.attendees, 32, 'participant_id', 'comparecencias', (row) => ({
    participant_id: uuid(row?.participant_id, 'participante'),
    revision: revision(row?.revision),
    capacity: text(row?.capacity, 'Calidad en esta sesi\u00f3n', 100, true),
    observation: text(row?.observation, 'Observaci\u00f3n', 500, false, true),
  }));
  attendees.sort((a, b) =>
    a.participant_id < b.participant_id ? -1 : a.participant_id > b.participant_id ? 1 : 0,
  );
  const agreements = uniqueList(draft.agreements, 16, 'id', 'acuerdos', (row) => ({
    id: uuid(row?.id, 'acuerdo'),
    text: text(row?.text, 'Texto del acuerdo', 1000, true, true),
  }));
  const source = draft.provenance;
  if (!['operator_note', 'oral_reference', 'written_record'].includes(source?.kind))
    invalid('Selecciona la procedencia del relato.');
  return {
    occurrence: draft.occurrence,
    extent: draft.extent,
    event_time: hearingResultTime(draft.time, now),
    summary: text(draft.summary, 'Relato del operador', 1000, true, true),
    attendees,
    agreements,
    provenance: {
      kind: source.kind,
      reference: text(
        source.reference,
        'Localizador de la fuente',
        200,
        source.kind !== 'operator_note',
      ),
      support: exactSupport(source.support),
    },
  };
}
export function hearingResultCommand(draft, options) {
  const { action, base, now } = options;
  const operation_id = uuid(options.operationId, 'operaci\u00f3n');
  const hearing_id = uuid(options.hearingId, 'audiencia');
  const result_id = uuid(options.resultId, 'registro');
  let change;
  if (action === 'record') {
    if (base) invalid('El alta requiere un registro nuevo.');
    const source = options.continuation;
    const continuation =
      source == null
        ? null
        : {
            result_id: uuid(source.result_id, 'antecedente'),
            revision: revision(source.revision),
          };
    if (continuation?.result_id === result_id)
      invalid('El antecedente debe pertenecer a otro registro, no al propio.');
    change = {
      action,
      expected_revision: 0,
      anchor_revision: revision(options.anchorRevision),
      continuation,
      values: hearingResultValues(draft, now),
    };
  } else {
    if (!['correct', 'withdraw'].includes(action) || !base)
      invalid('Consulta el registro antes de rectificarlo o retirarlo.');
    if (
      uuid(base.id, 'registro') !== result_id ||
      uuid(base.hearing_id, 'audiencia') !== hearing_id
    )
      invalid('La base consultada no corresponde al registro seleccionado.');
    if (base.status !== 'recorded') invalid('El registro retirado no admite cambios.');
    const expected_revision = revision(base.revision);
    if (expected_revision === maximumRevision)
      invalid('Se agotaron las revisiones de este registro.');
    change = { action, expected_revision };
    if (action === 'correct') change.values = hearingResultValues(draft, now);
    change.reason = text(draft.reason, 'Motivo', 1000, true, true);
  }
  return { operation_id, hearing_id, result_id, change };
}
