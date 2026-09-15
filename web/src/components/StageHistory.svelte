<script>
  import { onMount, onDestroy } from 'svelte';
  import StageEntry from './StageEntry.svelte';
  export let api,
    ondenied,
    disabled = false,
    busy = false;
  let entries = [],
    more = false,
    cursor,
    error = '',
    alive = true;
  export async function refresh() {
    await load();
  }
  async function load(beforeRevision) {
    if (!alive || busy) return;
    busy = true;
    error = '';
    try {
      const result = await api.history({ beforeRevision });
      if (!alive) return;
      entries = beforeRevision ? [...entries, ...result.entries] : result.entries;
      more = result.has_more;
      cursor = result.next_before_revision;
    } catch (failure) {
      if (!alive) return;
      error = failure.message;
      if ([403, 404].includes(failure.status)) {
        entries = [];
        ondenied(failure);
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

<section class="stage-history case-history" aria-busy={busy}>
  <h2>Historial de etapas</h2>
  <p class="hint">
    Registros ordenados por revisi&#243;n del sistema. Las fechas declaradas pueden ser anteriores.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={busy || disabled} onclick={() => load()}
      >Consultar historial de nuevo</button
    >{/if}
  {#if busy}<p role="status">Consultando historial de etapas...</p>{/if}
  {#each entries as record (record.stage_revision)}<StageEntry {record} />{/each}
  {#if !busy && !entries.length && !error}<p>A&#250;n no hay registros de etapa.</p>{/if}
  <p class="hint">{entries.length} registros consultados.</p>
  {#if more}<button class="secondary" disabled={busy || disabled} onclick={() => load(cursor)}
      >Cargar etapas anteriores</button
    >{/if}
</section>
