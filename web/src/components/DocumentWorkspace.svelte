<script>
  import { onDestroy } from 'svelte';
  import DocumentMetadata from './DocumentMetadata.svelte';
  import DocumentVersions from './DocumentVersions.svelte';
  export let api;
  export let user;
  export let document;
  export let onupdate;
  export let onmetadata;
  export let ondenied;
  export let loading = false;
  let ready = false;
  let metadataPending = false;
  let versionsPending = false;
  $: if (!loading) ready = true;
  let alive = true;
  function deny(failure) {
    if (alive) ondenied(failure);
  }
  function metadata(record) {
    if (alive) return onmetadata(record);
  }
  onDestroy(() => {
    alive = false;
  });
</script>

{#if ready}
  <DocumentMetadata
    {api}
    {user}
    {document}
    disabled={loading || versionsPending}
    bind:pending={metadataPending}
    onmetadata={metadata}
    ondenied={deny}
  />
  <DocumentVersions
    {api}
    {user}
    {document}
    disabled={loading || metadataPending}
    bind:pending={versionsPending}
    {onupdate}
    ondenied={deny}
  />
{:else}<p class="notice" role="status">Preparando la ficha del documento...</p>{/if}
