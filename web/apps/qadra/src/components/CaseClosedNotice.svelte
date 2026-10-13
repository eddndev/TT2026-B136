<script>
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
    <button type="button" class="secondary" disabled={busy} onclick={refresh}
      >{busy ? 'Consultando estado...' : 'Consultar estado del expediente'}</button
    >
  </div>{/if}
