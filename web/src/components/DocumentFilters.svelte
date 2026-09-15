<script>
  import Icon from './Icon.svelte';
  import { metadataFilters } from '../lib/document-metadata.mjs';
  export let onapply;
  export let layout = 'list';
  export let filter = 'all';
  let search = '';
  let documentType = '';
  let classification = '';
  let tag = '';
  let error = '';
  function apply(event) {
    event?.preventDefault();
    try {
      const metadata = metadataFilters({ document_type: documentType, classification, tag });
      error = '';
      onapply({ name: search.trim(), filter, ...metadata });
    } catch (failure) {
      error = failure.message;
    }
  }
  export function clear() {
    search = '';
    documentType = '';
    classification = '';
    tag = '';
    filter = 'all';
    error = '';
    onapply({ name: '', filter });
  }
</script>

<div class="list-toolbar">
  <form class="server-search" onsubmit={apply}>
    <label class="search-field"
      ><Icon name="search" size={18} /><input
        aria-label="Buscar por nombre"
        placeholder="Buscar por nombre..."
        bind:value={search}
      /></label
    ><button class="secondary">Buscar</button>
  </form>
  <select aria-label="Filtrar por estado" bind:value={filter} onchange={apply}
    ><option value="all">Todos los estados</option><option value="pending"
      >Pendientes de sello</option
    ><option value="sealed">Sellados</option></select
  >
  <div class="view-switch" aria-label="Presentaci&#243;n del listado">
    <button
      class="icon-button"
      class:active={layout === 'list'}
      aria-label="Vista de lista"
      aria-pressed={layout === 'list'}
      onclick={() => (layout = 'list')}><Icon name="list" size={18} /></button
    ><button
      class="icon-button"
      class:active={layout === 'grid'}
      aria-label="Vista de tarjetas"
      aria-pressed={layout === 'grid'}
      onclick={() => (layout = 'grid')}><Icon name="grid" size={18} /></button
    >
  </div>
</div>
<details class="metadata-filters">
  <summary>Filtros de clasificaci&#243;n</summary>
  <form onsubmit={apply}>
    <div class="metadata-filter-fields">
      <label>Tipo exacto<input bind:value={documentType} /></label><label
        >Clasificaci&#243;n exacta<input bind:value={classification} /></label
      ><label>Etiqueta exacta<input bind:value={tag} /></label>
    </div>
    <p class="hint">
      Coinciden con el valor completo y distinguen may&#250;sculas y acentos. Una coma forma parte
      de la etiqueta.
    </p>
    <div class="action-row">
      <button class="secondary">Aplicar filtros</button><button
        type="button"
        class="text-button"
        onclick={clear}>Limpiar filtros</button
      >
    </div>
  </form>
</details>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
