<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { pendingActivityDrafts } from '../lib/resource-activity-draft.mjs';
  import { pendingResourceDeadlineDrafts } from '../lib/resource-deadline-draft.mjs';
  import ResourceActivityEditor from './ResourceActivityEditor.svelte';
  import ResourceDeadlineEditor from './ResourceDeadlineEditor.svelte';
  import ResourceActivityDetail from './ResourceActivityDetail.svelte';
  import ResourceHearingScheduling from './ResourceHearingScheduling.svelte';
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
  const session = getContext('session-drafts');
  const pendingDrafts = () => [
    ...pendingActivityDrafts(session, caseId, resource.id),
    ...pendingResourceDeadlineDrafts(session, caseId, resource.id),
  ];
  let savedDraft = null,
    drafts = pendingDrafts();
  export let intent = null,
    onintent = () => {};
  let initialized = false,
    consumed;
  $: if (initialized && intent && intent !== consumed) {
    consumed = intent;
    open(intent.id, intent.revision, intent.capture_digest);
    onintent();
  }
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
    hearingBusy = false,
    alive = true,
    generation = 0;
  $: pending = busy || opening || editorBusy || hearingBusy || !!action;
  $: manage = canResources(user.role, 'manage') && !$administration.closed;
  function fail(failure) {
    error = resourceActivityFailure(failure);
    if (resourceDenied(failure)) {
      if (failure.code === 'case_not_found' || failure.status === 403)
        session?.registry.denyContext(caseId);
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
  async function open(id, exact, captureDigest) {
    const request = ++generation;
    opening = true;
    selected = null;
    error = '';
    try {
      const view = exact === undefined ? await scoped.get(id) : await scoped.revision(id, exact);
      if (alive && request === generation) {
        if (
          captureDigest !== undefined &&
          view.association.receipt.capture_digest !== captureDigest
        )
          throw new Error('La captura del v\u00ednculo no coincide con la selecci\u00f3n.');
        selected = view;
        historical = exact !== undefined;
      }
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) opening = false;
    }
  }
  async function edit(record = null, createDeadline = false) {
    if (pending || disabled || !manage) return;
    opening = true;
    error = '';
    notice = '';
    try {
      const value = await resources.get(resource.id);
      if (!alive || (createDeadline && session && !session.canAdmit())) return;
      if (!record && value.status !== 'active') {
        error = 'El recurso esta archivado. Consulta su historia o reactivalo para vincular.';
        return;
      }
      head = value;
      editorRecord = record;
      action = createDeadline ? 'create-deadline' : record ? 'unlink' : 'link';
      savedDraft = createDeadline
        ? (drafts.find((row) => row.editorKind === 'resource-deadline') ?? null)
        : (drafts.find(
            (row) =>
              row.editorKind === 'resource-activity' &&
              row.action === action &&
              row.instanceId === (record?.id ?? null),
          ) ?? null);
      editorKey++;
    } catch (failure) {
      if (alive && createDeadline && session?.canAdmit() && failure.status === 404)
        pendingResourceDeadlineDrafts(session, caseId, resource.id).forEach((saved) =>
          session?.registry.closeEditor(saved.key),
        );
      if (alive) fail(failure);
    } finally {
      if (alive) opening = false;
    }
  }
  function closeEditor() {
    action = null;
    savedDraft = null;
    editorBusy = false;
    drafts = pendingDrafts();
  }
  async function resume(saved) {
    if (pending || disabled || !session?.canAdmit()) return;
    opening = true;
    error = '';
    try {
      head = await resources.get(resource.id);
      if (!alive || !session.canAdmit()) return;
      editorRecord =
        saved.action === 'unlink' ? (await scoped.get(saved.instanceId)).association : null;
      if (!alive || !session.canAdmit()) return;
      savedDraft = saved;
      action = saved.editorKind === 'resource-deadline' ? 'create-deadline' : saved.action;
      editorKey++;
    } catch (failure) {
      if (alive) {
        if (failure.status === 404) session.registry.closeEditor(saved.key);
        fail(failure);
      }
    } finally {
      if (alive) opening = false;
    }
  }
  async function confirmed(record, exact, confirmedView = null) {
    if (!alive) return;
    closeEditor();
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
  onMount(() => {
    load().then(() => {
      if (alive) initialized = true;
    });
  });
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
    {#if canResources(user.role, 'manage')}<button
        class="primary"
        disabled={disabled || pending || !manage}
        onclick={() => edit(null, true)}>Crear plazo</button
      >{/if}
  </div>
  <p class="hint">
    V&#237;nculos organizativos con revisiones exactas. La historia y el estado actual se consultan
    por separado.
  </p>
  <ResourceHearingScheduling
    {api}
    {user}
    {caseId}
    {resource}
    {ondenied}
    disabled={disabled || busy || opening || !!action}
    oncreated={(value) => confirmed(value.association, true)}
    bind:pending={hearingBusy}
  />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
  {#if !action}{#each drafts as saved (saved.key)}<button
        class="secondary"
        disabled={pending || disabled}
        onclick={() => resume(saved)}
        >{saved.editorKind === 'resource-deadline'
          ? 'Retomar borrador de plazo'
          : 'Retomar borrador de actividad'}</button
      >{/each}{/if}
  {#if action === 'create-deadline'}{#key editorKey}<ResourceDeadlineEditor
        {api}
        {caseId}
        {user}
        {resource}
        {head}
        {ondenied}
        {disabled}
        {savedDraft}
        onconfirmed={confirmed}
        oncancel={closeEditor}
        bind:pending={editorBusy}
      />{/key}
  {:else if action}{#key editorKey}<ResourceActivityEditor
        {api}
        {caseId}
        {user}
        {resource}
        {head}
        record={editorRecord}
        {savedDraft}
        {ondenied}
        {disabled}
        onconfirmed={confirmed}
        oncancel={closeEditor}
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
