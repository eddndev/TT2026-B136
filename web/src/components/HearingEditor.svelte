<script>
  import { onDestroy } from 'svelte';
  import HearingFields from './HearingFields.svelte';
  import HearingValues from './HearingValues.svelte';
  import HearingReconciliation from './HearingReconciliation.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    hearingDraft,
    hearingCommand,
    hearingKinds,
    hearingDenied,
    hearingUncertain,
  } from '../lib/hearings.mjs';
  import { hearingFailure } from '../lib/hearing-errors.mjs';
  import { readHearingSubmission } from '../lib/hearing-submission.mjs';
  export let api,
    participantsApi,
    typedApi,
    documents,
    caseId,
    user,
    action,
    record,
    initialContext,
    ondenied,
    onconfirmed,
    oncontext,
    oncancel,
    disabled = false,
    pending = false;
  const administration = caseState(),
    hearingId = record?.id || crypto.randomUUID();
  let base = record,
    context = initialContext,
    draft = hearingDraft(record);
  if (!record)
    draft.kind =
      Object.keys(hearingKinds).find((key) => hearingKinds[key].stage === context?.stage) ||
      'initial';
  let rows = (record?.participants || []).map((row) => ({ ...row, retained: true }));
  let prepared = null,
    last = null,
    candidate = null,
    comparedContext = null;
  let mode = 'draft',
    error = '',
    issue = '',
    comparisonReady = false,
    alive = true,
    busy = false,
    fieldsBusy = false;
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled || pending || $administration.closed || ['uncertain', 'exhausted'].includes(mode);
  $: canAccept =
    comparisonReady &&
    mode === 'conflict' &&
    comparedContext?.administrative_status === 'active' &&
    (action === 'schedule'
      ? !candidate
      : candidate?.status === 'scheduled' && candidate.values.kind === base.values.kind);
  function fail(failure, writing = false) {
    if (hearingDenied(failure)) {
      ondenied(failure);
      return;
    }
    error = hearingFailure(failure);
    prepared = null;
    if (writing && (hearingUncertain(failure) || failure.code === 'hearing_operation_conflict')) {
      mode = 'uncertain';
      error =
        'No se pudo confirmar el resultado. Conservamos el env\u00edo para consultar su recibo exacto.';
    } else if (failure.code === 'hearing_revision_exhausted') mode = 'exhausted';
    else if (
      [
        'hearing_revision_conflict',
        'hearing_context_conflict',
        'hearing_already_cancelled',
        'hearing_submission_mismatch',
        'hearing_stage_incompatible',
        'hearing_context_required',
      ].includes(failure.code)
    ) {
      mode = 'conflict';
      comparisonReady = false;
    } else {
      mode = 'draft';
      if (failure.code === 'hearing_participant_changed') issue = 'participants';
      if (['hearing_support_changed', 'hearing_support_digest_mismatch'].includes(failure.code))
        issue = 'support';
    }
  }
  async function prepare() {
    if (frozen || issue || mode !== 'draft') return;
    busy = true;
    error = '';
    try {
      const command = hearingCommand(draft, context, base, action, crypto.randomUUID(), hearingId);
      const result = await api.prepare(command, user.id);
      if (alive) {
        prepared = structuredClone(result);
        mode = 'review';
      }
    } catch (failure) {
      if (alive) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(result, exact = false) {
    mode = 'confirmed';
    prepared = null;
    try {
      await onconfirmed(result, exact);
    } catch {
      if (alive)
        error =
          'La audiencia se guard\u00f3. No se pudieron actualizar todas las consultas; vuelve a consultar sin reenviar.';
    }
  }
  async function submit() {
    if (!prepared || frozen || mode !== 'review') return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    let result;
    try {
      result = await api.submit(last);
    } catch (failure) {
      if (alive) {
        fail(failure, true);
        busy = false;
      }
      return;
    }
    if (alive) {
      await finish(result);
      if (alive) busy = false;
    }
  }
  async function check() {
    if (pending || !last) return;
    busy = true;
    error = '';
    try {
      const result = await readHearingSubmission(api, last);
      if (!alive) return;
      if (result.state === 'matched') await finish(result.record, true);
      else if (result.state === 'absent')
        error =
          'La revisi\u00f3n a\u00fan no est\u00e1 disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = result.record;
        comparisonReady = false;
        error = 'La revisi\u00f3n pertenece a otro env\u00edo. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (alive) {
        error = hearingFailure(failure);
        if (hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function refresh() {
    if (pending) return;
    busy = true;
    error = '';
    comparisonReady = false;
    try {
      const nextContext = await api.context();
      let current = null;
      try {
        current = await api.get(hearingId);
      } catch (failure) {
        if (failure.code !== 'hearing_not_found' || failure.status !== 404) throw failure;
      }
      if (!alive) return;
      comparedContext = nextContext;
      candidate = current;
      comparisonReady = true;
      oncontext(nextContext);
    } catch (failure) {
      if (alive) {
        error = hearingFailure(failure);
        if (hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (!canAccept || pending) return;
    base = candidate;
    context = comparedContext;
    rows = rows.map((row) => ({
      ...row,
      retained: !!base?.values.participants.some(
        (ref) => ref.participant_id === row.id && ref.revision === row.revision,
      ),
    }));
    prepared = null;
    comparisonReady = false;
    mode = 'draft';
    error = '';
  }
  onDestroy(() => {
    alive = false;
    pending = false;
  });
</script>

<section
  class="card case-editor hearing-editor"
  aria-label="Formulario de audiencia"
  aria-busy={pending}
>
  <h2>
    {action === 'schedule'
      ? 'Programar audiencia'
      : action === 'replace'
        ? 'Corregir o reprogramar audiencia'
        : 'Cancelar audiencia'}
  </h2>
  <p class="hint">
    La programaci&#243;n conserva lo declarado. No registra celebraci&#243;n, asistencia ni
    resultados.
  </p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if issue}<p class="notice">
      {issue === 'participants'
        ? 'Quita o vuelve a seleccionar las fichas que requieren revisi\u00f3n antes de preparar.'
        : 'Elige y consulta de nuevo el soporte exacto antes de preparar.'}
    </p>{/if}
  {#if ['conflict', 'uncertain'].includes(mode)}<HearingReconciliation
      {mode}
      {last}
      {candidate}
      context={comparedContext}
      ready={comparisonReady}
      {canAccept}
      busy={pending}
      oncheck={check}
      onrefresh={refresh}
      onaccept={accept}
    />{/if}
  {#if prepared}<div class="case-comparison hearing-preview">
      <h3>Confirma el registro</h3>
      <p>
        Revisi&#243;n esperada {prepared.command.change.expected_revision} / Revisi&#243;n a registrar
        {prepared.result_revision}.
      </p>
      <HearingValues values={prepared.values} participants={rows} support={draft.support} />
      {#if prepared.command.change.reason}<p class="case-multiline">
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <div class="action-row">
        <button
          class="secondary"
          disabled={pending}
          onclick={() => {
            prepared = null;
            mode = 'draft';
          }}>Volver al borrador</button
        >
        <button class="primary" disabled={frozen} onclick={submit}
          >{busy ? 'Registrando audiencia...' : 'Confirmar registro'}</button
        >
      </div>
    </div>
  {:else if mode !== 'confirmed'}
    {#if action === 'cancel'}<HearingValues
        values={base.values}
        participants={rows}
        support={base.support}
      />
      <label
        >Motivo de cancelaci&#243;n<textarea rows="3" bind:value={draft.reason} disabled={frozen}
        ></textarea></label
      >
      <p class="hint">
        Se conserva la programaci&#243;n y su historia. La cancelaci&#243;n organizativa no declara
        nulidad procesal.
      </p>
    {:else}<HearingFields
        bind:draft
        bind:rows
        {base}
        {context}
        api={participantsApi}
        {typedApi}
        {documents}
        {caseId}
        {ondenied}
        disabled={disabled ||
          busy ||
          $administration.closed ||
          ['uncertain', 'exhausted'].includes(mode)}
        bind:pending={fieldsBusy}
        onreviewed={(kind) => {
          if (issue === kind) issue = '';
        }}
      />{/if}
    <button class="primary" disabled={frozen || mode !== 'draft' || !!issue} onclick={prepare}
      >Revisar registro</button
    >
  {/if}
  {#if mode === 'uncertain'}<p class="hint">
      Cerrar el formulario descarta su borrador local; no cancela un env&#237;o que pueda estar en
      proceso.
    </p>{/if}
  <button class="text-button" disabled={pending} onclick={oncancel}
    >Cerrar formulario de audiencia</button
  >
</section>
