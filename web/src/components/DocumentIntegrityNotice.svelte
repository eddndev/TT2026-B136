<script>
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  export let api, onopen;
  const scoped = api.integrityIncidents();
  let exists = false,
    error = '',
    busy = false,
    alive = true,
    generation = 0;
  export function observed(value) {
    if (!alive) return;
    generation++;
    busy = false;
    exists = value;
    error = '';
  }
  async function load() {
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const result = await scoped.list({ limit: 1 });
      if (alive && request === generation) exists = result.incidents.length > 0;
    } catch (failure) {
      if (alive && request === generation) error = failure.message;
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onMount(() => {
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
  });
</script>

{#if exists || error}
  <section class="notice" aria-label="Avisos de integridad" aria-busy={busy}>
    {#if error}<p role="alert">No se pudo consultar el aviso de integridad. {error}</p>
    {:else}<p><strong>Hay incidentes de integridad registrados.</strong></p>{/if}
    <button class="text-button" onclick={onopen}>
      <Icon name="shield" size={17} />Consultar incidentes
    </button>
  </section>
{/if}
