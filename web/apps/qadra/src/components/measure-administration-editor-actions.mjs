import {
  assertAdministrationBase,
  administrationFormCommand,
} from './measure-administration-editor-values.mjs';

export function createMeasureAdministrationActions({
  read,
  update,
  admitted,
  fresh,
  scoped,
  measures,
  finish,
  fail,
  saveInputs,
}) {
  const available = () => {
    const value = read();
    return admitted() && !value.pending && !value.disabled && !value.blocked;
  };
  async function current(writing = false, checkBase = false) {
    const value = await fresh();
    if (!value || !admitted()) return null;
    update({ closed: value.closed });
    if (writing && value.closed) throw { status: 409, code: 'case_closed' };
    if (checkBase) {
      const base = read().base,
        current = await measures.get(base.reference.id);
      if (!admitted()) return null;
      assertAdministrationBase(base, current);
    }
    return value.context;
  }
  async function prepare() {
    const value = read();
    if (!available() || value.mode !== 'draft' || value.closed) return;
    saveInputs();
    update({ busy: true, error: '', prepared: null, last: null, acknowledged: false });
    try {
      const context = await current(true, true);
      if (!context) return;
      const prepared = await scoped.prepare(administrationFormCommand(value, context), value.actor);
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
      if (!(await current(true, true))) return;
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
      if (result.state === 'confirmed') await finish(result.operation);
      else
        update({
          retryAvailable: true,
          error:
            'El resultado sigue incierto. La ausencia del recibo no confirma que el envio fallo.',
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
      if (!(await current(true))) return;
      const result = await scoped.submit(value.last);
      if (admitted()) await finish(result);
    } catch (failure) {
      if (admitted()) fail(failure, true, true);
    } finally {
      update({ busy: false });
    }
  }
  return { prepare, submit, check, retry };
}
