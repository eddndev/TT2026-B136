<script>
  import { onDestroy } from 'svelte';
  import FactValues from './FactValues.svelte';
  import FactSources from './FactSources.svelte';
  import CalendarValues from './CalendarValues.svelte';
  import HearingResultValues from './HearingResultValues.svelte';
  import { loadDeadlineReference } from './deadline-reference-data.mjs';
  import { deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  import { deadlineFamilies } from './deadline-view-labels.mjs';
  export let api,
    caseId,
    captured,
    kind = 'source',
    label,
    ondenied;
  let value = null,
    busy = false,
    error = '',
    alive = true,
    generation = 0;
  async function load() {
    const request = ++generation;
    busy = true;
    error = '';
    value = null;
    try {
      const result = await loadDeadlineReference(api, caseId, kind, captured);
      if (alive && request === generation) value = result;
    } catch (failure) {
      if (alive && request === generation) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<article class="case-comparison deadline-reference" aria-label={label}>
  <h4>{label}</h4>
  {#if kind === 'calendar'}<p>{captured.title} / Revisi&#243;n {captured.revision}</p>
  {:else}<p>
      {deadlineFamilies[captured.reference.family]} / Revisi&#243;n {captured.reference.revision}
    </p>
    {#if captured.reference.family === 'notification'}<p>
        Resoluci&#243;n padre: revisi&#243;n {captured.reference.resolution.revision}
      </p>{/if}
    {#if captured.reference.family === 'hearing_result'}<p>
        {captured.reference.agreement_id === null
          ? 'Sin acuerdo espec\u00edfico.'
          : `Acuerdo seleccionado: ${captured.reference.agreement_id}`}
      </p>{/if}
  {/if}
  <p>
    Estado en esta revisi&#243;n: {['published', 'recorded'].includes(captured.status)
      ? 'Registrado'
      : 'Retirado'}.
  </p>
  <button class="secondary" disabled={busy} onclick={load}
    >{busy ? 'Consultando...' : `Consultar ${label.toLowerCase()}`}</button
  >
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if value}{#if kind === 'calendar'}<CalendarValues values={value.values} />
    {:else if captured.reference.family === 'hearing_result'}<HearingResultValues
        values={value.values}
        attendees={value.attendees}
        support={value.support}
      />
    {:else}<FactValues values={value.values} family={captured.reference.family} /><FactSources
        sources={value.sources}
      />{/if}{/if}
  <details>
    <summary>Referencia y huellas conservadas</summary>
    <p>
      <code
        >{kind === 'calendar'
          ? captured.id
          : (captured.reference.id ?? captured.reference.result_id)}</code
      >
    </p>
    {#if captured.reference?.resolution}<p>
        Resoluci&#243;n: <code>{captured.reference.resolution.id}</code>
      </p>{/if}
    <p>Valores: <code>{captured.values_digest}</code></p>
    <p>Recibo: <code>{captured.submission_digest}</code></p>
    {#if captured.sources_digest}<p>Fuentes: <code>{captured.sources_digest}</code></p>{/if}
  </details>
</article>
