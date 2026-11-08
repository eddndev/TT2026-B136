<script>
  import Button from './Button.svelte';
  import { onMount, onDestroy } from 'svelte';
  import {
    hearingKinds,
    hearingStatus,
    hearingTimeLabel,
    hearingDenied,
  } from '../lib/hearings.mjs';
  export let api,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false,
    canApply = () => true;
  let roots = [],
    revisions = [],
    selected = null,
    rootCursor,
    historyCursor,
    rootMore = false,
    historyMore = false,
    error = '',
    alive = true,
    generation = 0;
  async function work(fn) {
    if (busy || disabled || !canApply()) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const value = await fn();
      if (alive && canApply() && generation === request) return value;
    } catch (failure) {
      if (alive && canApply() && generation === request) {
        error = failure.message;
        if (hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && generation === request) busy = false;
    }
  }
  async function list(afterId) {
    const value = await work(() => api.list({ status: 'all', afterId, limit: 20 }));
    if (value) {
      roots = value.hearings;
      rootMore = value.has_more;
      rootCursor = value.next_after_id;
      selected = null;
      revisions = [];
    }
  }
  async function history(id, beforeRevision) {
    const value = await work(() => api.history(id, { limit: 20, beforeRevision }));
    if (value) {
      selected = id;
      revisions = value.revisions;
      historyMore = value.has_more;
      historyCursor = value.next_before_revision;
    }
  }
  async function select(row) {
    const value = await work(() => api.revision(row.id, row.revision));
    if (value) onselected(value);
  }
  onMount(() => list());
  onDestroy(() => {
    alive = false;
    generation++;
    busy = false;
  });
</script>

<section
  class="case-comparison"
  aria-label="Elegir programaci&#243;n hist&#243;rica"
  aria-busy={busy}
>
  <h3>Elegir programaci&#243;n hist&#243;rica</h3>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <Button variant="secondary" disabled={busy} onclick={() => list()}
      >Consultar programaciones de nuevo</Button
    >{/if}
  {#each roots as row}<div class="hearing-result-picker-row">
      <p>
        {hearingKinds[row.kind]?.label} / {hearingTimeLabel(row.scheduled_at)} / {hearingStatus[
          row.status
        ]}
      </p>
      <details><summary>Referencia de audiencia</summary><code>{row.id}</code></details>
      <Button
        variant="secondary"
        disabled={busy || disabled}
        aria-label={`Consultar programaci\u00f3n ${row.id}`}
        onclick={() => history(row.id)}>Consultar revisiones de esta programaci&#243;n</Button
      >
    </div>{/each}
  {#if rootMore}<Button
      variant="secondary"
      disabled={busy || disabled}
      onclick={() => list(rootCursor)}>Siguientes programaciones</Button
    >{/if}
  {#if selected}<h4>Revisiones disponibles</h4>
    {#each revisions as row}<div class="hearing-result-picker-row">
        <p>
          Revisi&#243;n {row.revision} / {hearingStatus[row.status]} / {hearingTimeLabel(
            row.values.scheduled_at,
          )}
        </p>
        <Button variant="primary" disabled={busy || disabled} onclick={() => select(row)}
          >Usar programaci&#243;n revisi&#243;n {row.revision}</Button
        >
      </div>{/each}
    {#if historyMore}<Button
        variant="secondary"
        disabled={busy || disabled}
        onclick={() => history(selected, historyCursor)}>Programaciones anteriores</Button
      >{/if}
  {/if}
  <Button variant="text" disabled={busy} onclick={oncancel}
    >Cerrar selector de programaci&#243;n</Button
  >
</section>
