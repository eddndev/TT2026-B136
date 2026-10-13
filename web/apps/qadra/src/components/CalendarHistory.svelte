<script>
  import { onMount, onDestroy } from 'svelte';
  import { calendarStatus, calendarDenied } from '../lib/judicial-calendar-labels.mjs';
  export let api,
    id,
    onselect,
    ondenied,
    disabled = false;
  let rows = [],
    cursor,
    more = false,
    busy = false,
    error = '',
    alive = true,
    generation = 0;
  async function load(beforeRevision) {
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const page = await api.history(id, { limit: 10, beforeRevision });
      if (alive && request === generation) {
        rows = beforeRevision ? [...rows, ...page.revisions] : page.revisions;
        more = page.has_more;
        cursor = page.next_before_revision;
      }
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if (calendarDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<section class="case-history" aria-label="Historial del calendario" aria-busy={busy}>
  <h3>Historial del calendario</h3>
  <p class="hint">
    Revisiones inmutables. Consulta una revisi&#243;n exacta para leer sus fuentes y reglas.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={busy || disabled} onclick={() => load()}
      >Consultar historial de nuevo</button
    >{/if}
  {#each rows as row (row.revision)}<article class="calendar-source-row">
      <h4>Revisi&#243;n {row.revision} / {calendarStatus[row.status]}</h4>
      <p>{row.recorded_by.email} / <time datetime={row.recorded_at}>{row.recorded_at}</time></p>
      {#if row.reason}<p class="case-multiline">{row.reason}</p>{/if}<button
        class="secondary"
        disabled={busy || disabled}
        onclick={() => onselect(id, row.revision)}
        >Consultar calendario revisi&#243;n {row.revision}</button
      >
    </article>{/each}
  {#if more}<button class="secondary" disabled={busy || disabled} onclick={() => load(cursor)}
      >Cargar revisiones anteriores del calendario</button
    >{/if}
</section>
