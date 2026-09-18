<script>
  import { onMount, onDestroy } from 'svelte';
  import { deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { deadlineActions } from './deadline-view-labels.mjs';
  export let api,
    id,
    onselect,
    ondenied,
    disabled = false,
    busy = false;
  let rows = [],
    more = false,
    cursor,
    error = '',
    alive = true,
    generation = 0;
  async function load(beforeRevision) {
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const value = await api.history(id, { limit: 10, beforeRevision });
      if (alive && request === generation) {
        rows = beforeRevision === undefined ? value.revisions : [...rows, ...value.revisions];
        more = value.has_more;
        cursor = value.next_before_revision;
      }
    } catch (failure) {
      if (alive && request === generation) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) {
          rows = [];
          ondenied(failure);
        }
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
    busy = false;
  });
</script>

<section class="case-history" aria-label="Historial de plazo" aria-busy={busy}>
  <h3>Historial del plazo</h3>
  <p class="hint">Cada revisi&#243;n conserva sus datos y su c&#225;lculo original.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={busy || disabled} onclick={() => load()}
      >Consultar historial de nuevo</button
    >{/if}
  {#each rows as row (row.revision)}<div class="fact-history-row">
      <p><strong>Revisi&#243;n {row.revision}</strong> / {deadlineActions[row.receipt.action]}</p>
      <p>{row.recorded_by.email} / {deadlineInstantLabel(row.recorded_at)}</p>
      {#if row.reason}<p class="case-multiline">{row.reason}</p>{/if}
      <button
        class="secondary"
        disabled={busy || disabled}
        onclick={() => onselect(id, row.revision)}
        >Consultar plazo revisi&#243;n {row.revision}</button
      >
    </div>{/each}
  {#if more}<button class="secondary" disabled={busy || disabled} onclick={() => load(cursor)}
      >Cargar cambios anteriores</button
    >{/if}
</section>
