<script>
  import { getContext, onMount, onDestroy, afterUpdate } from 'svelte';
  import MeasureDecisionFields from './MeasureDecisionFields.svelte';
  import MeasureDecisionReview from './MeasureDecisionReview.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { factDenied, factUncertain, canFacts } from '../lib/procedural-fact-errors.mjs';
  import {
    captureMeasureDecisionDraft,
    createMeasureDecisionDraft,
  } from '../lib/measure-decision-draft.mjs';
  import { newMeasureEffect, decisionEditorFailure } from './measure-decision-editor-values.mjs';
  import { createMeasureDecisionActions } from './measure-decision-editor-actions.mjs';
  import { refreshDecisionSources } from './measure-decision-editor-sources.mjs';
  export let api,
    caseId,
    user,
    savedDraft = null,
    disabled = false,
    pending = false,
    ondenied,
    onconfirmed,
    oncancel;
  const session = getContext('session-drafts'),
    administration = caseState();
  const scoped = api.caseMeasureDecisions(caseId),
    hearings = api.casePrecautionaryHearings(caseId),
    measures = api.caseMeasures(caseId),
    actor = { id: user.id, email: user.email, role: user.role };
  let decisionId = savedDraft?.resourceId ?? crypto.randomUUID(),
    operationId = crypto.randomUUID(),
    fields = {
      authority: '',
      justification: '',
      locator: '',
      declaredAt: { precision: '' },
      outcome: 'changes',
      statement: '',
    },
    support = null,
    anchor = null,
    effects = [newMeasureEffect()],
    inputs = null,
    view,
    retainedInputs = null;
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
      decisionId,
      operationId,
      fields,
      support,
      anchor,
      effects,
      inputs: retainedInputs ?? inputs,
      mode,
      last,
    };
  }
  const recovery = session
    ? createMeasureDecisionDraft({
        session,
        caseId,
        decisionId,
        capture: () => captureMeasureDecisionDraft(snapshot()),
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
    if (!blocked) {
      inputs = view?.captureInputs() ?? retainedInputs ?? inputs;
      retainedInputs = inputs;
    }
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
    const context = await hearings.context();
    return admitted()
      ? { context, closed: context.administration.administrative_status === 'closed' }
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
          ({ decisionId, operationId, fields, support, anchor, effects, inputs, mode, last } =
            value);
          context = current;
          loaded = true;
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar la decision cautelar.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register();
          loaded = true;
        }
      }
      closed = context.closed;
      if (
        savedDraft &&
        mode !== 'uncertain' &&
        !(await refreshDecisionSources(api, caseId, snapshot(), admitted))
      )
        return;
      if (admitted()) blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = decisionEditorFailure(failure);
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
    error = decisionEditorFailure(failure);
    acknowledged = retryAvailable = false;
    prepared = null;
    if (failure.code === 'case_closed') closed = true;
    if (
      retain ||
      (writing &&
        (factUncertain(failure) || failure.code === 'measure_decision_operation_conflict'))
    )
      mode = 'uncertain';
    else {
      last = null;
      mode = 'draft';
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
  const actions = createMeasureDecisionActions({
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
    measures,
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
  afterUpdate(() => {
    if (!blocked && view && mode === 'draft') retainedInputs = view.captureInputs();
  });
  onMount(initialize);
  onDestroy(() => {
    recovery?.dispose();
    alive = false;
    pending = false;
    for (const client of [scoped, hearings, measures]) client.dispose();
  });
</script>

<section class="card case-editor fact-editor" aria-label="Decision cautelar" aria-busy={pending}>
  <h2>Registrar decision cautelar</h2>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if closed}<p class="notice">
      Expediente cerrado administrativamente. Puedes consultar el resultado de un envio anterior.
    </p>{/if}
  {#if blocked || closed}<button
      class="secondary"
      disabled={pending || disabled}
      onclick={initialize}>Volver a consultar el contexto</button
    >{/if}
  {#if mode === 'uncertain'}
    <h3>Resultado incierto</h3>
    <p>Consulta la operacion original antes de decidir un reintento.</p>
    <button class="primary" disabled={pending || disabled || blocked} onclick={actions.check}
      >Consultar resultado</button
    >
    {#if retryAvailable}<button
        class="secondary"
        disabled={pending || disabled || blocked || closed}
        onclick={actions.retry}>Reintentar envio exacto</button
      >{/if}
    <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
  {:else if mode === 'review' && prepared}
    <MeasureDecisionReview value={prepared} />
    <label class="checkbox"
      ><input type="checkbox" bind:checked={acknowledged} disabled={frozen} />Reconozco la decision
      y las fuentes seleccionadas</label
    >
    <div class="action-row">
      <button class="primary" disabled={frozen || !acknowledged} onclick={actions.submit}
        >Confirmar decision</button
      >
      <button
        class="secondary"
        disabled={frozen}
        onclick={() => {
          mode = 'draft';
          prepared = null;
          acknowledged = false;
        }}>Volver al borrador</button
      >
    </div>
  {/if}
  {#if blocked}<p role="status">Confirma tu acceso actual antes de recuperar la decision.</p>
  {:else if !['review', 'confirmed'].includes(mode)}
    <MeasureDecisionFields
      {api}
      {caseId}
      {inputs}
      ondenied={deny}
      canApply={admitted}
      bind:this={view}
      bind:fields
      bind:support
      bind:anchor
      bind:effects
      bind:pending={fieldsBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={actions.prepare}
      >Revisar decision</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={close}
    >Cerrar formulario</button
  >
</section>
