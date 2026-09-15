<script>
  import { onMount, onDestroy } from 'svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  export let api;
  export let id;
  export let ondenied;
  export let busy = false;
  let rows = [],
    hasMore = false,
    before,
    error = '',
    alive = true;
  let pending;
  export async function refresh() {
    await pending;
    if (alive) return load(false);
  }
  function load(more = true) {
    if (pending) return pending;
    pending = read(more).finally(() => {
      pending = undefined;
    });
    return pending;
  }
  async function read(more) {
    busy = true;
    error = '';
    if (!more) {
      rows = [];
      before = undefined;
      hasMore = false;
    }
    try {
      const page = await api.history(id, { beforeRevision: before });
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
  onMount(() => {
    load(false);
  });
  onDestroy(() => {
    alive = false;
  });
</script>

<section class="participant-history" aria-label="Historial del participante" aria-busy={busy}>
  <h3>Historial de cambios</h3>
  {#each rows as row}<details class="participant-revision">
      <summary
        ><span
          ><strong>Cambio {row.revision}</strong> <span>{row.changed_by.email}</span>
          <time datetime={row.changed_at} title={row.changed_at}
            >{new Date(row.changed_at).toLocaleString('es-MX')}</time
          ></span
        ></summary
      >
      <ParticipantSummary record={row} />
      <p class="hint">Identidad del autor: <code>{row.changed_by.id}</code></p>
    </details>{/each}
  {#if busy}<p class="hint" role="status">Consultando cambios...</p>{/if}
  {#if !busy && !rows.length && !error}<p class="hint">No hay revisiones en esta consulta.</p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" onclick={load}>Volver a consultar historial</button>{/if}
  {#if hasMore}<button class="secondary" disabled={busy} onclick={load}
      >Cargar cambios anteriores</button
    >{/if}
</section>
