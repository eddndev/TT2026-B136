<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import PrecautionaryHearingEditorBody from './PrecautionaryHearingEditorBody.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { factDenied, factUncertain, canFacts } from '../lib/procedural-fact-errors.mjs';
  import {
    capturePrecautionaryHearingDraft,
    createPrecautionaryHearingDraft,
  } from '../lib/precautionary-hearing-draft.mjs';
  import {
    initialPrecautionaryFields,
    refreshPrecautionarySources,
    precautionaryEditorFailure,
    assertPrecautionaryBase,
  } from './precautionary-hearing-editor-values.mjs';
  import { createPrecautionaryHearingActions } from './precautionary-hearing-editor-actions.mjs';
  export let api,
    caseId,
    user,
    action,
    base = null,
    savedDraft = null,
    disabled = false,
    pending = false,
    ondenied,
    onconfirmed,
    oncancel;
  const session = getContext('session-drafts'),
    administration = caseState();
  const scoped = api.casePrecautionaryHearings(caseId),
    typed = api.caseTypedParticipants(caseId),
    documents = api.caseDocuments(caseId);
  const actor = { id: user.id, email: user.email, role: user.role };
  let hearingId =
      savedDraft?.resourceId ?? base?.capture.review.command.hearing_id ?? crypto.randomUUID(),
    operationId = crypto.randomUUID(),
    fields = initialPrecautionaryFields(base),
    participants = structuredClone(base?.capture.review.sources.participants ?? []),
    support = structuredClone(base?.capture.review.sources.support ?? null),
    reviewTargets = structuredClone(base?.capture.review.resolved_values.review_targets ?? []),
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
    error = '';
  function snapshot() {
    return {
      hearingId,
      operationId,
      action,
      base,
      fields,
      participants,
      support,
      reviewTargets,
      inputs: (!blocked && view?.captureInputs()) || inputs,
      mode,
      last,
    };
  }
  const recovery = session
    ? createPrecautionaryHearingDraft({
        session,
        caseId,
        action,
        hearingId,
        capture: () => capturePrecautionaryHearingDraft(snapshot()),
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
    ({ busy, error, acknowledged, prepared, mode, last, retryAvailable, closed } = {
      busy,
      error,
      acknowledged,
      prepared,
      mode,
      last,
      retryAvailable,
      closed,
      ...next,
    });
  }
  async function fresh() {
    if (!admitted()) return null;
    const context = await scoped.context();
    if (!admitted()) return null;
    const current = action === 'schedule' ? null : await scoped.get(hearingId);
    return admitted()
      ? { context, current, closed: context.administration.administrative_status === 'closed' }
      : null;
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
        const result = await recovery.restore(savedDraft, fresh, (value, current) => {
          ({
            hearingId,
            operationId,
            base,
            fields,
            participants,
            support,
            reviewTargets,
            inputs,
            mode,
            last,
          } = value);
          context = current;
          loaded = true;
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar la convocatoria cautelar.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(base?.capture.review.result_revision ?? 0);
          loaded = true;
        }
      }
      closed = context.closed;
      if (savedDraft && mode !== 'uncertain') {
        if (
          !(await refreshPrecautionarySources(
            typed,
            documents,
            { ...snapshot(), caseId },
            admitted,
          ))
        )
          return;
      }
      if (mode !== 'uncertain') {
        try {
          assertPrecautionaryBase(base, context.current);
        } catch (failure) {
          mode = 'conflict';
          error = precautionaryEditorFailure(failure);
        }
      }
      if (admitted()) blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = precautionaryEditorFailure(failure);
        if (factDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function deny(failure) {
    if (failure.status === 401) {
      blocked = true;
      ondenied(failure);
      return;
    }
    if (failure.code === 'case_not_found' || failure.status === 403)
      session?.registry.denyContext(caseId);
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = blocked = true;
    prepared = last = null;
    ondenied(failure);
  }
  function fail(failure, writing = false, retain = false) {
    if (factDenied(failure)) return deny(failure);
    error = precautionaryEditorFailure(failure);
    acknowledged = retryAvailable = false;
    prepared = null;
    if (failure.code === 'case_closed') closed = true;
    if (
      retain ||
      (writing &&
        (factUncertain(failure) || failure.code === 'precautionary_hearing_operation_conflict'))
    )
      mode = 'uncertain';
    else {
      last = null;
      mode = failure.code === 'precautionary_editor_base_changed' ? 'conflict' : 'draft';
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
  const actions = createPrecautionaryHearingActions({
    read: () => ({
      ...snapshot(),
      caseId,
      actor,
      pending,
      disabled,
      blocked,
      closed,
      mode,
      prepared,
      acknowledged,
      retryAvailable,
    }),
    update,
    admitted,
    fresh,
    scoped,
    finish,
    fail,
    saveInputs,
  });
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled ||
    blocked ||
    pending ||
    closed ||
    $administration.closed ||
    !canFacts(user.role, 'manage') ||
    !admitted() ||
    ['uncertain', 'confirmed'].includes(mode);
  onMount(initialize);
  onDestroy(() => {
    recovery?.dispose();
    alive = false;
    pending = false;
    for (const client of [scoped, typed, documents]) client.dispose();
  });
</script>

<PrecautionaryHearingEditorBody
  bind:this={view}
  {api}
  {caseId}
  {action}
  {inputs}
  ondenied={deny}
  canApply={admitted}
  bind:fields
  bind:participants
  bind:support
  bind:reviewTargets
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
  {retryAvailable}
  {actions}
  {initialize}
  {close}
/>
