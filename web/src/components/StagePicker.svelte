<script>
  import { onMount, onDestroy } from 'svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  export let api,
    caseId,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  let rows = [],
    versions = [],
    document = null,
    exact = null,
    more = false,
    versionMore = false;
  let offset = 0,
    cursor,
    name = '',
    query = '',
    error = '',
    alive = true;
  async function work(operation) {
    if (busy || disabled || !alive) return;
    busy = true;
    error = '';
    try {
      await operation();
    } catch (failure) {
      if (!alive) return;
      error = failure.message;
      if ([403, 404].includes(failure.status)) {
        rows = [];
        versions = [];
        exact = null;
        ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  const list = (next = 0) =>
    work(async () => {
      const result = await api.list({ limit: 20, offset: next, name: query || undefined });
      if (!alive) return;
      rows = result.documents;
      more = result.has_more;
      offset = next;
      document = null;
      exact = null;
    });
  const history = (record, beforeVersion) =>
    work(async () => {
      const result = await api.versions(record.id, { limit: 20, beforeVersion });
      if (!alive) return;
      document = record;
      exact = null;
      versions = beforeVersion ? [...versions, ...result.versions] : result.versions;
      versionMore = result.has_more;
      cursor = result.next_before_version;
    });
  const select = (record) =>
    work(async () => {
      exact = null;
      const scoped = api.version(record.id, record.version);
      try {
        const result = await scoped.detail();
        if (!alive) return;
        if (
          result.case_id !== caseId ||
          result.id !== record.id ||
          result.version !== record.version
        )
          throw new Error('La respuesta no corresponde a la versi\u00f3n elegida.');
        exact = result;
      } finally {
        scoped.dispose();
      }
    });
  onMount(() => list());
  onDestroy(() => {
    alive = false;
    busy = false;
  });
</script>

<section
  class="case-comparison stage-picker"
  role="region"
  aria-label="Seleccionar soporte exacto"
  aria-busy={busy}
>
  <h4>Seleccionar soporte exacto</h4>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if busy}<p class="hint" role="status">Consultando soporte...</p>{/if}
  {#if !document}
    <div class="stage-search">
      <label>Nombre del soporte<input bind:value={name} disabled={busy || disabled} /></label>
      <button
        type="button"
        class="secondary"
        disabled={busy || disabled}
        onclick={() => {
          query = name.trim();
          list();
        }}>Buscar soporte</button
      >
    </div>
    <div class="stage-pick-list">
      {#each rows as row (row.id)}<button
          type="button"
          class="case-card"
          disabled={busy || disabled}
          onclick={() => history(row)}>{row.name} / versi&#243;n actual {row.version}</button
        >{/each}
    </div>
    {#if !busy && !rows.length}<p>No hay documentos en esta p&#225;gina.</p>{/if}
    <div class="action-row">
      {#if offset}<button
          type="button"
          class="secondary"
          disabled={busy || disabled}
          onclick={() => list(Math.max(0, offset - 20))}>Documentos anteriores</button
        >{/if}
      {#if more}<button
          type="button"
          class="secondary"
          disabled={busy || disabled}
          onclick={() => list(offset + 20)}>Documentos siguientes</button
        >{/if}
    </div>
  {:else}
    <p>Versiones de {document.name}</p>
    <div class="stage-pick-list">
      {#each versions as row (row.version)}<button
          type="button"
          class="case-card"
          disabled={busy || disabled}
          onclick={() => select(row)}
          >Versi&#243;n {row.version} / {row.name} / {row.version === document.version
            ? 'Actual al consultar'
            : 'Hist\u00f3rica'}</button
        >{/each}
    </div>
    {#if versionMore}<button
        type="button"
        class="secondary"
        disabled={busy || disabled}
        onclick={() => history(document, cursor)}>Cargar versiones anteriores</button
      >{/if}
    {#if exact}<StageSupportSummary record={exact} /><button
        type="button"
        class="primary"
        disabled={busy || disabled}
        onclick={() => onselected(exact)}>Usar esta versi&#243;n</button
      >{/if}
    <button
      type="button"
      class="text-button"
      disabled={busy || disabled}
      onclick={() => {
        document = null;
        exact = null;
      }}>Elegir otro documento</button
    >
  {/if}
  <button type="button" class="text-button" disabled={busy || disabled} onclick={oncancel}
    >Cerrar selector</button
  >
</section>
