<script>
  import { getContext, onMount, onDestroy, afterUpdate } from 'svelte';
  import MeasureRecordSummary from './MeasureRecordSummary.svelte';
  import MeasureAdministrationFields from './MeasureAdministrationFields.svelte';
  import MeasureAdministrationReview from './MeasureAdministrationReview.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { factDenied, factUncertain, canFacts } from '../lib/procedural-fact-errors.mjs';
  import {
    captureMeasureAdministrationDraft,
    createMeasureAdministrationDraft,
  } from '../lib/measure-administration-draft.mjs';
  import {
    administrationFields,
    administrationEditorFailure,
  } from './measure-administration-editor-values.mjs';
  import { createMeasureAdministrationActions } from './measure-administration-editor-actions.mjs';
  import { refreshAdministrationSources } from './measure-administration-editor-sources.mjs';
  export let api,
    caseId,
    user,
    action,
    base,
    savedDraft = null,
    disabled = false,
    pending = false,
    ondenied,
    onconfirmed,
    oncancel;
  const session = getContext('session-drafts'),
    administration = caseState();
  const scoped = api.caseMeasureAdministrations(caseId),
    hearings = api.casePrecautionaryHearings(caseId),
    measures = api.caseMeasures(caseId),
    actor = { id: user.id, email: user.email, role: user.role };
  let operationId = crypto.randomUUID(),
    fields = base
      ? administrationFields(base)
      : {
          reason: '',
          conditions: '',
          validity: { start: { precision: '' }, statement: '', end: null },
          supervisionText: '',
        },
    replacementId = action === 'replace_entered_in_error' ? crypto.randomUUID() : null,
    subject = null,
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
      action,
      base,
      replacementId,
      operationId,
      fields,
      subject,
      inputs: retainedInputs ?? inputs,
      mode,
      last,
    };
  }
  const recovery = session
    ? createMeasureAdministrationDraft({
        session,
        caseId,
        measureId: savedDraft?.resourceId ?? base.reference.id,
        action,
        capture: () => captureMeasureAdministrationDraft(snapshot()),
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
          ({ action, base, replacementId, operationId, fields, subject, inputs, mode, last } =
            value);
          context = current;
          loaded = true;
        });
        if (!admitted()) return;
        if (result.status !== 'restored') throw new Error('No se pudo recuperar la rectificacion.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(base.reference.revision);
          loaded = true;
        }
      }
      closed = context.closed;
      if (
        savedDraft &&
        mode !== 'uncertain' &&
        !(await refreshAdministrationSources(api, caseId, snapshot(), admitted))
      )
        return;
      if (admitted()) blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = administrationEditorFailure(failure);
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
    error = administrationEditorFailure(failure);
    acknowledged = retryAvailable = false;
    prepared = null;
    if (failure.code === 'case_closed') closed = true;
    if (
      retain ||
      (writing &&
        (factUncertain(failure) || failure.code === 'measure_administrative_operation_conflict'))
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
  const actions = createMeasureAdministrationActions({
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

<section
  class="card case-editor fact-editor"
  aria-label="Rectificacion de medida"
  aria-busy={pending}
>
  <h2>Rectificar registro de medida</h2>
  {#if !blocked && base}<details class="case-comparison">
      <summary
        >Registro exacto que se rectifica: {base.record.capture.result.projection.subject
          .display_name} / Revision {base.reference.revision}</summary
      >
      <MeasureRecordSummary value={base} />
    </details>{/if}
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
    <MeasureAdministrationReview value={prepared} bind:confirmed={acknowledged} disabled={frozen} />
    <div class="action-row">
      <button class="primary" disabled={frozen || !acknowledged} onclick={actions.submit}
        >Confirmar rectificacion</button
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
  {#if blocked}<p role="status">Confirma tu acceso actual antes de recuperar la rectificacion.</p>
  {:else if !['review', 'confirmed'].includes(mode)}
    <MeasureAdministrationFields
      {api}
      {caseId}
      {inputs}
      ondenied={deny}
      canApply={admitted}
      bind:this={view}
      bind:fields
      {action}
      bind:subject
      bind:pending={fieldsBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={actions.prepare}
      >Revisar rectificacion</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={close}
    >Cerrar formulario</button
  >
</section>
