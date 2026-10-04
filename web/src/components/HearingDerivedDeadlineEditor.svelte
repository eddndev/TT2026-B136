<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import HearingDerivedDeadlineFields from './HearingDerivedDeadlineFields.svelte';
  import HearingDerivedDeadlineReview from './HearingDerivedDeadlineReview.svelte';
  import HearingResultSources from './HearingResultSources.svelte';
  import HearingResultValues from './HearingResultValues.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { hearingResultDraft, hearingResultCommand } from '../lib/hearing-result-values.mjs';
  import { createHearingDerivedDeadlineFailure } from '../lib/hearing-derived-deadline-failure.mjs';
  import { initialResourceDeadline } from '../lib/resource-deadline-draft.mjs';
  import {
    initialDeadlinePolicies,
    deadlinePoliciesCommand,
  } from '../lib/deadline-editor-policies.mjs';
  import { readResultDraftSources, refreshHearingReferences } from '../lib/hearing-draft.mjs';
  import { createHearingDerivedDeadlineActions } from '../lib/hearing-derived-deadline-actions.mjs';
  import {
    createHearingDerivedDeadlineDraft,
    captureHearingDerivedDeadlineDraft,
  } from '../lib/hearing-derived-deadline-draft.mjs';
  export let api,
    caseId,
    user,
    hearing,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false,
    savedDraft = null;
  const session = getContext('session-drafts'),
    administration = caseState();
  const scoped = api.caseHearingDerivedDeadlines(caseId, hearing.id),
    hearings = api.caseHearings(caseId),
    participants = api.caseParticipants(caseId),
    typed = api.caseTypedParticipants(caseId),
    documents = api.caseDocuments(caseId);
  let resultId = crypto.randomUUID(),
    deadlineId = crypto.randomUUID();
  let anchor = {
      hearing_id: hearing.id,
      revision: hearing.revision,
      values_digest: hearing.values_digest,
      submission_digest: hearing.receipt.submission_digest,
      status: hearing.status,
      kind: hearing.values.kind,
      scheduled_at: hearing.values.scheduled_at,
      scheduling_context: hearing.scheduling_context,
    },
    continuation = null,
    draft = hearingResultDraft(null),
    rows = [],
    candidates = hearing.participants;
  let definition = initialResourceDeadline(caseId);
  definition.input.selection.source = {
    kind: 'known',
    value: {
      family: 'hearing_result',
      hearing_id: hearing.id,
      result_id: resultId,
      revision: 1,
      agreement_id: null,
    },
  };
  let policies = initialDeadlinePolicies(definition),
    mode = 'draft',
    prepared = null,
    last = null,
    inputs = null,
    acknowledged = false,
    retryAvailable = false,
    error = '';
  let alive = true,
    finished = false,
    blocked = true,
    loaded = false,
    restoredClosed = false,
    busy = false,
    fieldsBusy = false,
    fields;
  const recovery = session
    ? createHearingDerivedDeadlineDraft({
        session,
        caseId,
        hearingId: hearing.id,
        capture: () =>
          captureHearingDerivedDeadlineDraft({
            resultId,
            deadlineId,
            anchor,
            continuation,
            draft,
            definition,
            policies,
            mode,
            last,
            inputs: fields?.captureInputs() ?? inputs,
          }),
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  $: pending = busy || fieldsBusy;
  $: closed = restoredClosed || $administration.closed;
  $: frozen = disabled || pending || blocked || closed || !admitted();
  function update(next) {
    if (!alive) return;
    if (Object.hasOwn(next, 'draft')) draft = next.draft;
    ({ busy, error, acknowledged, prepared, mode, last, retryAvailable } = {
      busy,
      error,
      acknowledged,
      prepared,
      mode,
      last,
      retryAvailable,
      ...next,
    });
  }
  function buildCommand() {
    inputs = fields?.captureInputs() ?? inputs;
    return {
      case_id: caseId,
      result: hearingResultCommand(draft, {
        action: 'record',
        operationId: crypto.randomUUID(),
        hearingId: hearing.id,
        resultId,
        anchorRevision: anchor.revision,
        continuation,
      }),
      deadline: {
        operation_id: crypto.randomUUID(),
        deadline_id: deadlineId,
        change: {
          action: 'register',
          expected_revision: 0,
          definition: structuredClone(definition),
          tracking: deadlinePoliciesCommand(policies, definition),
        },
      },
    };
  }
  const actions = createHearingDerivedDeadlineActions({
    read: () => ({
      caseId,
      hearingId: hearing.id,
      user,
      mode,
      busy,
      pending: fieldsBusy,
      disabled,
      blocked,
      closed,
      prepared,
      last,
      acknowledged,
      retryAvailable,
    }),
    update,
    admitted,
    scoped,
    buildCommand,
    finish,
    fail,
  });
  async function initialize() {
    if (pending || !admitted()) return;
    blocked = busy = true;
    error = '';
    acknowledged = retryAvailable = false;
    prepared = null;
    if (mode === 'review') mode = 'draft';
    try {
      let context;
      if (savedDraft && !loaded && recovery) {
        const outcome = await recovery.restore(savedDraft, (value, fresh) => {
          ({
            resultId,
            deadlineId,
            anchor,
            continuation,
            draft,
            definition,
            policies,
            mode,
            last,
            inputs,
          } = value);
          context = fresh;
          loaded = true;
        });
        if (!admitted()) return;
        if (outcome.status !== 'restored')
          throw new Error('No se pudo recuperar el resultado y plazo.');
      } else {
        context = recovery ? await recovery.fresh() : { closed: $administration.closed };
        if (!context || !admitted()) return;
        recovery?.register(resultId, anchor.revision);
        loaded = true;
      }
      restoredClosed = context.closed;
      // A historical compound origin is queried before any current catalog or support.
      if (last && ['uncertain', 'conflict'].includes(mode)) {
        blocked = false;
        return;
      }
      const origin = await readResultDraftSources(
        api,
        hearings,
        caseId,
        anchor,
        continuation,
        admitted,
      );
      if (!origin || !admitted()) return;
      candidates = origin.participants;
      const selected = await refreshHearingReferences(
        draft,
        typed,
        documents,
        caseId,
        admitted,
        true,
      );
      if (!selected || !admitted()) return;
      rows = selected;
      blocked = false;
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  function deny(failure) {
    recovery?.deny(failure);
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    ondenied(failure);
  }
  const reportFailure = createHearingDerivedDeadlineFailure({
    read: () => ({ draft, mode, last }),
    update,
    deny,
    discardSupport: () => recovery?.discardSupport(['provenance', 'support']),
  });
  function fail(failure, writing = false) {
    if (failure.code === 'case_closed') restoredClosed = true;
    reportFailure(failure, writing);
  }
  async function finish(record, recovered = false) {
    recovery?.close();
    finished = true;
    busy = false;
    prepared = null;
    mode = 'confirmed';
    try {
      await onconfirmed(record, recovered);
    } catch {
      if (alive) error = 'Resultado y plazo guardados. Vuelve a consultar la lista sin reenviar.';
    }
  }
  function close() {
    if (pending) return;
    recovery?.close();
    actions.dispose();
    oncancel();
  }
  const canApply = () => admitted() && !blocked && !closed && !disabled && mode === 'draft';
  function supportDenied(failure) {
    if (admitted()) fail(Object.assign(failure, { draftReference: 'support' }));
  }
  onMount(initialize);
  onDestroy(() => {
    alive = false;
    pending = false;
    actions.dispose();
    recovery?.dispose();
    for (const client of [scoped, hearings, participants, typed, documents]) client.dispose();
  });
</script>

<section class="card case-editor" aria-label="Resultado y plazo configurado" aria-busy={pending}>
  <h2>Registrar resultado y plazo</h2>
  <p>Revisa ambos registros antes de confirmarlos juntos.</p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if blocked}<button class="secondary" disabled={pending} onclick={initialize}
      >Volver a consultar el contexto</button
    >{/if}
  <HearingResultSources {anchor} {continuation} />
  {#if last && ['uncertain', 'conflict'].includes(mode)}
    <div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Conservamos el mismo intento. Una ausencia temporal no confirma que el envio fallo.</p>
      <HearingResultValues
        values={last.result.values}
        attendees={last.result.attendees}
        support={last.result.support}
      />
      <p>Plazo solicitado: {last.command.deadline.change.definition.title}</p>
      <button class="primary" disabled={pending || blocked || disabled} onclick={actions.check}
        >Consultar envio exacto</button
      >
      {#if retryAvailable}<button class="secondary" disabled={frozen} onclick={actions.retry}
          >Reintentar el mismo envio</button
        >{/if}
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </div>
  {:else if mode === 'review' && prepared}
    <HearingDerivedDeadlineReview {prepared} />
    <label class="checkbox"
      ><input type="checkbox" bind:checked={acknowledged} disabled={frozen} />Confirmo el resultado
      y el plazo revisados</label
    >
    <div class="action-row">
      <button class="secondary" disabled={pending} onclick={actions.edit}
        >Volver al borrador conjunto</button
      >
      <button class="primary" disabled={frozen || !acknowledged} onclick={actions.submit}
        >Confirmar resultado y plazo</button
      >
    </div>
  {:else if mode === 'draft' && !blocked}
    <HearingDerivedDeadlineFields
      bind:this={fields}
      bind:draft
      bind:rows
      {candidates}
      {caseId}
      {api}
      {participants}
      {typed}
      {documents}
      bind:definition
      bind:policies
      {ondenied}
      {supportDenied}
      {canApply}
      {inputs}
      disabled={frozen}
      bind:pending={fieldsBusy}
      supportContext={() =>
        recovery?.supportContext(
          ['provenance', 'support'],
          canApply,
          (context) => (restoredClosed = context.closed),
          deny,
        )}
      discardSupport={() => recovery?.discardSupport(['provenance', 'support'])}
    />
    <button class="primary" disabled={frozen} onclick={actions.prepare}
      >Preparar resultado y plazo</button
    >
  {/if}
  <button class="text-button" disabled={pending} onclick={close}>Cerrar resultado y plazo</button>
</section>
