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
  let alive = true;
  function deny(failure) {
    if (alive) ondenied(failure);
  }
  function metadata(record) {
    if (alive) onmetadata(record);
  }
  onDestroy(() => {
    alive = false;
  });
</script>

<DocumentMetadata {api} {user} {document} onmetadata={metadata} ondenied={deny} />
<DocumentVersions {api} {user} {document} {onupdate} ondenied={deny} />
