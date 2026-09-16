<script>
  import { onMount, onDestroy } from 'svelte';
  import HearingValues from './HearingValues.svelte';
  import { hearingDenied, hearingStatus } from '../lib/hearings.mjs';
  export let api,
    id,
    ondenied,
    disabled = false,
    busy = false;
  let rows = [],
    cursor,
    more = false,
    error = '',
    alive = true,
    generation = 0;
  export async function refresh() {
    return load();
  }
  async function load(beforeRevision) {
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const page = await api.history(id, { beforeRevision });
      if (!alive || request !== generation) return;
      rows = beforeRevision ? [...rows, ...page.revisions] : page.revisions;
      cursor = page.next_before_revision;
      more = page.has_more;
    } catch (failure) {
      if (!alive || request !== generation) return;
      error = failure.message;
      if (hearingDenied(failure)) {
        rows = [];
        ondenied(failure);
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

<section class="case-history" aria-label="Historial de audiencia" aria-busy={busy}>
  <h3>Historial de audiencia</h3>
  <p class="hint">
    Revisiones inmutables en orden de registro. La fecha programada puede ser pasada o futura.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={busy || disabled} onclick={() => load()}
      >Consultar historial de nuevo</button
    >{/if}
  {#each rows as row (row.revision)}<details class="hearing-revision">
      <summary
        >Revisi&#243;n {row.revision} / {hearingStatus[row.status]} / {row.recorded_by
          .email}</summary
      >
      <p>Registro: <time datetime={row.recorded_at}>{row.recorded_at}</time></p>
      {#if row.reason}<p class="case-multiline">Motivo: {row.reason}</p>{/if}
      <HearingValues values={row.values} participants={row.participants} support={row.support} />
    </details>{/each}
  {#if busy}<p role="status">Consultando historial...</p>{/if}
  {#if !busy && !rows.length && !error}<p>No hay revisiones en esta consulta.</p>{/if}
  {#if more}<button class="secondary" disabled={busy || disabled} onclick={() => load(cursor)}
      >Cargar revisiones anteriores</button
    >{/if}
</section>
