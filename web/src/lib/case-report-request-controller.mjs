import {
  initialReportFilters,
  reportFilters,
  reportScope,
  reportFailure,
  canReports,
} from './case-reports-presentation.mjs';
import { createReportRequestDraft, freshReportRequest } from './case-report-request-draft.mjs';

export function createReportRequestController({
  api,
  user,
  session,
  capture,
  onstate,
  onstart,
  onrequested,
  ondenied,
}) {
  const scoped = api.reports();
  let alive = true,
    pickerRevision = 0;
  let state = {
    draft: initialReportFilters(),
    pending: null,
    busy: false,
    pickerBusy: false,
    pickerError: '',
    pickerMore: false,
    pickerNext: null,
    lawyers: [],
    error: '',
    blocked: false,
    contextBusy: false,
    saved: false,
    denied: false,
    dirty: false,
  };
  const recovery = session
    ? createReportRequestDraft({
        session,
        user,
        capture: () => ({
          draft: state.blocked || state.saved ? state.draft : capture(),
          pending: state.pending,
          role: user.role,
        }),
      })
    : null;
  const admitted = () => alive && !state.denied && (!recovery || recovery.admitted());
  function publish() {
    if (alive) onstate({ ...state });
  }
  function unavailable() {
    return [state.draft.assigned, state.pending?.filters.assigned_litigator].some(
      (id) => id && !state.lawyers.some((row) => row.user_id === id),
    );
  }
  function deny(failure, notify = true) {
    recovery?.close();
    pickerRevision++;
    state = {
      ...state,
      draft: initialReportFilters(),
      pending: null,
      saved: false,
      dirty: false,
      denied: true,
      blocked: true,
      lawyers: [],
      pickerMore: false,
      pickerBusy: false,
      contextBusy: false,
      busy: false,
      error: notify ? reportFailure(failure) : '',
    };
    publish();
    if (notify) ondenied(failure);
  }
  function failure(error, picker = false) {
    if ([403, 404].includes(error.status)) return deny(error);
    state[picker ? 'pickerError' : 'error'] = reportFailure(error);
  }
  async function loadPicker(append = false) {
    if (
      !admitted() ||
      state.pickerBusy ||
      state.busy ||
      state.contextBusy ||
      (append && !state.pickerMore)
    )
      return;
    const revision = ++pickerRevision;
    state.pickerBusy = true;
    state.pickerError = '';
    if (!append) {
      state.lawyers = [];
      state.pickerMore = false;
      state.pickerNext = null;
    }
    publish();
    try {
      const page = await scoped.litigators({
        limit: 20,
        ...(append ? { after_id: state.pickerNext } : {}),
      });
      if (!admitted() || revision !== pickerRevision) return;
      if (page.scope !== reportScope(user.role))
        throw new Error('El alcance del selector no corresponde a tu acceso.');
      state.lawyers = append ? [...state.lawyers, ...page.litigators] : page.litigators;
      state.pickerMore = page.has_more;
      state.pickerNext = page.next_after_id;
      if (!state.saved && unavailable()) {
        state.blocked = true;
        state.error =
          'El litigante seleccionado no esta en la consulta actual. Vuelve a consultar el contexto.';
      }
    } catch (error) {
      if (admitted() && revision === pickerRevision) failure(error, true);
    } finally {
      if (alive && revision === pickerRevision) {
        state.pickerBusy = false;
        publish();
      }
    }
  }
  async function context() {
    if (!admitted() || state.contextBusy || state.busy || state.pickerBusy) return;
    if (!state.blocked && !state.saved) state.draft = capture();
    state.blocked = state.contextBusy = true;
    state.error = state.pickerError = '';
    pickerRevision++;
    publish();
    const fresh = () => freshReportRequest(api, scoped, user, admitted);
    try {
      if (state.saved && recovery) {
        const result = await recovery.restore(fresh, (value, lawyers) => {
          state.draft = value.draft;
          state.pending = value.pending;
          state.lawyers = lawyers;
          state.saved = false;
          state.dirty = true;
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar la solicitud pendiente.');
      } else {
        const lawyers = await fresh();
        if (!lawyers || !admitted()) return;
        state.lawyers = lawyers;
      }
      state.pickerMore = false;
      state.pickerNext = null;
      state.blocked = unavailable();
      if (state.blocked)
        state.error =
          'El litigante de la solicitud ya no esta disponible. Conservamos su seleccion para consultar de nuevo.';
    } catch (error) {
      if (admitted()) failure(error);
    } finally {
      if (alive) {
        state.contextBusy = false;
        publish();
      }
    }
  }
  function edit() {
    if (!admitted() || state.blocked || state.saved || state.busy) return;
    state.draft = capture();
    recovery?.register();
    state.dirty = true;
    publish();
  }
  function discard() {
    if (!admitted() || state.busy || state.contextBusy) return;
    recovery?.close();
    state = {
      ...state,
      draft: initialReportFilters(),
      pending: null,
      saved: false,
      dirty: false,
      blocked: false,
      error: '',
      pickerError: '',
    };
    publish();
  }
  async function request(retry = false) {
    if (
      !admitted() ||
      state.busy ||
      state.pickerBusy ||
      state.contextBusy ||
      state.pickerError ||
      state.blocked ||
      state.saved
    )
      return;
    let command;
    try {
      state.draft = capture();
      if (retry) command = state.pending;
      else {
        const filters = reportFilters(state.draft, state.lawyers);
        command =
          state.pending && JSON.stringify(state.pending.filters) === JSON.stringify(filters)
            ? state.pending
            : { operation_id: crypto.randomUUID(), filters };
      }
      if (!command) return;
      recovery?.register();
      command = structuredClone(command);
    } catch (error) {
      state.error = reportFailure(error);
      publish();
      return;
    }
    state.pending = command;
    state.dirty = true;
    state.busy = true;
    state.error = '';
    publish();
    onstart();
    let confirmed = false;
    try {
      const value = await scoped.request(command);
      if (!admitted()) return;
      if (value.scope !== reportScope(user.role))
        throw new Error('El alcance del informe no corresponde a tu acceso actual.');
      recovery?.close();
      state.pending = null;
      state.saved = false;
      state.dirty = false;
      confirmed = true;
      onrequested(value);
    } catch (error) {
      if (admitted()) {
        if (confirmed)
          state.error = 'La solicitud se guardo. Consulta los informes sin repetir el envio.';
        else {
          failure(error);
          if (error.status && error.status < 500) state.pending = null;
        }
      }
    } finally {
      if (alive) {
        state.busy = false;
        publish();
      }
    }
  }
  return {
    state: () => ({ ...state }),
    edit,
    discard,
    request,
    context,
    loadPicker,
    deny: (error) => deny(error, false),
    initialize() {
      if (!canReports(user.role)) return deny({ status: 403 });
      state.saved = recovery?.pending() ?? false;
      publish();
      loadPicker();
    },
    dispose() {
      alive = false;
      pickerRevision++;
      recovery?.dispose();
      scoped.dispose();
    },
  };
}
