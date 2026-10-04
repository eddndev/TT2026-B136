import { hearingFormCommand, compatibleHearingKind } from './resource-hearing-editor-values.mjs';

export function createResourceHearingActions({
  read,
  update,
  admitted,
  fresh,
  scoped,
  finish,
  fail,
  saveInputs,
  register,
}) {
  const available = () => {
    const value = read();
    return admitted() && !value.pending && !value.disabled && !value.blocked;
  };
  async function current(writing = false) {
    const context = await fresh();
    if (!context || !admitted()) return null;
    update({ closed: context.closed });
    if (writing && context.closed) throw { status: 409, code: 'case_closed' };
    return context.current;
  }
  async function prepare() {
    const value = read();
    if (!available() || value.mode !== 'draft' || value.closed) return;
    saveInputs();
    update({ busy: true, error: '', prepared: null, last: null, acknowledged: false });
    try {
      const head = await current(true);
      if (!head) return;
      if (head.revision !== value.base.revision || head.status !== 'active')
        throw { status: 409, code: 'resource_activity_resource_revision_conflict' };
      const command = hearingFormCommand(value);
      const prepared = await scoped.prepare(command, value.user);
      if (admitted()) update({ prepared, mode: 'review' });
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      update({ busy: false });
    }
  }
  async function submit() {
    const value = read();
    if (
      !available() ||
      value.mode !== 'review' ||
      !value.prepared ||
      !value.acknowledged ||
      value.closed
    )
      return;
    update({ busy: true, error: '' });
    let sent = false;
    try {
      const head = await current(true);
      if (!head) return;
      if (head.revision !== value.base.revision || head.status !== 'active')
        throw { status: 409, code: 'resource_activity_resource_revision_conflict' };
      const last = structuredClone(value.prepared);
      update({ last, mode: 'uncertain', retryAvailable: false });
      sent = true;
      const result = await scoped.submit(last);
      if (admitted()) await finish(result);
    } catch (failure) {
      if (admitted()) fail(failure, sent);
    } finally {
      update({ busy: false });
    }
  }
  async function check() {
    const value = read();
    if (!available() || value.mode !== 'uncertain' || !value.last) return;
    update({ busy: true, error: '', retryAvailable: false });
    try {
      if (!(await current())) return;
      const result = await scoped.readSubmission(value.last);
      if (!admitted()) return;
      if (result.state === 'confirmed') await finish(result.creation);
      else
        update({
          retryAvailable: true,
          error:
            'El resultado sigue incierto. La revision no esta disponible; esto no confirma que el envio fallo.',
        });
    } catch (failure) {
      if (admitted()) fail(failure, true, true);
    } finally {
      update({ busy: false });
    }
  }
  async function retry() {
    const value = read();
    if (
      !available() ||
      value.mode !== 'uncertain' ||
      !value.last ||
      !value.retryAvailable ||
      value.closed
    )
      return;
    update({ busy: true, error: '', retryAvailable: false });
    try {
      const head = await current(true);
      if (!head) return;
      if (head.status !== 'active')
        throw { status: 409, code: 'resource_activity_resource_archived' };
      const result = await scoped.submit(value.last);
      if (admitted()) await finish(result);
    } catch (failure) {
      if (admitted()) fail(failure, true, true);
    } finally {
      update({ busy: false });
    }
  }
  async function compare() {
    if (!available()) return;
    update({ busy: true, error: '', candidate: null });
    try {
      const candidate = await current();
      if (candidate && admitted()) update({ candidate });
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      update({ busy: false });
    }
  }
  function accept() {
    const value = read();
    if (!available() || value.closed || !value.candidate || value.candidate.status !== 'active')
      return;
    if (compatibleHearingKind(value.candidate) !== value.fields.kind) {
      update({
        error: 'La cabeza actual ya no tiene el tipo y modalidad escrita de esta audiencia.',
      });
      return;
    }
    register(value.candidate.revision);
    update({
      base: value.candidate,
      candidate: null,
      last: null,
      prepared: null,
      mode: 'draft',
      acknowledged: false,
      error: '',
    });
  }
  return { prepare, submit, check, retry, compare, accept };
}
