import { validId } from './ca-operations-values.mjs';

const initial = () => ({ phase: 'idle', status: null, operation_id: null,
  expected_revision: null, receipt: null, busy: false, error: '' });
const hash = (value) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const revision = (value) => Number.isSafeInteger(value) && value > 0 && value <= 0xffffffff;
const number = (value) => Number.isSafeInteger(value) && value >= 0;
const pick = (value, names) => Object.fromEntries(names.map((name) => [name, value[name]]));

function statusValue(value) {
  if (!value || !revision(value.revision) || !hash(value.root_fingerprint) || !hash(value.crl_digest)
    || !number(value.crl_number) || !['valid_from', 'valid_until', 'checked_at']
      .every((name) => Number.isSafeInteger(value[name])) || value.valid_from > value.valid_until
    || typeof value.valid_now !== 'boolean' || typeof value.maintenance_pending !== 'boolean'
    || !Array.isArray(value.revoked_serials)) throw new Error('Invalid public CRL status');
  return { ...pick(value, ['revision', 'root_fingerprint', 'crl_digest', 'crl_number',
    'valid_from', 'valid_until', 'checked_at', 'valid_now', 'maintenance_pending']),
    revoked_count: value.revoked_serials.length };
}

function receiptValue(value, expected) {
  if (!value || value.operation_id !== expected.operation_id || !validId(value.operator_id)
    || !Number.isSafeInteger(value.recorded_at) || value.recorded_at <= 0
    || value.expected_revision !== expected.expected_revision
    || value.revision !== expected.expected_revision + 1 || !revision(value.revision)
    || !number(value.crl_number) || !hash(value.root_fingerprint) || value.state !== 'published'
    || (expected.root_fingerprint && value.root_fingerprint !== expected.root_fingerprint)) {
    throw new Error('Invalid publication receipt');
  }
  return pick(value, ['operation_id', 'operator_id', 'recorded_at', 'expected_revision',
    'revision', 'crl_number', 'root_fingerprint', 'state']);
}

export function createCaPublication({ oncommand, onState = () => {}, uuid = () => crypto.randomUUID() }) {
  let state = initial();
  let intent = null;
  let generation = 0;
  let disposed = false;
  const snapshot = () => structuredClone(state);
  const emit = () => onState(snapshot());
  const available = () => !disposed && !state.busy;
  const current = (value) => !disposed && value === generation;

  async function refresh() {
    if (!available()) return;
    const expected = ++generation;
    state.status = null;
    state.busy = true;
    state.error = '';
    emit();
    try {
      const result = await oncommand({ action: 'status' });
      if (current(expected)) state.status = statusValue(result);
    } catch {
      if (current(expected)) state.error = 'No se pudo consultar la CRL registrada en Qadra. Actualiza antes de publicar.';
    } finally {
      if (current(expected)) { state.busy = false; emit(); }
    }
  }

  async function send(command) {
    const expected = ++generation;
    state.phase = 'publishing';
    state.busy = true;
    state.error = '';
    emit();
    try {
      const result = await oncommand(command);
      if (!current(expected)) return;
      state.receipt = receiptValue(result, intent);
      state.phase = 'completed';
      state.status = null;
    } catch {
      if (!current(expected)) return;
      state.phase = 'uncertain';
      state.error = 'No se pudo confirmar la publicaci\u00f3n. Retoma esta misma operaci\u00f3n; el mantenimiento puede seguir activo.';
    } finally {
      if (current(expected)) { state.busy = false; emit(); }
    }
  }

  async function confirm(value) {
    if (!available() || state.phase !== 'idle' || !state.status || state.operation_id) return;
    if (value?.acknowledge_maintenance !== true || value.expected_revision !== state.status.revision
      || state.status.maintenance_pending) {
      state.error = 'Confirma la revisi\u00f3n actual y el mantenimiento. Si hay una operaci\u00f3n pendiente, ret\u00f3mala desde Historial.';
      emit();
      return;
    }
    let operationId;
    try {
      operationId = uuid();
      if (!validId(operationId)) throw new Error('Invalid operation identity');
    } catch {
      state.error = 'No se pudo preparar el identificador de la publicaci\u00f3n.';
      emit();
      return;
    }
    intent = { operation_id: operationId, expected_revision: state.status.revision,
      root_fingerprint: state.status.root_fingerprint };
    state.operation_id = operationId;
    state.expected_revision = intent.expected_revision;
    await send({ action: 'renew-crl', operation_id: operationId,
      expected_revision: intent.expected_revision, acknowledge_maintenance: true });
  }

  async function resume() {
    if (available() && state.phase === 'uncertain' && intent)
      await send({ action: 'resume-crl', operation_id: intent.operation_id });
  }

  async function resumeOperation(record) {
    if (!available() || record?.kind !== 'publication' || !validId(record.operation_id)
      || !revision(record.expected_revision)) return;
    if (state.operation_id && state.phase !== 'completed') {
      if (state.operation_id === record.operation_id) await resume();
      return;
    }
    intent = { operation_id: record.operation_id, expected_revision: record.expected_revision };
    state = { ...initial(), operation_id: record.operation_id, expected_revision: record.expected_revision };
    await send({ action: 'resume-crl', operation_id: record.operation_id });
  }

  return { snapshot, refresh, confirm, resume, resumeOperation,
    reset() {
      if (!available() || state.phase !== 'completed') return;
      generation++; intent = null; state = initial(); emit();
    },
    dispose() { disposed = true; generation++; intent = null; state = initial(); emit(); },
  };
}
