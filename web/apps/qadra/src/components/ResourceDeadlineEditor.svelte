<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import ResourceDeadlineEditorBody from './ResourceDeadlineEditorBody.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    canDeadlines,
    deadlineFailure,
    deadlineDenied,
    deadlineUncertain,
  } from '../lib/deadline-errors.mjs';
  import { resourceActivityFailure } from '../lib/resource-activity-errors.mjs';
  import { initialDeadlinePolicies } from '../lib/deadline-editor-policies.mjs';
  import {
    initialResourceDeadline,
    captureResourceDeadline,
    createResourceDeadlineDraft,
  } from '../lib/resource-deadline-draft.mjs';
  import { refreshResourceDeadlineReferences } from '../lib/resource-deadline-references.mjs';
  import { createResourceDeadlineActions } from '../lib/resource-deadline-editor-actions.mjs';
  export let api,
    caseId,
    user,
    resource,
    head,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false,
    savedDraft = null;
  const session = getContext('session-drafts'),
    administration = caseState();
  const scoped = api.caseResourceDeadlines(caseId, resource.id),
    deadlines = api.deadlines(caseId),
    associations = api.caseResourceActivities(caseId, resource.id),
    resources = api.caseResources(caseId);
  let deadlineId = crypto.randomUUID(),
    associationId = crypto.randomUUID(),
    base = head;
  let selection = {
    resource: {
      id: resource.id,
      revision: resource.revision,
      capture_digest: resource.receipt.capture_digest,
    },
    act: null,
  };
  let definition = initialResourceDeadline(caseId),
    policies = initialDeadlinePolicies(definition);
  let profile = null,
    responsible = null,
    inputs = null,
    view;
  let alive = true,
    busy = false,
    fieldsBusy = false,
    contextBusy = false;
  let blocked = true,
    loaded = false,
    finished = false,
    restoredClosed = false;
  let mode = 'draft',
    prepared = null,
    last = null,
    retryAvailable = false,
    paired = false,
    candidate = null,
    acknowledged = false,
    error = '';
  const recovery = session
    ? createResourceDeadlineDraft({
        session,
        caseId,
        resourceId: resource.id,
        capture: () =>
          captureResourceDeadline({
            deadlineId,
            associationId,
            base,
            selection,
            definition,
            policies,
            mode,
            last,
            inputs: (blocked ? null : view?.captureInputs()) ?? inputs,
          }),
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  const message = (failure) =>
    failure?.code?.startsWith('resource_') || failure?.code?.startsWith('procedural_resource_')
      ? resourceActivityFailure(failure)
      : deadlineFailure(failure);
  function saveInputs() {
    if (!blocked) inputs = view?.captureInputs() ?? inputs;
  }
  function update(next) {
    if (!alive) return;
    ({ busy, error, acknowledged, prepared, mode, last, retryAvailable, paired, base, candidate } =
      {
        busy,
        error,
        acknowledged,
        prepared,
        mode,
        last,
        retryAvailable,
        paired,
        base,
        candidate,
        ...next,
      });
  }
  const actions = createResourceDeadlineActions({
    read: () => ({
      caseId,
      user,
      resource,
      deadlineId,
      associationId,
      base,
      selection,
      definition,
      policies,
      busy,
      pending,
      disabled,
      blocked,
      frozen,
      closed,
      mode,
      prepared,
      last,
      acknowledged,
      retryAvailable,
      paired,
      candidate,
    }),
    update,
    admitted,
    scoped,
    deadlines,
    associations,
    finish,
    fail,
    saveInputs,
    resources,
    report,
    register: (revision) => recovery?.register(revision),
  });
  async function fresh() {
    if (!admitted()) return null;
    if (!session) return { current: head, closed: $administration.closed };
    const value = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    if (
      value.id !== caseId ||
      !['active', 'closed'].includes(value.administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    let current;
    try {
      current = await resources.get(resource.id);
    } catch (failure) {
      if (failure.status === 404) failure.deadlineOwnerDenied = true;
      throw failure;
    }
    return admitted()
      ? { current, closed: value.administration.administrative_status === 'closed' }
      : null;
  }
  async function initialize() {
    if (session?.canAdmit() && !canDeadlines(session.principal()?.role, 'manage')) {
      deny({ status: 403, code: 'permission_denied' });
      return;
    }
    if (pending || !admitted()) return;
    saveInputs();
    blocked = busy = true;
    error = '';
    retryAvailable = paired = acknowledged = false;
    prepared = null;
    if (mode === 'review') mode = 'draft';
    try {
      let context;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(savedDraft, fresh, (value, current) => {
          ({
            deadlineId,
            associationId,
            base,
            selection,
            definition,
            policies,
            mode,
            last,
            inputs,
          } = value);
          context = current;
          loaded = true;
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el plazo del recurso.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(base.revision);
          loaded = true;
        }
      }
      restoredClosed = context.closed;
      if (
        mode !== 'uncertain' &&
        (context.current.revision !== base.revision || context.current.status !== 'active')
      )
        mode = 'conflict';
      if (!context.closed && $administration.closed) {
        await $administration.refresh?.();
        if (!admitted()) return;
      }
      if (savedDraft) {
        const references = await refreshResourceDeadlineReferences(
          api,
          caseId,
          resource.id,
          selection,
          definition,
          admitted,
        );
        if (!references || !admitted()) return;
        ({ profile, responsible } = references);
      }
      if (admitted()) blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = message(failure);
        if (deadlineDenied(failure) || failure.deadlineOwnerDenied) deny(failure);
      }
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
  function close() {
    if (pending || disabled) return;
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    oncancel();
  }
  $: closed = restoredClosed || $administration.closed;
  $: pending = busy || fieldsBusy || contextBusy;
  $: frozen =
    disabled ||
    blocked ||
    !admitted() ||
    pending ||
    closed ||
    !canDeadlines(user.role, 'manage') ||
    mode === 'uncertain' ||
    mode === 'confirmed';
  function fail(failure, writing = false) {
    if (deadlineDenied(failure)) return deny(failure);
    error = message(failure);
    prepared = null;
    acknowledged = retryAvailable = false;
    if (
      writing &&
      (deadlineUncertain(failure) ||
        [
          'resource_activity_operation_conflict',
          'deadline_operation_conflict',
          'deadline_revision_conflict',
        ].includes(failure.code))
    )
      mode = 'uncertain';
    else {
      last = null;
      if (
        [
          'resource_activity_resource_revision_conflict',
          'resource_activity_resource_archived',
        ].includes(failure.code)
      ) {
        mode = 'conflict';
        candidate = null;
      } else mode = 'draft';
    }
  }
  function report(failure) {
    error = message(failure);
    if (deadlineDenied(failure)) deny(failure);
  }
  async function finish(value, exact = false) {
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    prepared = last = null;
    mode = 'confirmed';
    try {
      await onconfirmed(value.association, exact);
    } catch {
      if (alive) error = 'El plazo y vinculo se guardaron. Consulta de nuevo sin reenviar.';
    }
  }
  onMount(initialize);
  onDestroy(() => {
    recovery?.dispose();
    alive = false;
    pending = false;
    for (const scope of [scoped, deadlines, associations, resources]) scope.dispose();
  });
</script>

<ResourceDeadlineEditorBody
  bind:this={view}
  {api}
  {caseId}
  {resource}
  ondenied={deny}
  bind:selection
  bind:definition
  bind:policies
  bind:profile
  bind:responsible
  bind:fieldsBusy
  bind:contextBusy
  bind:mode
  bind:prepared
  bind:acknowledged
  {pending}
  {disabled}
  {frozen}
  {closed}
  {blocked}
  {error}
  {candidate}
  {retryAvailable}
  {paired}
  {inputs}
  recoverable={!!recovery}
  {actions}
  {close}
  {initialize}
/>
