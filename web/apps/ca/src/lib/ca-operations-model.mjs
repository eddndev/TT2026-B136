import { emptyDraft, emptyPage, validDraft, validId, previewValue,
  receiptValue, certificateValue, pageValue } from './ca-operations-values.mjs';

const initial = () => ({ tab: 'requests', draft: emptyDraft(), preview: null,
  operation_id: null, phase: 'draft', receipt: null, busy: false, error: '',
  inventory: emptyPage(), history: emptyPage() });

export function createCaOperations({ oncommand, onState = () => {}, uuid = () => crypto.randomUUID() }) {
  let state = initial();
  let generation = 0;
  let disposed = false;
  const snapshot = () => structuredClone(state);
  const emit = () => onState(snapshot());
  const current = (value) => !disposed && value === generation;
  const available = () => !disposed && !state.busy;

  function edit(patch) {
    if (disposed || state.operation_id) return;
    if (state.busy && state.phase !== 'previewing') return;
    generation++;
    for (const name of Object.keys(emptyDraft())) {
      if (Object.hasOwn(patch, name)) state.draft[name] = patch[name];
    }
    state.preview = null;
    state.phase = 'draft';
    state.error = '';
    state.busy = false;
    emit();
  }

  async function preview() {
    if (!available() || state.operation_id) return;
    state.preview = null;
    if (!validDraft(state.draft)) {
      state.phase = 'draft';
      state.error = 'Revisa el CSR p\u00fablico y completa titular, referencia y criterio (hasta 500 bytes por campo).';
      emit();
      return;
    }
    const request = { ...state.draft };
    const expected = ++generation;
    state.phase = 'previewing';
    state.busy = true;
    state.error = '';
    emit();
    try {
      const result = await oncommand({ action: 'preview', csr_pem: request.csr_pem, purpose: request.purpose });
      if (!current(expected)) return;
      state.preview = previewValue(result, request.purpose);
      state.phase = 'review';
    } catch {
      if (!current(expected)) return;
      state.phase = 'draft';
      state.error = 'No se pudo validar esta solicitud. Revisa la firma del CSR, su nombre y que el pa\u00eds y la organizaci\u00f3n coincidan con la CA.';
    } finally {
      if (current(expected)) { state.busy = false; emit(); }
    }
  }

  async function signing(payload, request = null) {
    const expected = ++generation;
    state.operation_id = payload.operation_id;
    state.phase = 'issuing';
    state.busy = true;
    state.error = '';
    state.receipt = null;
    emit();
    try {
      const result = await oncommand(payload);
      if (!current(expected)) return;
      state.receipt = receiptValue(result, payload.operation_id, request);
      state.phase = state.receipt.state === 'rejected' ? 'rejected' : 'completed';
      state.inventory.loaded = state.history.loaded = false;
    } catch {
      if (!current(expected)) return;
      state.phase = 'uncertain';
      state.error = 'No se pudo confirmar el resultado. Retoma esta misma operaci\u00f3n para consultar o completar su emisi\u00f3n.';
    } finally {
      if (current(expected)) { state.busy = false; emit(); }
    }
  }

  async function confirm() {
    if (!available() || state.phase !== 'review' || !state.preview || state.operation_id) return;
    let operationId;
    try {
      operationId = uuid();
      if (!validId(operationId)) throw new Error('Invalid operation identity');
    } catch {
      state.error = 'No se pudo preparar el identificador de la operaci\u00f3n.';
      emit();
      return;
    }
    const request = { ...state.draft };
    await signing({ action: 'issue', operation_id: operationId, ...request,
      expected_root_fingerprint: state.preview.root_fingerprint }, request);
  }

  async function resume(operationId) {
    if (!available() || !validId(operationId)) return;
    if (state.operation_id && state.operation_id !== operationId
      && !['completed', 'rejected'].includes(state.phase)) return;
    if (state.operation_id !== operationId
      && !state.history.items.some((item) => item.operation_id === operationId && item.kind === 'issue')) return;
    state.tab = 'requests';
    const request = state.operation_id === operationId && state.preview ? { ...state.draft } : null;
    if (!request) { state.draft = emptyDraft(); state.preview = null; }
    await signing({ action: 'resume-issue', operation_id: operationId }, request);
  }

  async function loadPage(kind, offset = 0) {
    if (!available() || !Number.isSafeInteger(offset) || offset < 0 || offset > 0xffffffff) return;
    const expected = ++generation;
    state.busy = true;
    state.error = '';
    emit();
    try {
      const result = await oncommand({ action: kind, offset, limit: 20 });
      if (current(expected)) state[kind] = pageValue(result, kind, offset);
    } catch {
      if (current(expected)) state.error = 'No se pudo actualizar la consulta. Puedes volver a intentarlo.';
    } finally {
      if (current(expected)) { state.busy = false; emit(); }
    }
  }

  async function download(identity) {
    if (!available()) return null;
    const expected = ++generation;
    state.busy = true;
    state.error = '';
    emit();
    try {
      const result = await oncommand({ action: 'certificate', serial: identity.serial,
        fingerprint: identity.fingerprint });
      return current(expected) ? certificateValue(result, identity) : null;
    } catch {
      if (current(expected)) state.error = 'No se pudo obtener el certificado exacto. Actualiza el inventario y revisa su identidad.';
      return null;
    } finally {
      if (current(expected)) { state.busy = false; emit(); }
    }
  }

  function startNew() {
    if (!available() || !['completed', 'rejected'].includes(state.phase)) return;
    generation++;
    state = { ...state, draft: emptyDraft(), preview: null, operation_id: null,
      receipt: null, phase: 'draft', error: '', tab: 'requests' };
    emit();
  }

  return { snapshot, edit, preview, confirm, resume, download, startNew,
    loadInventory: (offset = 0) => loadPage('inventory', offset),
    loadHistory: (offset = 0) => loadPage('history', offset),
    setTab(tab) {
      if (!disposed && ['requests', 'inventory', 'history', 'publication'].includes(tab)) {
        state.tab = tab;
        emit();
      }
    },
    dispose() { disposed = true; generation++; state = initial(); emit(); },
  };
}
