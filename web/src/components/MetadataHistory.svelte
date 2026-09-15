<script>
  import { onMount, onDestroy } from 'svelte';
  import MetadataSummary from './MetadataSummary.svelte';
  export let api;
  export let ondenied;
  let rows = [];
  let hasMore = false;
  let before;
  let busy = false;
  let error = '';
  let alive = true;
  async function load() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const page = await api.history({ beforeRevision: before });
      if (!alive) return;
      rows = [...rows, ...page.revisions];
      hasMore = page.has_more;
      before = page.next_before_revision;
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  onMount(load);
  onDestroy(() => {
    alive = false;
  });
</script>

<section class="metadata-history" aria-label="Historial de clasificaci&#243;n" aria-busy={busy}>
  <h3>Historial de clasificaci&#243;n</h3>
  {#each rows as row}<details class="metadata-revision">
      <summary
        ><span class="metadata-change-heading"
          ><strong>Cambio {row.metadata_revision}</strong><span>{row.changed_by.email}</span><time
            datetime={row.changed_at}
            title={row.changed_at}>{new Date(row.changed_at).toLocaleString('es-MX')}</time
          ></span
        ></summary
      >
      <MetadataSummary metadata={row} />
      <p class="hint">Identidad del autor: <code>{row.changed_by.id}</code></p>
    </details>{/each}
  {#if busy}<p class="hint" role="status">Consultando cambios...</p>{/if}
  {#if !busy && !rows.length && !error}<p class="hint">
      A&#250;n no hay cambios de clasificaci&#243;n registrados.
    </p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" onclick={load}>Volver a consultar historial</button>{/if}
  {#if hasMore}<button class="secondary" disabled={busy} onclick={load}
      >Cargar cambios anteriores</button
    >{/if}
</section>
