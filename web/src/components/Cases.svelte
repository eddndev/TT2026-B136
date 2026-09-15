<script>
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import Pagination from './Pagination.svelte';
  export let api;
  export let user;
  export let onselect;
  let cases = [];
  let offset = 0;
  let hasMore = false;
  let busy = false;
  let error = '';
  let creating = false;
  let title = '';
  let reference = '';
  let generation = 0;
  let alive = true;
  $: canCreate = ['owner', 'litigator'].includes(user.role);
  async function load(nextOffset = 0) {
    const current = ++generation;
    busy = true;
    error = '';
    cases = [];
    hasMore = false;
    offset = nextOffset;
    try {
      const result = await api.cases({ limit: 51, offset });
      if (!alive || current !== generation) return;
      cases = result.slice(0, 50);
      hasMore = result.length > 50;
    } catch (failure) {
      if (alive && current === generation) error = failure.message;
    } finally {
      if (alive && current === generation) busy = false;
    }
  }
  async function open(record) {
    const current = ++generation;
    busy = true;
    error = '';
    try {
      const detail = await api.caseDetail(record.id);
      if (alive && current === generation) onselect(detail);
    } catch (failure) {
      if (alive && current === generation) {
        cases = cases.filter((item) => item.id !== record.id);
        error = failure.message;
      }
    } finally {
      if (alive && current === generation) busy = false;
    }
  }
  async function create(event) {
    event.preventDefault();
    const current = ++generation;
    busy = true;
    error = '';
    try {
      const record = await api.createCase(title.trim(), reference.trim());
      if (alive && current === generation) onselect(record);
    } catch (failure) {
      if (alive && current === generation) error = failure.message;
    } finally {
      if (alive && current === generation) busy = false;
    }
  }
  onMount(() => {
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">ARCHIVO DEL DESPACHO</span>
    <h1>Expedientes</h1>
    <p>Consulta los expedientes a los que tienes acceso.</p>
  </div>
  {#if canCreate}<button class="primary" disabled={busy} onclick={() => (creating = !creating)}
      ><Icon name="plus" size={18} />Nuevo expediente</button
    >{/if}
</div>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if creating && canCreate}<section class="card case-create">
    <div class="section-heading">
      <h2>Nuevo expediente</h2>
      <button class="text-button" disabled={busy} onclick={() => (creating = false)}
        >Cancelar</button
      >
    </div>
    <form class="stack" onsubmit={create}>
      <label
        >T&#237;tulo del expediente<input
          required
          maxlength="200"
          bind:value={title}
          disabled={busy}
        /></label
      >
      <label
        >Referencia del expediente<input
          required
          maxlength="100"
          bind:value={reference}
          disabled={busy}
          placeholder="NUC o referencia interna"
        /></label
      >
      <button class="primary" disabled={busy}>{busy ? 'Guardando...' : 'Crear expediente'}</button>
    </form>
  </section>{/if}
<section class="card case-list" aria-busy={busy}>
  <div class="section-heading">
    <div>
      <h2>Archivo de expedientes</h2>
      <p class="hint">Tus asignaciones se comprueban en cada consulta.</p>
    </div>
    <button class="secondary" disabled={busy} onclick={() => load(offset)}>Actualizar</button>
  </div>
  {#if busy}<p class="hint" role="status">Consultando expedientes...</p>
  {:else if cases.length}<div class="case-grid">
      {#each cases as record}<button class="case-card" disabled={busy} onclick={() => open(record)}>
          <span class="tile-icon"><Icon name="briefcase" size={24} /></span><span class="case-text"
            ><strong>{record.title}</strong><small>{record.reference}</small></span
          ><Icon name="arrow" size={18} />
        </button>{/each}
    </div>
  {:else if !error}<div class="empty-state">
      <span class="empty-icon"><Icon name="folder" size={35} /></span>
      <h3>No hay expedientes en esta consulta</h3>
      <p>
        {canCreate
          ? 'Crea tu primer expediente para organizar sus documentos.'
          : 'El administrador puede asignarte un expediente para comenzar.'}
      </p>
    </div>{/if}
  <Pagination {offset} count={cases.length} {hasMore} {busy} onchange={load} />
</section>
