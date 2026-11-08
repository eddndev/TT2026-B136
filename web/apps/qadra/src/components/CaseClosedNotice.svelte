<script>
  import Button from './Button.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  let busy = false;
  async function refresh() {
    if (busy) return;
    busy = true;
    try {
      await $administration.refresh?.();
    } finally {
      busy = false;
    }
  }
</script>

{#if $administration.closed}<div class="notice stack">
    <p>El expediente est&#225; cerrado administrativamente. Tu formulario se conserva.</p>
    <Button type="button" variant="secondary" disabled={busy} onclick={refresh}
      >{busy ? 'Consultando estado...' : 'Consultar estado del expediente'}</Button
    >
  </div>{/if}
