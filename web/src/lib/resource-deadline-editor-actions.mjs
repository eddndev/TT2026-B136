import { deadlinePoliciesCommand } from './deadline-editor-policies.mjs';
import { readResourceDeadlineSubmission } from './resource-deadline-recovery.mjs';

export function createResourceDeadlineActions({
  read,
  update,
  admitted,
  scoped,
  deadlines,
  associations,
  finish,
  fail,
  saveInputs,
  resources,
  register,
  report,
}) {
  const available = (value) =>
    !value.busy && !value.pending && !value.disabled && !value.blocked && admitted();
  async function prepare() {
    const value = read();
    if (!available(value) || value.frozen || value.mode !== 'draft') return;
    saveInputs();
    update({ busy: true, error: '', acknowledged: false, last: null });
    try {
      const command = {
        case_id: value.caseId,
        resource_id: value.resource.id,
        association_id: value.associationId,
        expected_resource_revision: value.base.revision,
        ...structuredClone(value.selection),
        deadline: {
          operation_id: crypto.randomUUID(),
          deadline_id: value.deadlineId,
          change: {
            action: 'register',
            expected_revision: 0,
            definition: structuredClone(value.definition),
            tracking: deadlinePoliciesCommand(value.policies, value.definition),
          },
        },
      };
      const prepared = await scoped.prepare(command, value.user);
      if (admitted()) update({ prepared: structuredClone(prepared), mode: 'review' });
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
      value.mode !== 'review' ||
      !value.prepared ||
      !value.acknowledged
    )
      return;
    const last = structuredClone(value.prepared);
    update({
      busy: true,
      error: '',
      last,
      mode: 'uncertain',
      retryAvailable: false,
      paired: false,
    });
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
    if (!available(value) || value.mode !== 'uncertain' || !value.last) return;
    update({ busy: true, error: '', retryAvailable: false, paired: false });
    try {
      const checked = await readResourceDeadlineSubmission(deadlines, associations, value.last);
      if (!admitted()) return;
      update({
        paired: checked.state === 'paired',
        retryAvailable: ['paired', 'absent'].includes(checked.state),
        error:
          checked.state === 'paired'
            ? 'Los recibos coinciden. Confirma el mismo envio para verificar que se guardaron juntos.'
            : checked.state === 'absent'
              ? 'Ninguna revision esta disponible todavia. La ausencia no confirma que el envio fallo.'
              : 'Los dos recibos no corresponden al mismo envio confirmado. Conserva el borrador y consulta de nuevo.',
      });
    } catch (failure) {
      if (admitted()) report(failure);
    } finally {
      update({ busy: false });
    }
  }
  async function retry() {
    const value = read();
    if (
      !available(value) ||
      (value.closed && !value.paired) ||
      value.mode !== 'uncertain' ||
      !value.last ||
      !value.retryAvailable
    )
      return;
    update({ busy: true, error: '', retryAvailable: false });
    try {
      const result = await scoped.submit(value.last);
      if (admitted()) await finish(result, true);
    } catch (failure) {
      if (admitted()) {
        fail(failure, true);
        if (!admitted()) return;
        update({ mode: 'uncertain', last: value.last });
        if (value.paired && failure.code === 'resource_activity_operation_conflict')
          update({
            error:
              'El servidor no confirma el origen conjunto de estos recibos. El resultado sigue incierto.',
          });
      }
    } finally {
      update({ busy: false });
    }
  }
  async function compare() {
    if (!available(read())) return;
    update({ busy: true, error: '', candidate: null });
    try {
      const current = await resources.get(read().resource.id);
      if (admitted()) update({ candidate: current });
    } catch (failure) {
      if (admitted()) report(failure);
    } finally {
      update({ busy: false });
    }
  }
  function accept() {
    const value = read();
    if (value.frozen || !admitted() || !value.candidate || value.candidate.status !== 'active')
      return;
    register(value.candidate.revision);
    update({ base: value.candidate, candidate: null, last: null, mode: 'draft', error: '' });
  }
  return { prepare, submit, check, retry, compare, accept };
}
