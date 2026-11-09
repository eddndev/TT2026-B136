<script>
  import Button from './Button.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { precautionaryHearingRecordOverview } from '../lib/precautionary-hearing-record.mjs';
  import { hearingTimeLabel, hearingStatus } from '../lib/hearings.mjs';
  import { factDenied, factFailure } from '../lib/procedural-fact-errors.mjs';
  export let api,
    caseId,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false,
    canApply = () => true;
  const scoped = api.casePrecautionaryHearings(caseId);
  let alive = true,
    rows = [],
    selected = null,
    more = false,
    nextId,
    error = '';
  async function work(fn) {
    if (disabled || busy || !canApply()) return;
    busy = true;
    error = '';
    try {
      const value = await fn();
      if (alive && canApply()) return value;
    } catch (failure) {
      if (alive && canApply()) {
        error = factFailure(failure);
        if (factDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function load(afterId) {
    const result = await work(() => scoped.list({ limit: 10, ...(afterId ? { afterId } : {}) }));
    if (result) {
      rows = result.items;
      more = result.has_more;
      nextId = result.next_after_id;
      selected = null;
    }
  }
  async function choose(capture) {
    const result = await work(() => scoped.exact(precautionaryHearingRecordOverview({ capture })));
    if (result) onselected(result);
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    scoped.dispose();
    busy = false;
  });
</script>

<section class="case-comparison" aria-label="Elegir audiencia cautelar de origen" aria-busy={busy}>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each rows as row}
    <Button variant="secondary" disabled={disabled || busy} onclick={() => (selected = row)}>
      Consultar convocatoria {hearingTimeLabel(
        row.capture.review.resolved_values.scheduled_at,
      )}</Button
    >
  {/each}
  {#if more}<Button variant="secondary" disabled={disabled || busy} onclick={() => load(nextId)}
      >Siguientes convocatorias</Button
    >{/if}
  {#if selected}{#each selected.history.captures as capture}
      <p>
        Revisi&#243;n {capture.review.result_revision} / {hearingStatus[capture.review.status]} /
        {hearingTimeLabel(capture.review.resolved_values.scheduled_at)}
      </p>
      <Button variant="primary" disabled={disabled || busy} onclick={() => choose(capture)}
        >Usar audiencia cautelar revisi&#243;n {capture.review.result_revision}</Button
      >
    {/each}{/if}
  <Button variant="text" disabled={disabled || busy} onclick={oncancel}
    >Cerrar selector de audiencia</Button
  >
</section>
