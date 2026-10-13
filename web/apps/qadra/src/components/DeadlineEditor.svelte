<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import DeadlineEditorBody from './DeadlineEditorBody.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    canDeadlines,
    deadlineDenied,
    deadlineFailure,
    deadlineUncertain,
  } from '../lib/deadline-errors.mjs';
  import { initialDeadlinePolicies } from '../lib/deadline-editor-policies.mjs';
  import { initialDeadline, captureDeadline, createDeadlineDraft } from '../lib/deadline-draft.mjs';
  import { refreshDeadlineReferences } from '../lib/deadline-draft-references.mjs';
  import { createDeadlineActions } from '../lib/deadline-editor-actions.mjs';
  export let api,
    user,
    caseId,
    base = null,
    mode = 'register',
    onsaved,
    oncancel,
    ondenied = () => {},
    pending = false,
    disabled = false,
    savedDraft = null;
  const session = getContext('session-drafts'),
    administration = caseState();
  const scoped = canDeadlines(user?.role, 'manage') ? api.deadlines(caseId) : null;
  let current = base,
    id = base?.id || crypto.randomUUID();
  let definition = base ? structuredClone(base.definition) : initialDeadline(caseId);
  let policies = initialDeadlinePolicies(definition, base?.tracking?.policies);
  let attention = { status: '' },
    reason = '',
    profile = null,
    responsible = base?.responsible || null;
  let inputs = null,
    view,
    fieldsVersion = 0;
  let alive = true,
    busy = false,
    fieldsBusy = false,
    blocked = true,
    loaded = false,
    finished = false,
    restoredClosed = false;
  let step = 'draft',
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    error = '',
    acknowledge = false;
  const recovery = session
    ? createDeadlineDraft({
        session,
        caseId,
        action: mode,
        capture: () =>
          captureDeadline({
            id,
            current,
            definition,
            policies,
            attention,
            reason,
            step,
            last,
            inputs: (blocked ? null : view?.captureInputs()) ?? inputs,
          }),
      })
    : null;
  function admitted() {
    return (
      alive &&
      !!scoped &&
      canDeadlines(user?.role, 'manage') &&
      !finished &&
      (!recovery || recovery.admitted())
    );
  }
  function saveInputs() {
    if (!blocked) inputs = view?.captureInputs() ?? inputs;
  }
  function update(next) {
    if (!alive) return;
    ({
      busy,
      error,
      acknowledge,
      prepared,
      step,
      last,
      current,
      candidate,
      compared,
      definition,
      policies,
      profile,
      responsible,
      fieldsVersion,
      inputs,
    } = {
      busy,
      error,
      acknowledge,
      prepared,
      step,
      last,
      current,
      candidate,
      compared,
      definition,
      policies,
      profile,
      responsible,
      fieldsVersion,
      inputs,
      ...next,
    });
  }
  const actions = createDeadlineActions({
    read: () => ({
      user,
      mode,
      id,
      current,
      definition,
      policies,
      attention,
      reason,
      pending,
      disabled,
      blocked,
      frozen,
      step,
      prepared,
      acknowledge,
      last,
      candidate,
      compared,
      fieldsVersion,
      inputs,
    }),
    update,
    admitted,
    scoped,
    saveInputs,
    finish,
    fail,
    report,
    register: (resourceId, revision) => recovery?.register(resourceId, revision),
  });
  async function fresh() {
    if (!admitted()) return null;
    if (!session) return { current: base, closed: $administration.closed };
    const record = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    const currentAdministration = record.administration;
    if (
      record.id !== caseId ||
      currentAdministration?.case_id !== caseId ||
      !['active', 'closed'].includes(currentAdministration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    let head = null;
    if (mode !== 'register') {
      try {
        head = await scoped.get(id);
      } catch (failure) {
        if (failure.status === 404) failure.deadlineOwnerDenied = true;
        throw failure;
      }
    }
    return admitted()
      ? { current: head, closed: currentAdministration.administrative_status === 'closed' }
      : null;
  }
  async function initialize() {
    if (session?.canAdmit() && !canDeadlines(session.principal()?.role, 'manage')) {
      deny({ status: 403, code: 'permission_denied' });
      return;
    }
    if (busy || !admitted()) return;
    saveInputs();
    blocked = busy = true;
    error = '';
    acknowledge = compared = false;
    prepared = candidate = null;
    if (step === 'review') step = 'draft';
    try {
      let context;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(savedDraft, fresh, (value, freshContext) => {
          ({ id, current, definition, policies, attention, reason, step, last, inputs } = value);
          context = freshContext;
          loaded = true;
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el borrador de plazo.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(mode === 'register' ? null : id, current?.revision ?? 0);
          loaded = true;
        }
      }
      restoredClosed = context.closed;
      if (
        current &&
        step !== 'uncertain' &&
        (context.current.revision !== current.revision || context.current.status !== 'active')
      )
        step = 'conflict';
      if (!context.closed && $administration.closed) {
        await $administration.refresh?.();
        if (!admitted()) return;
      }
      if (savedDraft && ['register', 'correct'].includes(mode)) {
        const references = await refreshDeadlineReferences(api, caseId, definition, admitted);
        if (!references || !admitted()) return;
        ({ profile, responsible } = references);
      }
      if (admitted()) {
        fieldsVersion++;
        blocked = false;
      }
    } catch (failure) {
      if (admitted()) report(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  function deny(failure) {
    if (failure.code === 'case_not_found' || failure.status === 403)
      session?.registry.denyContext(caseId);
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = blocked = true;
    ondenied(failure);
  }
  function report(failure) {
    error = deadlineFailure(failure);
    if (deadlineDenied(failure) || failure.deadlineOwnerDenied) deny(failure);
  }
  function fail(failure, writing = false) {
    prepared = null;
    acknowledge = false;
    if (deadlineDenied(failure)) return deny(failure);
    error = deadlineFailure(failure);
    if (writing && (deadlineUncertain(failure) || failure.code === 'deadline_operation_conflict')) {
      step = 'uncertain';
      error =
        'No se pudo confirmar el resultado. Conservamos el envio para consultar su revision exacta.';
    } else {
      last = null;
      if (failure.code === 'deadline_revision_exhausted') step = 'exhausted';
      else if (
        ['deadline_revision_conflict', 'deadline_retired', 'deadline_not_found'].includes(
          failure.code,
        )
      ) {
        step = 'conflict';
        candidate = null;
        compared = false;
      } else step = 'draft';
    }
  }
  function close() {
    if (pending || disabled) return;
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    oncancel();
  }
  async function finish(value, exact = false) {
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    prepared = last = null;
    step = 'confirmed';
    try {
      await onsaved(value, exact);
    } catch {
      if (alive)
        error = 'El plazo se guardo. Consulta de nuevo sin reenviar si faltan datos en pantalla.';
    }
  }
  $: allowed = canDeadlines(user?.role, 'manage');
  $: closed = restoredClosed || $administration.closed;
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled ||
    blocked ||
    !admitted() ||
    pending ||
    closed ||
    !allowed ||
    ['uncertain', 'exhausted', 'confirmed'].includes(step);
  onMount(initialize);
  onDestroy(() => {
    recovery?.dispose();
    alive = false;
    pending = false;
    scoped?.dispose();
  });
</script>

{#if allowed}
  <DeadlineEditorBody
    bind:this={view}
    {api}
    {caseId}
    {mode}
    {pending}
    {frozen}
    {step}
    {error}
    {blocked}
    {closed}
    {fieldsVersion}
    {prepared}
    {last}
    {candidate}
    {compared}
    {inputs}
    recoverable={!!recovery}
    bind:definition
    bind:policies
    bind:attention
    bind:reason
    bind:profile
    bind:responsible
    bind:acknowledge
    bind:fieldsBusy
    fieldsDisabled={disabled ||
      blocked ||
      busy ||
      closed ||
      ['uncertain', 'exhausted'].includes(step)}
    ondenied={deny}
    {actions}
    oncontext={initialize}
    onclose={close}
    onback={() => {
      prepared = null;
      step = 'draft';
      acknowledge = false;
    }}
  />
{/if}
