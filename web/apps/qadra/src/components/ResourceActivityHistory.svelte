<script>
  import { onMount, onDestroy } from 'svelte';
  import { resourceActivityFailure, resourceDenied } from '../lib/resource-activity-errors.mjs';
  import { resourceActivityStatuses } from '../lib/resource-activity-values.mjs';
  export let api,
    id,
    onselect,
    ondenied,
    disabled = false,
    busy = false;
  let rows = [],
    error = '',
    cursors = [undefined],
    index = 0,
    next,
    more = false,
    alive = true;
  async function load(position = 0) {
    busy = true;
    error = '';
    rows = [];
    more = false;
    index = position;
    try {
      const page = await api.history(id, { beforeRevision: cursors[position] });
      if (alive) {
        rows = page.revisions;
        more = page.has_more;
        next = page.next_before_revision;
      }
    } catch (failure) {
      if (alive) {
        error = resourceActivityFailure(failure);
        if (resourceDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    busy = false;
  });
</script>

<section class="case-history" aria-label="Historial de vinculo" aria-busy={busy}>
  <h3>Historia del v&#237;nculo</h3>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each rows as row (row.revision)}
    <button
      class="case-card fact-row"
      disabled={disabled || busy}
      aria-label={`Consultar v\u00ednculo revisi\u00f3n ${row.revision}`}
      onclick={() => onselect(id, row.revision)}
    >
      <span
        ><strong>Revisi&#243;n {row.revision} / {resourceActivityStatuses[row.status]}</strong>
        <small>{row.recorded_by.email} / {row.recorded_at}</small></span
      >
    </button>
  {/each}
  {#if busy}<p role="status">Consultando historia...</p>{/if}
  <div class="action-row">
    <button class="secondary" disabled={disabled || busy || !index} onclick={() => load(index - 1)}
      >Revisiones posteriores</button
    >
    <button
      class="secondary"
      disabled={disabled || busy || !more}
      onclick={() => {
        cursors = [...cursors.slice(0, index + 1), next];
        load(index + 1);
      }}>Revisiones anteriores</button
    >
  </div>
</section>
