<script>
  import { onDestroy } from 'svelte';
  import DocumentDetail from './DocumentDetail.svelte';
  export let api;
  export let user;
  export let document;
  export let onupdate;
  export let disabled = false;
  export let busy = '';
  export let ondenied = () => {};
  const selected = api.version(document.id, document.version);
  onDestroy(() => selected.dispose());
</script>

<DocumentDetail bind:busy api={selected} {user} {document} {disabled} {onupdate} {ondenied} />
