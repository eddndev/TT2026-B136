<script>
  import { onMount, onDestroy } from 'svelte';
  import MetadataSummary from './MetadataSummary.svelte';
  import MetadataEditor from './MetadataEditor.svelte';
  import MetadataHistory from './MetadataHistory.svelte';
  import { can } from '../lib/documents.mjs';
  export let api;
  export let user;
  export let document;
  export let onmetadata;
  export let ondenied;
  const scoped = api.metadata(document.id);
  let current = document.current_metadata || {
    metadata_revision: 0,
    document_type: null,
    classification: null,
    tags: [],
  };
  let editor;
  let showHistory = false;
  let busy = false;
  let error = '';
  let alive = true;
  let generation = 0;
  function merge(record) {
    if (!alive || record.metadata_revision < current.metadata_revision) return;
    current = record;
    onmetadata(record);
  }
  function confirmed(record) {
    generation++;
    busy = false;
    merge(record);
  }
  $: if (document.current_metadata?.metadata_revision > current.metadata_revision)
    confirmed(document.current_metadata);
  async function load() {
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const record = await scoped.get();
      if (alive && request === generation) merge(record);
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onMount(load);
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
  });
</script>

<section
  class="card document-metadata"
  aria-label="Clasificaci&#243;n actual del documento"
  aria-busy={busy}
>
  <div class="section-heading">
    <div>
      <span class="eyebrow">ORGANIZACI&#211;N DEL DOCUMENTO</span>
      <h2>Clasificaci&#243;n actual del documento</h2>
    </div>
    {#if can(user.role, 'classify')}<button class="secondary" onclick={() => editor.open()}
        >Editar clasificaci&#243;n</button
      >{/if}
  </div>
  <p class="hint">
    Organiza este documento. Los archivos y sus evidencias conservan su propia versi&#243;n.
  </p>
  {#if current.metadata_revision}<p class="hint">
      Revisi&#243;n de clasificaci&#243;n: {current.metadata_revision}
    </p>
  {:else}<p class="hint">A&#250;n no hay cambios de clasificaci&#243;n registrados.</p>{/if}
  <MetadataSummary metadata={current} />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <div class="action-row">
    <button class="secondary" disabled={busy} onclick={load}>Actualizar clasificaci&#243;n</button
    ><button
      class="text-button"
      aria-expanded={showHistory}
      onclick={() => (showHistory = !showHistory)}
      >{showHistory
        ? 'Ocultar historial de clasificaci\u00f3n'
        : 'Ver historial de clasificaci\u00f3n'}</button
    >
  </div>
  {#if showHistory}{#key current.metadata_revision}<MetadataHistory
        api={scoped}
        {ondenied}
      />{/key}{/if}
</section>
{#if can(user.role, 'classify')}<MetadataEditor
    bind:this={editor}
    api={scoped}
    {current}
    onconfirmed={confirmed}
    {ondenied}
  />{/if}
