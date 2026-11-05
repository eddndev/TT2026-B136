<script>
  import { onDestroy } from 'svelte';
  import HeroIcon from './HeroIcon.svelte';
  export let api;
  let result = null,
    busy = false,
    error = '',
    alive = true;
  async function verify() {
    busy = true;
    error = '';
    result = null;
    try {
      const value = await api.audit();
      if (alive) result = value;
    } catch (failure) {
      if (alive) error = failure.message;
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
  });
</script>

<section
  class="card flex flex-wrap items-center gap-x-4 gap-y-3"
  aria-label="Integridad de la cadena"
  aria-busy={busy}
>
  <HeroIcon name="shield" class="size-6 text-brand-800" />
  <div class="min-w-0 flex-1">
    <h2 class="text-base">Integridad de la cadena</h2>
    {#if result}
      <p role="status" class={result.valid ? 'text-success' : 'text-red-700'}>
        <strong>{result.valid ? 'Cadena \u00edntegra' : 'Alteraci\u00f3n detectada'}</strong>
        &#183;
        {result.valid
          ? result.entries === 1
            ? '1 evento verificado'
            : `${result.entries} eventos verificados`
          : `primer evento roto: ${result.first_broken_index}`}
      </p>
    {:else if error}<p class="text-red-700" role="alert">{error}</p>
    {:else}<p>Comprueba que ning&#250;n movimiento se haya alterado o eliminado.</p>{/if}
  </div>
  <button class="primary" disabled={busy} onclick={verify}
    >{busy ? 'Verificando...' : 'Verificar cadena'}</button
  >
</section>
