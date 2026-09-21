<script>
  import { onMount, onDestroy } from 'svelte';
  import ResourceActivityEditor from './ResourceActivityEditor.svelte';
  import ResourceActivityDetail from './ResourceActivityDetail.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { resourceActivityFailure, resourceDenied } from '../lib/resource-activity-errors.mjs';
  import { canResources } from '../lib/procedural-resource-errors.mjs';
  import {
    resourceActivityKinds,
    resourceActivityStatuses,
  } from '../lib/resource-activity-values.mjs';
  export let api,
    user,
    caseId,
    resource,
    ondenied,
    disabled = false,
    pending = false;
  const scoped = api.caseResourceActivities(caseId, resource.id),
    resources = api.caseResources(caseId),
    administration = caseState();
  let rows = [],
    selected = null,
    historical = false,
    action = null,
    head = null,
    editorRecord = null,
    editorKey = 0;
  let kind = 'all',
    status = 'linked',
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
    alive = true,
    generation = 0;
  $: pending = busy || opening || editorBusy || !!action;
  $: manage = canResources(user.role, 'manage') && !$administration.closed;
  function fail(failure) {
    error = resourceActivityFailure(failure);
    if (resourceDenied(failure)) {
      generation++;
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
    more = false;
    index = position;
    try {
      const page = await scoped.list({ ...applied, afterId: cursors[position] });
      if (alive && request === generation) {
        rows = page.associations;
        next = page.next_after_id;
        more = page.has_more;
      }
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function open(id, exact) {
    const request = ++generation;
    opening = true;
    selected = null;
    error = '';
    try {
      const view = exact === undefined ? await scoped.get(id) : await scoped.revision(id, exact);
      if (alive && request === generation) {
        selected = view;
        historical = exact !== undefined;
      }
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) opening = false;
    }
  }
  async function edit(record = null) {
    if (pending || disabled || !manage) return;
    opening = true;
    error = '';
    notice = '';
    try {
      const value = await resources.get(resource.id);
      if (!alive) return;
      if (!record && value.status !== 'active') {
        error = 'El recurso esta archivado. Consulta su historia o reactivalo para vincular.';
        return;
      }
      head = value;
      editorRecord = record;
      action = record ? 'unlink' : 'link';
      editorKey++;
    } catch (failure) {
      if (alive) fail(failure);
    } finally {
      if (alive) opening = false;
    }
  }
  async function confirmed(record, exact, confirmedView = null) {
    if (!alive) return;
    action = null;
    selected = null;
    notice = 'Vinculo guardado.';
    cursors = [undefined];
    await load();
    if (alive && !error) {
      if (confirmedView) {
        selected = confirmedView;
        historical = true;
      } else await open(record.id, exact ? record.revision : undefined);
    }
  }
  function reset() {
    selected = null;
    cursors = [undefined];
    load();
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
    pending = false;
    scoped.dispose();
    resources.dispose();
  });
</script>

<section class="fact-collection" aria-label="Actividades del recurso">
  <div class="section-heading">
    <h2>Actividades del recurso</h2>
    {#if canResources(user.role, 'manage')}<button
        class="primary"
        disabled={disabled || pending || !manage}
        onclick={() => edit()}>Vincular actividad</button
      >{/if}
  </div>
  <p class="hint">
    V&#237;nculos organizativos con revisiones exactas. La historia y el estado actual se consultan
    por separado.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
  {#if action}{#key editorKey}<ResourceActivityEditor
        {api}
        {caseId}
        {user}
        {resource}
        {head}
        record={editorRecord}
        {ondenied}
        {disabled}
        onconfirmed={confirmed}
        oncancel={() => (action = null)}
        bind:pending={editorBusy}
      />{/key}{/if}
  <section class="card" aria-label="Vinculos registrados" aria-busy={busy || opening}>
    <div class="section-heading">
      <h3>V&#237;nculos registrados</h3>
      <button class="secondary" disabled={disabled || pending} onclick={reset}
        >Actualizar actividades</button
      >
    </div>
    <div class="action-row">
      <label
        >Tipo de actividad consultada<select bind:value={kind} disabled={disabled || pending}
          ><option value="all">Todas</option
          >{#each Object.entries(resourceActivityKinds) as [value, label]}<option {value}
              >{label}</option
            >{/each}</select
        ></label
      >
      <label
        >Estado del v&#237;nculo<select bind:value={status} disabled={disabled || pending}
          ><option value="all">Todos</option
          >{#each Object.entries(resourceActivityStatuses) as [value, label]}<option {value}
              >{label}</option
            >{/each}</select
        ></label
      >
      <button
        class="secondary"
        disabled={disabled || pending}
        onclick={() => {
          applied = { kind, status };
          reset();
        }}>Filtrar actividades</button
      >
    </div>
    {#each rows as row (row.association.id)}
      <button
        class="case-card fact-row"
        disabled={disabled || pending}
        aria-label={`Consultar v\u00ednculo ${row.association.id}`}
        onclick={() => open(row.association.id)}
      >
        <span
          ><strong>{resourceActivityKinds[row.association.selection.target.kind]}</strong><span
            >Revisi&#243;n vinculada: {row.association.selection.target.revision}</span
          ><small>V&#237;nculo revisi&#243;n {row.association.revision}</small></span
        >
        <span class="badge">{resourceActivityStatuses[row.association.status]}</span>
      </button>
    {/each}
    {#if !rows.length && !busy && !error}<p>No hay actividades en esta consulta.</p>{/if}
    {#if busy || opening}<p role="status">Consultando actividades...</p>{/if}
    <div class="action-row">
      <button
        class="secondary"
        disabled={disabled || pending || !index}
        onclick={() => {
          selected = null;
          load(index - 1);
        }}>Actividades anteriores</button
      >
      <button
        class="secondary"
        disabled={disabled || pending || !more}
        onclick={() => {
          selected = null;
          cursors = [...cursors.slice(0, index + 1), next];
          load(index + 1);
        }}>Siguientes actividades</button
      >
    </div>
  </section>
  {#if selected}{#key `${selected.association.id}:${selected.association.revision}:${historical}`}<ResourceActivityDetail
        view={selected}
        api={scoped}
        {ondenied}
        onselect={open}
        onedit={edit}
        canManage={manage}
        disabled={disabled || pending}
        {historical}
      />{/key}{/if}
</section>
