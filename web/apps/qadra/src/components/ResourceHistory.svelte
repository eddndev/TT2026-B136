<script>
  import { onMount, onDestroy } from 'svelte';
  import { resourceFailure, resourceDenied } from '../lib/procedural-resource-errors.mjs';
  import { resourceActions, resourceActKinds } from '../lib/procedural-resource-values.mjs';
  export let api,
    id,
    onselect,
    ondenied,
    disabled = false,
    busy = false;
  let rows = [],
    more = false,
    next,
    cursors = [undefined],
    index = 0,
    error = '',
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
        error = resourceFailure(failure);
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

<section class="case-history" aria-label="Historial de recurso" aria-busy={busy}>
  <h3>Historia del recurso y sus actos</h3>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each rows as row (row.revision)}
    <button
      class="case-card fact-row"
      disabled={disabled || busy}
      aria-label={`Consultar recurso revision ${row.revision}`}
      onclick={() => onselect(id, row.revision)}
    >
      <span
        ><strong>Revisi&#243;n {row.revision} / {resourceActions[row.receipt.action]}</strong>
        {#if row.act}<span
            >{resourceActKinds[row.act.values.kind]} / Revisi&#243;n de acto {row.act
              .revision}</span
          >{/if}
        <small>{row.recorded_by.email} / {row.recorded_at}</small></span
      >
    </button>
  {/each}
  {#if busy}<p role="status">Consultando historia...</p>{/if}
  <div class="action-row">
    <button
      class="secondary"
      disabled={disabled || busy || index === 0}
      onclick={() => load(index - 1)}>Revisiones posteriores</button
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
