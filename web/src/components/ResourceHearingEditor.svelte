<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import ResourceHearingEditorBody from './ResourceHearingEditorBody.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    canResources,
    resourceDenied,
    resourceUncertain,
  } from '../lib/procedural-resource-errors.mjs';
  import {
    captureResourceHearingDraft,
    createResourceHearingDraft,
  } from '../lib/resource-hearing-draft.mjs';
  import {
    initialHearingFields,
    refreshHearingSources,
    resourceHearingFailure,
  } from './resource-hearing-editor-values.mjs';
  import { createResourceHearingActions } from './resource-hearing-editor-actions.mjs';
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
  const scoped = api.caseResourceHearings(caseId, resource.id),
    resources = api.caseResources(caseId),
    typed = api.caseTypedParticipants(caseId),
    cases = api.caseAdministration(caseId);
  let hearingId = crypto.randomUUID(),
    associationId = crypto.randomUUID(),
    operationId = crypto.randomUUID(),
    base = head,
    act = null,
    fields = initialHearingFields(resource),
    participants = [],
    inputs = null,
    view;
  let alive = true,
    finished = false,
    loaded = false,
    blocked = true,
    busy = false,
    fieldsBusy = false,
    closed = false,
    mode = 'draft',
    prepared = null,
    last = null,
    acknowledged = false,
    retryAvailable = false,
    candidate = null,
    error = '';
  const actor = { id: user.id, email: user.email, role: user.role };
  function snapshot() {
    return {
      hearingId,
      associationId,
      operationId,
      base,
      resource,
      act,
      fields,
      participants,
      inputs: (!blocked && view?.captureInputs()) || inputs,
      mode,
      last,
    };
  }
  const recovery = session
    ? createResourceHearingDraft({
        session,
        caseId,
        resourceId: resource.id,
        capture: () => captureResourceHearingDraft(snapshot()),
      })
    : null;
  function admitted() {
    const current = session?.principal() || user;
    return (
      alive &&
      !finished &&
      (!recovery || recovery.admitted()) &&
      current.id === actor.id &&
      current.email === actor.email &&
      current.role === actor.role
    );
  }
  function saveInputs() {
    if (!blocked) inputs = view?.captureInputs() ?? inputs;
  }
  function update(next) {
    if (!alive) return;
    ({ busy, error, acknowledged, prepared, mode, last, retryAvailable, base, candidate, closed } =
      {
        busy,
        error,
        acknowledged,
        prepared,
        mode,
        last,
        retryAvailable,
        base,
        candidate,
        closed,
        ...next,
      });
  }
  async function fresh(authorize = true) {
    if (!admitted()) return null;
    let isClosed = closed;
    if (authorize) {
      const value = session ? await session.authorizeCase(caseId) : await cases.get();
      if (!admitted()) return null;
      if (
        value.id !== caseId ||
        !['active', 'closed'].includes(value.administration?.administrative_status)
      )
        throw new Error('No se pudo confirmar el expediente actual.');
      isClosed = value.administration.administrative_status === 'closed';
    }
    const current = await resources.get(resource.id);
    return admitted() ? { current, closed: isClosed } : null;
  }
  async function initialize() {
    if (!admitted() || busy) return;
    saveInputs();
    busy = blocked = true;
    error = '';
    prepared = null;
    acknowledged = retryAvailable = false;
    if (mode === 'review') mode = 'draft';
    try {
      let context;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(
          savedDraft,
          () => fresh(false),
          (value, current) => {
            ({
              hearingId,
              associationId,
              operationId,
              base,
              resource,
              act,
              fields,
              participants,
              inputs,
              mode,
              last,
            } = value);
            context = current;
            loaded = true;
          },
        );
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar la audiencia de este recurso.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(base.revision);
          loaded = true;
        }
      }
      closed = context.closed;
      if (savedDraft && loaded) {
        const sources = await refreshHearingSources(resources, typed, snapshot(), admitted);
        if (!sources || !admitted()) return;
        ({ resource, act, participants } = sources);
      }
      if (
        mode !== 'uncertain' &&
        (context.current.revision !== base.revision || context.current.status !== 'active')
      )
        mode = 'conflict';
      if (admitted()) blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = resourceHearingFailure(failure);
        if (resourceDenied(failure)) deny(failure);
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
    prepared = last = null;
    ondenied(failure);
  }
  function fail(failure, writing = false, retain = false) {
    if (resourceDenied(failure)) return deny(failure);
    error = resourceHearingFailure(failure);
    acknowledged = retryAvailable = false;
    prepared = null;
    if (failure.code === 'case_closed') closed = true;
    if (
      retain ||
      (writing &&
        (resourceUncertain(failure) || failure.code === 'resource_activity_operation_conflict'))
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
  function close() {
    if (pending || disabled) return;
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    oncancel();
  }
  async function finish(value) {
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    prepared = last = null;
    mode = 'confirmed';
    await onconfirmed(value);
  }
  const actions = createResourceHearingActions({
    read: () => ({
      ...snapshot(),
      caseId,
      user,
      pending,
      disabled,
      blocked,
      closed,
      mode,
      prepared,
      acknowledged,
      retryAvailable,
      candidate,
    }),
    update,
    admitted,
    fresh,
    scoped,
    finish,
    fail,
    saveInputs,
    register: (revision) => recovery?.register(revision),
  });
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled ||
    blocked ||
    pending ||
    closed ||
    $administration.closed ||
    !canResources(user.role, 'manage') ||
    !admitted() ||
    ['uncertain', 'confirmed'].includes(mode);
  onMount(initialize);
  onDestroy(() => {
    recovery?.dispose();
    alive = false;
    pending = false;
    for (const client of [scoped, resources, typed, cases]) client.dispose();
  });
</script>

<ResourceHearingEditorBody
  bind:this={view}
  {api}
  {caseId}
  {resource}
  ondenied={deny}
  canApply={admitted}
  {inputs}
  bind:act
  bind:fields
  bind:participants
  bind:fieldsBusy
  bind:mode
  bind:prepared
  bind:acknowledged
  {pending}
  {disabled}
  {frozen}
  {blocked}
  {closed}
  {error}
  {candidate}
  {retryAvailable}
  {actions}
  {initialize}
  {close}
/>
