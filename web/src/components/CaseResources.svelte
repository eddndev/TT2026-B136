<script>
  import { onMount, onDestroy } from 'svelte';
  import ResourceEditor from './ResourceEditor.svelte';
  import ResourceDetail from './ResourceDetail.svelte';
  import ResourceActivities from './ResourceActivities.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { resourceKinds } from '../lib/procedural-resource-values.mjs';
  import {
    canResources,
    resourceDenied,
    resourceFailure,
  } from '../lib/procedural-resource-errors.mjs';
  import '../styles/procedural-facts.css';
  export let api, user, record, ondenied;
  const caseId = record.id,
    scoped = api.caseResources(caseId),
    administration = caseState();
  let rows = [],
    selected = null,
    historical = false,
    action = null,
    editorBase = null,
    selectedAct = null,
    editorKey = 0;
  let kind = 'all',
    status = 'all',
    applied = { kind, status },
    cursors = [undefined],
    index = 0,
    next,
    more = false;
  let error = '',
    notice = '',
    busy = false,
    opening = false,
    editorBusy = false,
    activityBusy = false,
    alive = true,
    generation = 0,
    detailGeneration = 0;
  $: pending = busy || opening || editorBusy || activityBusy || !!action;
  $: manage = canResources(user.role, 'manage') && !$administration.closed;
  function fail(failure) {
    error = resourceFailure(failure);
    if (resourceDenied(failure)) {
      generation++;
      detailGeneration++;
      rows = [];
      selected = null;
      action = null;
      ondenied(failure);
    }
  }
  async function load(position = 0) {
    const request = ++generation;
    busy = true;
    rows = [];
    error = '';
    index = position;
    more = false;
    try {
      const page = await scoped.list({ ...applied, afterId: cursors[position] });
      if (alive && request === generation) {
        rows = page.resources;
        more = page.has_more;
        next = page.next_after_id;
        if (selected && rows.some((v) => v.id === selected.id && v.revision > selected.revision))
          historical = true;
      }
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function open(id, exact) {
    const request = ++detailGeneration;
    opening = true;
    selected = null;
    error = '';
    notice = '';
    try {
      const value = exact === undefined ? await scoped.get(id) : await scoped.revision(id, exact);
      if (alive && request === detailGeneration) {
        selected = value;
        historical = exact !== undefined;
      }
    } catch (failure) {
      if (alive && request === detailGeneration) fail(failure);
    } finally {
      if (alive && request === detailGeneration) opening = false;
    }
  }
  async function edit(nextAction) {
    if (pending || !manage) return;
    editorBase = nextAction === 'register' ? null : selected;
    selectedAct = nextAction === 'correct_act' ? selected.act : null;
    if (nextAction === 'correct_act') {
      opening = true;
      error = '';
      try {
        const current = await scoped.get(selected.id);
        if (!alive) return;
        if (current.status !== 'active') {
          error = 'El recurso esta archivado; consulta su registro actual.';
          return;
        }
        editorBase = current;
      } catch (failure) {
        if (alive) fail(failure);
        return;
      } finally {
        if (alive) opening = false;
      }
    }
    if (alive) {
      action = nextAction;
      editorKey++;
    }
  }
  async function confirmed(value, exact = false) {
    if (!alive) return;
    selected = value;
    historical = exact;
    notice = 'Registro guardado.';
    cursors = [undefined];
    await load();
    if (alive) action = null;
  }
  function reset() {
    selected = null;
    detailGeneration++;
    cursors = [undefined];
    load();
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
    detailGeneration++;
    scoped.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">GESTI&#211;N DEL EXPEDIENTE</span>
    <h1>Recursos procesales</h1>
    <p>Revocaci&#243;n y apelaci&#243;n con sus actos, personas y fuentes hist&#243;ricas.</p>
  </div>
</div>
<section class="fact-collection" aria-label="Recursos procesales del expediente">
  <div class="section-heading">
    <h2>Recursos del expediente</h2>
    {#if canResources(user.role, 'manage')}<button
        class="primary"
        disabled={pending || !manage}
        onclick={() => edit('register')}>Registrar recurso</button
      >{/if}
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
  {#if action}{#key editorKey}<ResourceEditor
        {api}
        {caseId}
        {user}
        {action}
        record={editorBase}
        {selectedAct}
        {ondenied}
        onconfirmed={confirmed}
        oncancel={() => (action = null)}
        bind:pending={editorBusy}
      />{/key}{/if}
  <section class="card" aria-label="Recursos registrados" aria-busy={busy || opening}>
    <div class="section-heading">
      <h3>Registros disponibles</h3>
      <button class="secondary" disabled={pending} onclick={reset}>Actualizar recursos</button>
    </div>
    <div class="action-row">
      <label
        >Tipo de recurso consultado<select bind:value={kind} disabled={pending}
          ><option value="all">Todos</option>
          {#each Object.entries(resourceKinds) as [value, label]}<option {value}>{label}</option
            >{/each}</select
        ></label
      >
      <label
        >Estado organizativo<select bind:value={status} disabled={pending}
          ><option value="all">Todos</option><option value="active">Activos</option><option
            value="archived">Archivados</option
          ></select
        ></label
      >
      <button
        class="secondary"
        disabled={pending}
        onclick={() => {
          applied = { kind, status };
          reset();
        }}>Aplicar filtros</button
      >
    </div>
    <p class="hint">El orden por identificador no indica precedencia procesal.</p>
    {#each rows as row (row.id)}<button
        class="case-card fact-row"
        disabled={pending}
        aria-label={`Consultar recurso ${row.values.title}`}
        onclick={() => open(row.id)}
      >
        <span
          ><strong>{row.values.title}</strong><span>{resourceKinds[row.values.kind]}</span><small
            >Revisi&#243;n {row.revision}</small
          ></span
        >
        <span class="badge" class:info={row.status === 'active'}
          >{row.status === 'active' ? 'Activo' : 'Archivado'}</span
        ></button
      >{/each}
    {#if !rows.length && !busy && !error}<p>No hay recursos en esta consulta.</p>{/if}
    {#if busy || opening}<p role="status">Consultando recursos...</p>{/if}
    <div class="action-row">
      <button
        class="secondary"
        disabled={pending || !index}
        onclick={() => {
          selected = null;
          load(index - 1);
        }}>Recursos anteriores</button
      >
      <button
        class="secondary"
        disabled={pending || !more}
        onclick={() => {
          selected = null;
          cursors = [...cursors.slice(0, index + 1), next];
          load(index + 1);
        }}>Siguientes recursos</button
      >
    </div>
  </section>
  {#if selected}{#key `${selected.id}:${selected.revision}:${historical}`}<ResourceDetail
        record={selected}
        api={scoped}
        {ondenied}
        onedit={edit}
        onselect={open}
        canManage={manage}
        disabled={pending}
        {historical}
      /><ResourceActivities
        {api}
        {user}
        {caseId}
        resource={selected}
        {ondenied}
        disabled={busy || opening || editorBusy || !!action}
        bind:pending={activityBusy}
      />{/key}{/if}
</section>
