<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { createStageDraft, refreshStageSupports } from '../lib/stage-draft.mjs';
  import { captureStage, stageFormState } from '../lib/stage-draft-values.mjs';
  import { stageFormActions } from '../lib/stage-form-actions.mjs';
  import StageFormBody from './StageFormBody.svelte';
  export let api, documents, caseId, current, onconfirmed, onobserved, ondenied, oncancel;
  export let disabled = false,
    readBlocked = false,
    pending = false,
    savedDraft = null;
  const session = getContext('session-drafts'),
    administration = caseState();
  let state = stageFormState(current, savedDraft?.action);
  let alive = true,
    generation = 0,
    finished = false;
  const patch = (changes) => {
    state = { ...state, ...changes };
  };
  const recovery = session
    ? createStageDraft({ session, caseId, api, capture: () => captureStage(state) })
    : null;
  const admitted = () => alive && !finished && (!recovery || recovery.admitted());
  $: locked = disabled || readBlocked || state.blocked || state.restoredClosed || state.incomplete;
  $: supportUnreviewed =
    state.supportIssue && state.oldSupports.some(([key, value]) => state.draft[key] === value);
  $: if (state.blockedByCase && !$administration.closed) {
    state.blockedByCase = state.restoredClosed = false;
    state.error = '';
  }
  function deny(failure) {
    recovery?.discard();
    state = stageFormState(null, state.action);
    ondenied(failure);
  }
  function observe(context) {
    if (!admitted()) return;
    patch({ restoredClosed: context.closed, incomplete: context.incomplete });
    if ((context.result.current?.stage_revision ?? 0) !== (state.base?.stage_revision ?? 0))
      patch({ needsReview: true, preview: null, historyComplete: false });
    onobserved(context.result);
  }
  async function freshContext() {
    if (recovery) return recovery.freshContext();
    const result = await api.get();
    return admitted() ? { result, closed: $administration.closed, incomplete: false } : null;
  }
  async function validate(context, valid) {
    observe(context);
    if (!(await refreshStageSupports(state.draft, documents, caseId, valid)) || !valid()) return;
    patch({ blocked: false, supportIssue: false, oldSupports: [] });
  }
  async function initialize() {
    if (pending || !admitted()) return;
    const ticket = ++generation,
      valid = () => admitted() && ticket === generation;
    patch({ busy: true, blocked: true, error: '' });
    try {
      let context;
      if (savedDraft && !state.loaded && recovery) {
        const result = await recovery.restore(savedDraft, (value, fresh) => {
          patch({ ...value, loaded: true });
          context = fresh;
        });
        if (!valid()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el borrador de etapa.');
      } else {
        context = await freshContext();
        if (!context || !valid()) return;
        if (!state.loaded) recovery?.register(state.action, state.base?.stage_revision ?? 0);
        patch({ loaded: true });
      }
      await validate(context, valid);
    } catch (failure) {
      if (valid()) {
        state.error = failure.message;
        if ([403, 404].includes(failure.status)) deny(failure);
      }
    } finally {
      if (alive && ticket === generation) state.busy = false;
    }
  }
  function supportContext(field) {
    return state.blocked
      ? null
      : (recovery?.supportContext(field, () => !state.blocked && !finished, observe, deny) ?? null);
  }
  function discardSupport(field) {
    recovery?.discardSupport(field);
  }
  function close() {
    if (pending) return;
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    recovery?.close();
    finished = true;
    generation++;
    oncancel();
  }
  async function work(operation, reading = false) {
    if (pending || !admitted() || state.blocked || readBlocked || (!reading && locked)) return;
    const ticket = generation,
      valid = () => admitted() && ticket === generation;
    patch({ busy: true, error: '' });
    try {
      await operation(valid);
    } catch (failure) {
      if (valid()) {
        state.error = failure.message;
        if (failure.code === 'case_closed') state.blockedByCase = state.restoredClosed = true;
        if ([403, 404].includes(failure.status)) deny(failure);
      }
    } finally {
      if (alive && ticket === generation) state.busy = false;
    }
  }
  const actions = stageFormActions({
    api,
    caseId,
    patch,
    work,
    freshContext,
    observe,
    get: () => ({ ...state, disabled: locked || !admitted(), pending, supportUnreviewed }),
    adopt(base) {
      state.base = base;
      recovery?.register(state.action, base?.stage_revision ?? 0);
    },
    async finish(result) {
      recovery?.close();
      finished = true;
      await onconfirmed(result);
    },
  });
  onMount(initialize);
  onDestroy(() => {
    alive = false;
    generation++;
    recovery?.dispose();
    pending = false;
  });
</script>

<StageFormBody
  bind:state
  bind:pending
  {actions}
  {documents}
  {caseId}
  ondenied={deny}
  {close}
  retry={initialize}
  disabled={locked}
  {readBlocked}
  {supportUnreviewed}
  {supportContext}
  {discardSupport}
/>
