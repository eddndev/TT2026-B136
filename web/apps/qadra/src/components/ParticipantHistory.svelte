<script>
  import { onMount, onDestroy } from 'svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import ParticipantCredentialEvidence from './ParticipantCredentialEvidence.svelte';
  export let typedApi,
    disabled = false;
  let reading = false,
    evidenceBusy = {};
  $: busy = reading || Object.values(evidenceBusy).some(Boolean);
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
    reading = true;
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
      if (alive) reading = false;
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
      {#if row.credential_origin}<ParticipantCredentialEvidence
          api={typedApi}
          reference={row.credential_origin}
          {ondenied}
          disabled={disabled ||
            reading ||
            Object.entries(evidenceBusy).some(
              ([revision, value]) => Number(revision) !== row.revision && value,
            )}
          bind:busy={evidenceBusy[row.revision]}
        />{/if}
      <p class="hint">Identidad del autor: <code>{row.changed_by.id}</code></p>
    </details>{/each}
  {#if busy}<p class="hint" role="status">Consultando cambios...</p>{/if}
  {#if !busy && !rows.length && !error}<p class="hint">No hay revisiones en esta consulta.</p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" onclick={load}>Volver a consultar historial</button>{/if}
  {#if hasMore}<button class="secondary" disabled={busy || disabled} onclick={load}
      >Cargar cambios anteriores</button
    >{/if}
</section>
