<script>
  import { onMount, onDestroy } from 'svelte';
  import { resultStatus } from '../lib/hearing-result-errors.mjs';
  import { hearingDenied } from '../lib/hearings.mjs';
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
        rows = beforeRevision ? [...rows, ...value.revisions] : value.revisions;
        more = value.has_more;
        cursor = value.next_before_revision;
      }
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if (hearingDenied(failure)) ondenied(failure);
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

<section class="case-history" aria-label="Historial del resultado" aria-busy={busy}>
  <h3>Historial del registro</h3>
  <p class="hint">
    Cambios de esta captura en orden descendente. Consulta una revisi&#243;n para leer sus valores
    exactos.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={busy || disabled} onclick={() => load()}
      >Consultar historial de resultados de nuevo</button
    >{/if}
  {#each rows as row (row.revision)}<div class="hearing-result-picker-row">
      <p>Revisi&#243;n {row.revision} / {resultStatus[row.status]} / {row.recorded_by.email}</p>
      <p><time datetime={row.recorded_at}>{row.recorded_at}</time></p>
      {#if row.reason}<p class="case-multiline">{row.reason}</p>{/if}
      <button
        class="secondary"
        disabled={busy || disabled}
        onclick={() => onselect(id, row.revision)}
        >Consultar resultado revisi&#243;n {row.revision}</button
      >
    </div>{/each}
  {#if more}<button class="secondary" disabled={busy || disabled} onclick={() => load(cursor)}
      >Cargar cambios anteriores del resultado</button
    >{/if}
</section>
