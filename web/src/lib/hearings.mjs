import { caseText } from './case-administration.mjs';
import { hearingInstant, hearingTimeParts } from './hearing-time.mjs';
export const hearingKinds = {
  initial: { label: 'Inicial', stage: 'investigation' },
  intermediate: { label: 'Intermedia', stage: 'intermediate' },
  oral_trial: { label: 'Debate de juicio', stage: 'trial' },
  sentencing: {
    label: 'Individualizaci\u00f3n de sanciones y reparaci\u00f3n del da\u00f1o',
    stage: 'trial',
  },
};
export const hearingModalities = { in_person: 'Presencial', videoconference: 'Videoconferencia' };
export const hearingStatus = { scheduled: 'Programada', cancelled: 'Cancelada' };
export const canHearings = (role, action) =>
  action === 'read'
    ? ['owner', 'litigator', 'paralegal'].includes(role)
    : action === 'manage' && ['owner', 'litigator'].includes(role);
export const hearingDenied = (error) =>
  error.status === 403 || (error.status === 404 && error.code === 'case_not_found');
export const hearingUncertain = (error) => !error.status || error.status >= 500;
const text = (raw, label, limit, required = false, multiline = false) =>
  caseText(raw, { key: label, label, limit, required, multiline });
export function hearingDraft(record) {
  const values = record?.values;
  return {
    kind: values?.kind || 'initial',
    time: values ? hearingTimeParts(values.scheduled_at) : { date: '', time: '', offset: '' },
    modality: values?.modality || 'in_person',
    venue: values?.venue || '',
    note: values?.note || '',
    participants: structuredClone(values?.participants || []),
    statement: values?.conviction_basis?.statement || '',
    support: values?.conviction_basis
      ? {
          id: values.conviction_basis.support.document_id,
          version: values.conviction_basis.support.version,
          digest: values.conviction_basis.support.digest,
          name: record.support?.name,
        }
      : null,
    reason: '',
  };
}
export function hearingCommand(draft, context, base, action, operationId, hearingId) {
  const expected_revision = base?.revision || 0;
  const change = { action, expected_revision };
  if (base?.status === 'cancelled')
    throw new Error('La audiencia est\u00e1 cancelada; no admite cambios.');
  if (action === 'cancel') {
    if (!base) throw new Error('Consulta la audiencia antes de cancelarla.');
    change.reason = text(draft.reason, 'Motivo', 1000, true, true);
  } else {
    if (!['schedule', 'replace'].includes(action) || (action === 'replace' && !base))
      throw new Error('La operaci\u00f3n de audiencia no es v\u00e1lida.');
    if (base && draft.kind !== base.values.kind)
      throw new Error('El tipo de audiencia no se puede cambiar.');
    if (!context?.profile_complete || !context.stage_revision)
      throw new Error('Completa la ficha penal y registra la etapa del expediente.');
    if (context.administrative_status === 'closed')
      throw new Error('El expediente est\u00e1 cerrado administrativamente.');
    if (hearingKinds[draft.kind]?.stage !== context.stage)
      throw new Error('El tipo de audiencia no corresponde a la etapa consultada.');
    if (!hearingModalities[draft.modality])
      throw new Error('Selecciona una modalidad de audiencia.');
    const participants = structuredClone(draft.participants);
    if (
      !Array.isArray(participants) ||
      participants.length > 32 ||
      participants.some(
        (row) =>
          !row.participant_id ||
          !Number.isInteger(row.revision) ||
          row.revision < 1 ||
          row.revision > 4294967295,
      ) ||
      new Set(participants.map((row) => row.participant_id)).size !== participants.length
    )
      throw new Error(
        'Selecciona hasta 32 fichas de participantes distintos con sus revisiones exactas.',
      );
    participants.sort((a, b) => a.participant_id.localeCompare(b.participant_id));
    let conviction_basis = null;
    if (draft.kind === 'sentencing') {
      const statement = text(draft.statement, 'Antecedente de condena', 1000, true, true);
      const support = draft.support;
      if (
        !support?.id ||
        !Number.isInteger(support.version) ||
        support.version < 1 ||
        support.version > 4294967295 ||
        !/^[a-f0-9]{64}$/.test(support.digest)
      )
        throw new Error(
          'Selecciona y consulta la versi\u00f3n exacta del soporte del antecedente.',
        );
      conviction_basis = {
        statement,
        support: { document_id: support.id, version: support.version, digest: support.digest },
      };
    }
    Object.assign(change, {
      expected_case_revision: context.case_revision,
      expected_stage_revision: context.stage_revision,
      values: {
        kind: draft.kind,
        scheduled_at: hearingInstant(draft.time),
        modality: draft.modality,
        venue: text(draft.venue, 'Sede o conexi\u00f3n', 500, true),
        note: text(draft.note, 'Nota', 1000, false, true),
        participants,
        conviction_basis,
      },
    });
    if (action === 'replace') change.reason = text(draft.reason, 'Motivo', 1000, true, true);
  }
  return { operation_id: operationId, hearing_id: hearingId, change };
}
export function hearingTimeLabel(value) {
  const parts = hearingTimeParts(value);
  return `${parts.date} / ${parts.time} / UTC${parts.offset}`;
}
