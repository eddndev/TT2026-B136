<script>
  import { onMount, onDestroy } from 'svelte';
  import { hearingKinds, hearingTimeLabel } from '../lib/hearings.mjs';
  import { hearingResultTimeLabel } from '../lib/hearing-result-time.mjs';
  export let api,
    caseId,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  const hearingsApi = api.caseHearings(caseId);
  let resultsApi = null,
    hearings = [],
    results = [],
    revisions = [],
    hearingId = null,
    resultId = null;
  let hearingMore = false,
    resultMore = false,
    historyMore = false,
    hearingCursor,
    resultCursor,
    historyCursor;
  let exact = null,
    agreement = '',
    error = '',
    alive = true;
  async function work(task) {
    if (busy || disabled) return;
    busy = true;
    error = '';
    try {
      const value = await task();
      if (alive) return value;
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([401, 403].includes(failure.status) || failure.code === 'case_not_found')
          ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function listHearings(afterId) {
    const page = await work(() => hearingsApi.list({ status: 'all', limit: 20, afterId }));
    if (page) {
      hearings = page.hearings;
      hearingMore = page.has_more;
      hearingCursor = page.next_after_id;
    }
  }
  async function listResults(id, afterId) {
    if (busy || disabled) return;
    if (hearingId !== id) {
      resultsApi?.dispose();
      resultsApi = api.caseHearingResults(caseId, id);
      hearingId = id;
      results = [];
      resultMore = false;
    }
    exact = null;
    revisions = [];
    resultId = null;
    const page = await work(() => resultsApi.list({ status: 'all', limit: 20, afterId }));
    if (page) {
      results = page.results;
      resultMore = page.has_more;
      resultCursor = page.next_after_id;
    }
  }
  async function history(id, beforeRevision) {
    exact = null;
    const page = await work(() => resultsApi.history(id, { limit: 10, beforeRevision }));
    if (page) {
      resultId = id;
      revisions = page.revisions;
      historyMore = page.has_more;
      historyCursor = page.next_before_revision;
    }
  }
  async function read(revision) {
    exact = null;
    const row = await work(() => resultsApi.revision(resultId, revision));
    if (row) {
      exact = row;
      agreement = '';
    }
  }
  function select() {
    if (!exact || busy || disabled) return;
    onselected({
      reference: {
        hearing_id: exact.hearing_id,
        result_id: exact.id,
        revision: exact.revision,
        agreement_id: agreement || null,
      },
      record: exact,
    });
  }
  onMount(() => listHearings());
  onDestroy(() => {
    alive = false;
    hearingsApi.dispose();
    resultsApi?.dispose();
    busy = false;
  });
</script>

<section class="case-comparison" aria-label="Elegir resultado hist&#243;rico" aria-busy={busy}>
  <h4>Elegir resultado hist&#243;rico</h4>
  <p class="hint">
    Consulta una audiencia, su resultado y la revisi&#243;n exacta. Los registros retirados siguen
    disponibles como antecedentes.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <h5>Audiencias del expediente</h5>
  {#each hearings as row}<div class="hearing-result-picker-row">
      <p>
        {hearingKinds[row.kind]?.label || 'Audiencia'} / {hearingTimeLabel(row.scheduled_at)} / {row.status ===
        'cancelled'
          ? 'Cancelada'
          : 'Programada'}
      </p>
      <details><summary>Identidad de la audiencia</summary><code>{row.id}</code></details>
      <button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => listResults(row.id)}
        aria-label={`Consultar resultados de audiencia ${row.id}`}>Consultar resultados</button
      >
    </div>{/each}
  {#if !hearings.length && !busy}<p>No hay audiencias en esta p&#225;gina.</p>{/if}
  {#if hearingMore}<button
      type="button"
      class="secondary"
      disabled={disabled || busy}
      onclick={() => listHearings(hearingCursor)}>Siguientes audiencias</button
    >{/if}
  {#if hearingId}<h5>Resultados de la audiencia seleccionada</h5>
    {#each results as row}<div class="hearing-result-picker-row">
        <p>
          {hearingResultTimeLabel(row.event_time)} / {row.status === 'withdrawn'
            ? 'Retirado'
            : 'Registrado'} / Revision {row.revision}
        </p>
        <details><summary>Identidad del resultado</summary><code>{row.id}</code></details>
        <button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => history(row.id)}
          aria-label={`Consultar revisiones de resultado ${row.id}`}
          >Consultar revisiones del resultado</button
        >
      </div>{/each}
    {#if !results.length && !busy}<p>No hay resultados en esta p&#225;gina.</p>{/if}
    {#if resultMore}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => listResults(hearingId, resultCursor)}>Siguientes resultados</button
      >{/if}
  {/if}
  {#if resultId}<h5>Revisiones del resultado seleccionado</h5>
    {#each revisions as row}<div class="hearing-result-picker-row">
        <p>Revision {row.revision} / {row.status === 'withdrawn' ? 'Retirado' : 'Registrado'}</p>
        <button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => read(row.revision)}
          >Consultar resultado revisi&#243;n {row.revision}</button
        >
      </div>{/each}
    {#if historyMore}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => history(resultId, historyCursor)}>Revisiones anteriores del resultado</button
      >{/if}
  {/if}
  {#if exact}<div class="case-comparison">
      <p>
        Revision exacta {exact.revision} / {exact.status === 'withdrawn'
          ? 'Retirado'
          : 'Registrado'}
      </p>
      <p>{hearingResultTimeLabel(exact.values.event_time)}</p>
      <p class="case-multiline">{exact.values.summary}</p>
      <label
        >Acuerdo de origen<select bind:value={agreement} disabled={disabled || busy}>
          <option value="">Resultado completo, sin acuerdo espec&#237;fico</option>
          {#each exact.values.agreements as row, index}<option value={row.id}
              >Acuerdo {index + 1}: {row.text}</option
            >{/each}
        </select></label
      >
      <button type="button" class="primary" disabled={disabled || busy} onclick={select}
        >Vincular este resultado exacto</button
      >
    </div>{/if}
  <button type="button" class="text-button" disabled={busy} onclick={oncancel}
    >Cerrar selector de resultados</button
  >
</section>
