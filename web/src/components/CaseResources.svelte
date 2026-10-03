<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { pendingResourceDrafts } from '../lib/resource-draft.mjs';
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
  export let intent = null,
    onintent = () => {};
  let initialized = false,
    consumed,
    associationIntent = null;
  $: if (initialized && intent && intent !== consumed) {
    consumed = intent;
    if (intent.case_id === record.id) open(intent.resource.id, intent.resource.revision, intent);
    onintent();
  }
  const caseId = record.id,
    scoped = api.caseResources(caseId),
    administration = caseState(),
    session = getContext('session-drafts');
  let savedDraft = null,
    drafts = pendingResourceDrafts(session, caseId);
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
  async function open(id, exact, linked = null) {
    const request = ++detailGeneration;
    opening = true;
    selected = null;
    associationIntent = null;
    error = '';
    notice = '';
    try {
      const value = exact === undefined ? await scoped.get(id) : await scoped.revision(id, exact);
      if (alive && request === detailGeneration) {
        if (linked && value.receipt.capture_digest !== linked.resource.capture_digest)
          throw new Error('La captura del recurso no coincide con el v\u00ednculo seleccionado.');
        associationIntent = linked?.association ?? null;
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
    selectedAct =
      nextAction === 'correct_act'
        ? { ...selected.act, resourceRevision: selected.revision }
        : null;
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
      savedDraft =
        drafts.find(
          (row) =>
            row.action === nextAction &&
            row.resourceId === (editorBase?.id ?? null) &&
            row.instanceId === (nextAction === 'correct_act' ? selectedAct?.id : null),
        ) ?? null;
      action = nextAction;
      editorKey++;
    }
  }
  async function resume(saved) {
    if (pending || !session?.canAdmit()) return;
    opening = true;
    error = '';
    try {
      editorBase = saved.resourceId ? await scoped.get(saved.resourceId) : null;
      if (!alive || !session.canAdmit()) return;
      selectedAct = null;
      savedDraft = saved;
      action = saved.action;
      editorKey++;
    } catch (failure) {
      if (alive) {
        if (failure.status === 404) {
          session.registry.closeEditor(saved.key);
          drafts = pendingResourceDrafts(session, caseId);
        }
        fail(failure);
      }
    } finally {
      if (alive) opening = false;
    }
  }
  function closeEditor() {
    action = null;
    savedDraft = null;
    drafts = pendingResourceDrafts(session, caseId);
  }
  async function confirmed(value, exact = false) {
    if (!alive) return;
    selected = value;
    historical = exact;
    notice = 'Registro guardado.';
    cursors = [undefined];
    await load();
    if (alive) closeEditor();
  }
  function reset() {
    selected = null;
    detailGeneration++;
    cursors = [undefined];
    load();
  }
  onMount(() => {
    load().then(() => {
      if (alive) initialized = true;
    });
  });
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
        {savedDraft}
        record={editorBase}
        {selectedAct}
        {ondenied}
        onconfirmed={confirmed}
        oncancel={closeEditor}
        bind:pending={editorBusy}
      />{/key}{/if}
  {#if !action}{#each drafts as saved (saved.key)}<button
        class="secondary"
        disabled={pending}
        onclick={() => resume(saved)}>Retomar borrador de recurso</button
      >{/each}{/if}
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
        intent={associationIntent}
        onintent={() => (associationIntent = null)}
        {ondenied}
        disabled={busy || opening || editorBusy || !!action}
        bind:pending={activityBusy}
      />{/key}{/if}
</section>
