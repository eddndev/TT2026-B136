<script>
  import { onDestroy } from 'svelte';
  import DocumentDetail from './DocumentDetail.svelte';
  export let api;
  export let user;
  export let document;
  export let onupdate;
  export let ondenied = () => {};
  const selected = api.version(document.id, document.version);
  onDestroy(() => selected.dispose());
</script>

<DocumentDetail api={selected} {user} {document} {onupdate} {ondenied} />
