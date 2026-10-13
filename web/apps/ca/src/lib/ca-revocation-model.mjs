import { validId } from './ca-operations-values.mjs';

const initial = () => ({ phase: 'idle', certificate: null, reason: '', operation_id: null,
  receipt: null, busy: false, error: '' });
const hash = (value) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const serial = (value) => typeof value === 'string' && /^[1-9A-F][0-9A-F]{0,39}$/.test(value);
const reasonValue = (value) => typeof value === 'string' && value.trim().length > 0
  && new TextEncoder().encode(value).length <= 500 && !/[\x00-\x1f\x7f]/.test(value);

export function canRevoke(value) {
  return Boolean(value && ['partner', 'document-signing'].includes(value.purpose)
    && ['issued', 'expired'].includes(value.local_status) && value.published_status !== 'revoked'
    && serial(value.serial) && hash(value.fingerprint) && hash(value.root_fingerprint));
}

function receiptValue(value, request) {
  if (!value || ['operation_id', 'serial', 'fingerprint', 'reason']
    .some((name) => value[name] !== request[name]) || !validId(value.operator_id)
    || !Number.isSafeInteger(value.recorded_at) || value.recorded_at <= 0
    || value.local_status !== 'revoked' || value.published_status !== 'pending') {
    throw new Error('Invalid revocation receipt');
  }
  return Object.fromEntries(['operation_id', 'recorded_at', 'operator_id', 'serial',
    'fingerprint', 'reason', 'local_status', 'published_status'].map((name) => [name, value[name]]));
}

export function createCaRevocation({ oncommand, onState = () => {}, uuid = () => crypto.randomUUID() }) {
  let state = initial();
  let request = null;
  let resumePayload = null;
  let generation = 0;
  let disposed = false;
  const snapshot = () => structuredClone(state);
  const emit = () => onState(snapshot());
  const current = (value) => !disposed && value === generation;

  function choose(certificate) {
    if (disposed || state.busy || !['idle', 'review', 'completed'].includes(state.phase)
      || !canRevoke(certificate)) return;
    generation++;
    request = null;
    resumePayload = null;
    const publicIdentity = Object.fromEntries(['serial', 'fingerprint', 'root_fingerprint',
      'subject', 'purpose', 'local_status', 'published_status'].map((name) => [name, certificate[name]]));
    state = { ...initial(), phase: 'review', certificate: publicIdentity };
    emit();
  }

  function setReason(value) {
    if (disposed || state.phase !== 'review' || state.busy) return;
    state.reason = value;
    state.error = '';
    emit();
  }

  async function send() {
    const expected = ++generation;
    state.phase = 'submitting';
    state.busy = true;
    state.error = '';
    emit();
    try {
      const result = await oncommand({ ...(resumePayload || request) });
      if (!current(expected)) return;
      state.receipt = receiptValue(result, request);
      state.phase = 'completed';
    } catch {
      if (!current(expected)) return;
      state.phase = 'uncertain';
      state.error = 'No se pudo confirmar la revocaci\u00f3n. Retoma esta misma operaci\u00f3n con sus datos originales.';
    } finally {
      if (current(expected)) { state.busy = false; emit(); }
    }
  }

  async function confirm(identity) {
    if (disposed || state.busy || state.phase !== 'review') return;
    if (!identity || identity.serial !== state.certificate.serial
      || identity.fingerprint !== state.certificate.fingerprint || !reasonValue(state.reason)) {
      state.error = 'Confirma la serie y la huella, e indica un motivo de hasta 500 bytes sin saltos de l\u00ednea.';
      emit();
      return;
    }
    let operationId;
    try {
      operationId = uuid();
      if (!validId(operationId)) throw new Error('Invalid operation identity');
    } catch {
      state.error = 'No se pudo preparar el identificador de la revocaci\u00f3n.';
      emit();
      return;
    }
    state.operation_id = operationId;
    resumePayload = null;
    request = { action: 'revoke', operation_id: operationId,
      serial: state.certificate.serial, fingerprint: state.certificate.fingerprint,
      reason: state.reason, expected_root_fingerprint: state.certificate.root_fingerprint };
    await send();
  }

  return { snapshot, choose, setReason, confirm,
    async resumeOperation(record) {
      if (disposed || state.busy || record?.kind !== 'revoke' || !validId(record.operation_id)
        || !serial(record.serial) || !hash(record.fingerprint) || !reasonValue(record.reason)) return;
      if (state.operation_id && state.phase !== 'completed') {
        if (state.operation_id === record.operation_id) await send();
        return;
      }
      request = { operation_id: record.operation_id, serial: record.serial,
        fingerprint: record.fingerprint, reason: record.reason };
      resumePayload = { action: 'resume-revoke', operation_id: record.operation_id };
      state = { ...initial(), operation_id: record.operation_id, reason: record.reason,
        certificate: { serial: record.serial, fingerprint: record.fingerprint } };
      await send();
    },
    async resume() {
      if (!disposed && !state.busy && state.phase === 'uncertain' && request) await send();
    },
    cancel() {
      if (disposed || state.busy || !['review', 'completed'].includes(state.phase)) return;
      generation++; request = resumePayload = null; state = initial(); emit();
    },
    dispose() { disposed = true; generation++; request = resumePayload = null; state = initial(); emit(); },
  };
}
