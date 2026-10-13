<script>
  import { onDestroy } from 'svelte';
  export let api, submitted, onselect;
  let rows = [],
    queried = false,
    hasMore = false,
    next;
  let busy = false,
    error = '',
    alive = true,
    generation = 0;
  async function search(more = false) {
    const request = ++generation;
    busy = true;
    error = '';
    if (!more) {
      rows = [];
      hasMore = false;
      next = undefined;
    }
    try {
      const result = await api.caseAdministrations({
        status: 'all',
        profile: 'all',
        ...submitted,
        afterId: more ? next : undefined,
      });
      if (!alive || request !== generation) return;
      rows = more ? [...rows, ...result.cases] : result.cases;
      hasMore = result.has_more;
      next = result.next_after_id;
      queried = true;
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        rows = [];
        hasMore = false;
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function open(record) {
    const request = ++generation;
    const scoped = api.caseAdministration(record.id);
    busy = true;
    error = '';
    try {
      const detail = await scoped.get();
      if (alive && request === generation) onselect(detail);
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        rows = [];
        hasMore = false;
      }
    } finally {
      scoped.dispose();
      if (alive && request === generation) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<section class="case-comparison" aria-label="Coincidencias guardadas">
  <h3>Consultar el resultado</h3>
  <p>
    Busca por el NUC y la carpeta del &#250;ltimo env&#237;o, incluidos expedientes cerrados. Una
    coincidencia no confirma qui&#233;n realiz&#243; el alta. Revisa sus datos antes de continuar.
  </p>
  <p class="hint">
    NUC enviado: {submitted.nuc} / Carpeta enviada: {submitted.judicial_case_number}
  </p>
  <button type="button" class="secondary" disabled={busy} onclick={() => search()}
    >Consultar expedientes guardados</button
  >
  {#if queried}<div class="case-reconciliation-results">
      {#each rows as record}<button
          type="button"
          class="case-card"
          disabled={busy}
          onclick={() => open(record)}
          ><span class="case-text"
            ><strong>{record.title}</strong><small>{record.reference}</small><span
              >Abrir datos guardados</span
            ></span
          ></button
        >{/each}
    </div>
    {#if !rows.length && !error}<p class="hint">
        No hay coincidencias en esta consulta. Esto no demuestra que el env&#237;o anterior se haya
        rechazado.
      </p>{/if}{/if}
  {#if hasMore}<button type="button" class="secondary" disabled={busy} onclick={() => search(true)}
      >Cargar m&#225;s coincidencias</button
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</section>
