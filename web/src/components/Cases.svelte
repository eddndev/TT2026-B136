<script>
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import Pagination from './Pagination.svelte';
  import CaseFilters from './CaseFilters.svelte';
  import CaseEditor from './CaseEditor.svelte';
  import { staffCase, manageCase, basicCase } from '../lib/case-administration.mjs';
  export let api, user, onselect;
  let cases = [],
    offset = 0,
    hasMore = false,
    busy = false,
    error = '',
    creating = false;
  let filters = { status: 'active', profile: 'all' },
    cursors = [undefined],
    index = 0,
    nextId;
  let generation = 0,
    alive = true;
  $: staff = staffCase(user.role);
  async function load(position = 0) {
    const current = ++generation;
    busy = true;
    error = '';
    cases = [];
    hasMore = false;
    if (staff) index = position;
    else offset = position;
    try {
      const result = staff
        ? await api.caseAdministrations({ ...filters, afterId: cursors[index] })
        : await api.cases({ limit: 51, offset });
      if (!alive || current !== generation) return;
      cases = staff ? result.cases : result.slice(0, 50);
      hasMore = staff ? result.has_more : result.length > 50;
      nextId = result.next_after_id;
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
    const scoped = staff ? api.caseAdministration(record.id) : null;
    try {
      const detail = scoped ? await scoped.get() : await api.caseDetail(record.id);
      if (alive && current === generation) onselect(basicCase(detail));
    } catch (failure) {
      if (alive && current === generation) {
        cases = cases.filter((item) => item.id !== record.id);
        error = failure.message;
      }
    } finally {
      scoped?.dispose();
      if (alive && current === generation) busy = false;
    }
  }
  function apply(query) {
    filters = query;
    cursors = [undefined];
    load(0);
  }
  function next() {
    cursors = [...cursors.slice(0, index + 1), nextId];
    load(index + 1);
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
  {#if manageCase(user.role)}<button
      class="primary"
      disabled={busy || creating}
      onclick={() => (creating = true)}><Icon name="plus" size={18} />Nuevo expediente penal</button
    >{/if}
</div>
{#if creating}<CaseEditor
    {api}
    onconfirmed={(record) => {
      if (alive) onselect(basicCase(record));
    }}
    oncancel={() => (creating = false)}
  />{/if}
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
<section class="card case-list" aria-busy={busy}>
  <div class="section-heading">
    <div>
      <h2>Archivo de expedientes</h2>
      <p class="hint">Tus asignaciones se comprueban en cada consulta.</p>
    </div>
    <button class="secondary" disabled={busy} onclick={() => load(staff ? index : offset)}
      >Actualizar</button
    >
  </div>
  {#if staff}<CaseFilters {busy} onapply={apply} />{/if}
  {#if busy}<p class="hint" role="status">Consultando expedientes...</p>
  {:else if cases.length}<div class="case-grid">
      {#each cases as record}<button class="case-card" disabled={busy} onclick={() => open(record)}>
          <span class="tile-icon"><Icon name="briefcase" size={24} /></span><span class="case-text"
            ><strong>{record.title}</strong><small>{record.reference}</small>
            {#if staff}<span class="case-card-badges"
                ><span class="badge"
                  >{record.administrative_status === 'closed'
                    ? 'Cerrado administrativamente'
                    : 'Activo'}</span
                ><span class="badge" class:info={record.profile_status === 'complete'}
                  >{record.profile_status === 'complete'
                    ? 'Ficha completa'
                    : 'Ficha pendiente'}</span
                ></span
              >
              {#if record.penal_identifiers}<small
                  >NUC: {record.penal_identifiers.nuc} / Carpeta: {record.penal_identifiers
                    .judicial_case_number}</small
                >{/if}
            {/if}</span
          ><Icon name="arrow" size={18} /></button
        >{/each}
    </div>
  {:else if !error}<div class="empty-state">
      <span class="empty-icon"><Icon name="folder" size={35} /></span>
      <h3>No hay expedientes en esta consulta</h3>
      <p>
        {manageCase(user.role)
          ? 'Crea un expediente penal o revisa los filtros.'
          : 'El administrador puede asignarte un expediente para comenzar.'}
      </p>
    </div>{/if}
  {#if staff}<div class="pagination">
      <span class="hint">{cases.length} expedientes en esta p&#225;gina</span>
      <div class="action-row">
        <button class="secondary" disabled={busy || !index} onclick={() => load(index - 1)}
          >Anterior</button
        ><button class="secondary" disabled={busy || !hasMore} onclick={next}>Siguiente</button>
      </div>
    </div>
  {:else}<Pagination {offset} count={cases.length} {hasMore} {busy} onchange={load} />{/if}
</section>
