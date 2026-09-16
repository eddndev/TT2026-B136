<script>
  import { onDestroy } from 'svelte';
  import HearingResultSources from './HearingResultSources.svelte';
  import HearingResultFields from './HearingResultFields.svelte';
  import HearingResultValues from './HearingResultValues.svelte';
  import HearingAnchorPicker from './HearingAnchorPicker.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { hearingResultDraft, hearingResultCommand } from '../lib/hearing-result-values.mjs';
  import { hearingResultFailure } from '../lib/hearing-result-errors.mjs';
  import { readHearingResultSubmission } from '../lib/hearing-result-submission.mjs';
  import { hearingDenied, hearingUncertain } from '../lib/hearings.mjs';
  export let api,
    caseId,
    user,
    hearing,
    action,
    record = null,
    continuation = null,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false;
  const administration = caseState(),
    resultId = record?.id || crypto.randomUUID();
  const hearings = api.caseHearings(caseId),
    participants = api.caseParticipants(caseId),
    typed = api.caseTypedParticipants(caseId),
    documents = api.caseDocuments(caseId);
  const anchorOf = (row) => ({
    hearing_id: row.id,
    revision: row.revision,
    values_digest: row.values_digest,
    submission_digest: row.receipt.submission_digest,
    status: row.status,
    kind: row.values.kind,
    scheduled_at: row.values.scheduled_at,
    scheduling_context: row.scheduling_context,
  });
  let anchor = record?.anchor || anchorOf(hearing),
    base = record,
    draft = hearingResultDraft(record),
    rows = structuredClone(record?.attendees || []);
  let candidates = hearing.participants,
    source = record?.continuation || continuation;
  let scoped = api.caseHearingResults(caseId, anchor.hearing_id),
    alive = true,
    busy = false,
    fieldsBusy = false,
    pickerBusy = false,
    picking = false;
  let mode = 'draft',
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    error = '';
  $: pending = busy || fieldsBusy || pickerBusy;
  $: frozen =
    disabled || pending || $administration.closed || ['uncertain', 'exhausted'].includes(mode);
  function fail(failure, writing = false) {
    if (hearingDenied(failure)) {
      ondenied(failure);
      return;
    }
    error = hearingResultFailure(failure);
    prepared = null;
    if (
      writing &&
      (hearingUncertain(failure) || failure.code === 'hearing_result_operation_conflict')
    ) {
      mode = 'uncertain';
      error =
        'No se pudo confirmar el resultado. Conservamos el env\u00edo para consultar su revisi\u00f3n exacta.';
    } else if (failure.code === 'hearing_result_revision_exhausted') mode = 'exhausted';
    else if (
      [
        'hearing_result_revision_conflict',
        'hearing_result_already_withdrawn',
        'hearing_result_not_found',
      ].includes(failure.code)
    ) {
      mode = 'conflict';
      compared = false;
    } else mode = 'draft';
  }
  function selectAnchor(value) {
    anchor = anchorOf(value);
    candidates = value.participants;
    picking = false;
    scoped.dispose();
    scoped = api.caseHearingResults(caseId, anchor.hearing_id);
    prepared = null;
  }
  async function prepare() {
    if (frozen || mode !== 'draft') return;
    busy = true;
    error = '';
    try {
      const command = hearingResultCommand(draft, {
        action,
        base,
        operationId: crypto.randomUUID(),
        hearingId: anchor.hearing_id,
        resultId,
        anchorRevision: anchor.revision,
        continuation: source,
      });
      const value = await scoped.prepare(command, user.id);
      if (alive) {
        prepared = structuredClone(value);
        mode = 'review';
      }
    } catch (failure) {
      if (alive) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(value, exact = false) {
    prepared = null;
    mode = 'confirmed';
    try {
      await onconfirmed(value, exact);
    } catch {
      if (alive)
        error =
          'El registro se guard\u00f3. No se actualizaron todas las consultas; vuelve a consultar sin reenviar.';
    }
  }
  async function submit() {
    if (frozen || !prepared || mode !== 'review') return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    try {
      const value = await scoped.submit(last);
      if (alive) await finish(value);
    } catch (failure) {
      if (alive) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (pending || !last) return;
    busy = true;
    error = '';
    try {
      const value = await readHearingResultSubmission(scoped, last);
      if (!alive) return;
      if (value.state === 'matched') await finish(value.record, true);
      else if (value.state === 'absent')
        error =
          'La revisi\u00f3n a\u00fan no est\u00e1 disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = value.record;
        compared = false;
        error = 'La revisi\u00f3n corresponde a otro env\u00edo. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (alive) {
        error = hearingResultFailure(failure);
        if (hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending) return;
    busy = true;
    error = '';
    compared = false;
    try {
      const value = await scoped.get(resultId);
      if (alive) {
        candidate = value;
        compared = true;
      }
    } catch (failure) {
      if (alive) {
        error = hearingResultFailure(failure);
        if (hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (!compared || !candidate || candidate.status !== 'recorded' || action === 'record' || frozen)
      return;
    base = candidate;
    mode = 'draft';
    compared = false;
    error = '';
  }
  onDestroy(() => {
    alive = false;
    pending = false;
    scoped.dispose();
    hearings.dispose();
    participants.dispose();
    typed.dispose();
    documents.dispose();
  });
</script>

<section
  class="card case-editor hearing-result-editor"
  aria-label="Formulario de sesi&#243;n o acto"
  aria-busy={pending}
>
  <h2>
    {action === 'record'
      ? source
        ? 'Registrar continuaci\u00f3n'
        : 'Registrar sesi\u00f3n o acto'
      : action === 'correct'
        ? 'Rectificar registro'
        : 'Retirar registro'}
  </h2>
  <p class="hint">
    Conserva lo comunicado con sus fuentes. Esta captura no declara notificaci&#243;n, firmeza ni
    plazos.
  </p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <HearingResultSources {anchor} continuation={source} />
  {#if action === 'record' && mode === 'draft'}<button
      class="secondary"
      disabled={frozen}
      onclick={() => (picking = true)}>Elegir programaci&#243;n de origen</button
    >{/if}
  {#if picking}<HearingAnchorPicker
      api={hearings}
      {ondenied}
      disabled={disabled || busy || fieldsBusy || $administration.closed}
      bind:busy={pickerBusy}
      onselected={selectAnchor}
      oncancel={() => (picking = false)}
    />{/if}
  {#if mode === 'uncertain'}<div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. Una ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={pending} onclick={check}>Consultar env&#237;o exacto</button
      >
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </div>{/if}
  {#if mode === 'conflict'}<div class="case-comparison">
      <h3>Comparar con el registro actual</h3>
      <button class="secondary" disabled={pending} onclick={compare}
        >Consultar base actual del resultado</button
      >
      {#if compared && candidate}<p>
          Base consultada: revisi&#243;n {candidate.revision} / {candidate.status === 'withdrawn'
            ? 'Registro retirado'
            : 'Registrado'}
        </p>
        <HearingResultValues
          values={candidate.values}
          attendees={candidate.attendees}
          support={candidate.support}
        />
        {#if action !== 'record' && candidate.status === 'recorded'}<button
            class="primary"
            disabled={frozen}
            onclick={accept}>Usar esta base y conservar borrador</button
          >{:else}<p>
            Esta captura no permite reenviar el borrador sobre la base consultada. Puedes cerrar y
            revisar sus fuentes.
          </p>{/if}
      {/if}
    </div>{/if}
  {#if prepared}<div class="case-comparison">
      <h3>Revisa el resultado a registrar</h3>
      <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
      <HearingResultValues
        values={prepared.values}
        attendees={prepared.attendees}
        support={prepared.support}
      />
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
          }}>Volver al borrador del resultado</button
        ><button class="primary" disabled={frozen} onclick={submit}>Confirmar resultado</button>
      </div>
    </div>{:else if mode !== 'confirmed'}
    {#if action === 'withdraw'}<HearingResultValues
        values={base.values}
        attendees={base.attendees}
        support={base.support}
      />
      <p class="notice">Retirar conserva la captura y su historia; no anula el acto informado.</p>
    {:else}<HearingResultFields
        bind:draft
        bind:rows
        {candidates}
        {caseId}
        {participants}
        {typed}
        {documents}
        {ondenied}
        disabled={disabled ||
          busy ||
          pickerBusy ||
          $administration.closed ||
          ['uncertain', 'exhausted'].includes(mode)}
        bind:pending={fieldsBusy}
      />{/if}
    {#if action !== 'record'}<label
        >Motivo<textarea rows="3" bind:value={draft.reason} disabled={frozen}></textarea></label
      >{/if}
    <button class="primary" disabled={frozen || mode !== 'draft' || picking} onclick={prepare}
      >Revisar resultado</button
    >
  {/if}
  <button class="text-button" disabled={pending} onclick={oncancel}
    >Cerrar formulario de resultado</button
  >
</section>
