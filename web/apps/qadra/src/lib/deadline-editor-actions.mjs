import {
  deadlinePoliciesCommand,
  adoptDeadlineDefinition,
  reconcileDeadlinePolicies,
} from './deadline-editor-policies.mjs';
import { readDeadlineSubmission } from './deadline-submission.mjs';

export function createDeadlineActions({
  read,
  update,
  admitted,
  scoped,
  saveInputs,
  finish,
  fail,
  report,
  register,
}) {
  const available = (value) => !value.pending && !value.disabled && !value.blocked && admitted();
  async function prepare() {
    const value = read();
    if (!available(value) || value.frozen || value.step !== 'draft') return;
    saveInputs();
    update({ busy: true, error: '', acknowledge: false, last: null });
    try {
      const change = { action: value.mode, expected_revision: value.current?.revision ?? 0 };
      if (['register', 'correct'].includes(value.mode)) {
        change.definition = structuredClone(value.definition);
        change.tracking = deadlinePoliciesCommand(value.policies, value.definition);
      }
      if (value.mode === 'set_attention') change.attention = structuredClone(value.attention);
      if (value.mode !== 'register') change.reason = value.reason;
      const prepared = await scoped.prepare(
        { operation_id: crypto.randomUUID(), deadline_id: value.id, change },
        value.user,
      );
      if (admitted()) update({ prepared: structuredClone(prepared), step: 'review' });
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      update({ busy: false });
    }
  }
  async function submit() {
    const value = read();
    if (
      !available(value) ||
      value.frozen ||
      value.step !== 'review' ||
      !value.prepared ||
      !value.acknowledge
    )
      return;
    const last = structuredClone(value.prepared);
    update({ busy: true, error: '', last, step: 'uncertain', prepared: null, acknowledge: false });
    try {
      const result = await scoped.submit(last);
      if (admitted()) await finish(result);
    } catch (failure) {
      if (admitted()) fail(failure, true);
    } finally {
      update({ busy: false });
    }
  }
  async function check() {
    const value = read();
    if (!available(value) || value.step !== 'uncertain' || !value.last) return;
    update({ busy: true, error: '' });
    try {
      const checked = await readDeadlineSubmission(scoped, value.last);
      if (!admitted()) return;
      if (checked.state === 'matched') await finish(checked.record, true);
      else if (checked.state === 'absent')
        update({
          error:
            'La revision aun no esta disponible. El resultado sigue incierto; puedes consultar de nuevo.',
        });
      else
        update({
          step: 'conflict',
          candidate: checked.record,
          compared: false,
          error: 'La revision pertenece a otro envio. Conservamos tu borrador.',
        });
    } catch (failure) {
      if (admitted()) report(failure);
    } finally {
      update({ busy: false });
    }
  }
  async function compare() {
    const value = read();
    if (!available(value)) return;
    update({ busy: true, error: '', compared: false });
    try {
      const candidate = await scoped.get(value.id);
      if (admitted()) update({ candidate, compared: true });
    } catch (failure) {
      if (admitted()) report(failure);
    } finally {
      update({ busy: false });
    }
  }
  function accept() {
    const value = read();
    if (
      !available(value) ||
      value.frozen ||
      !value.compared ||
      value.candidate?.status !== 'active' ||
      value.mode === 'register'
    )
      return;
    saveInputs();
    const next = {};
    if (value.mode === 'correct') {
      next.definition = adoptDeadlineDefinition(
        value.current.definition,
        value.definition,
        value.candidate.definition,
      );
      if (
        JSON.stringify(value.current.definition.input.selection) ===
        JSON.stringify(value.definition.input.selection)
      )
        next.inputs = { ...read().inputs, definition: null };
      if (next.definition.responsible_id === value.candidate.responsible.id)
        next.responsible = value.candidate.responsible;
      next.policies = reconcileDeadlinePolicies(value.policies, next.definition);
      next.profile = null;
      next.fieldsVersion = value.fieldsVersion + 1;
    }
    register(value.candidate.id, value.candidate.revision);
    update({
      ...next,
      current: value.candidate,
      prepared: null,
      last: null,
      step: 'draft',
      compared: false,
      error: '',
      acknowledge: false,
    });
  }
  return { prepare, submit, check, compare, accept };
}
