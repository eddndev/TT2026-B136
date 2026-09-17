<script>
  import { onMount, onDestroy } from 'svelte';
  import HearingList from './HearingList.svelte';
  import HearingDetail from './HearingDetail.svelte';
  import HearingEditor from './HearingEditor.svelte';
  import HearingResults from './HearingResults.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { canHearings, hearingDenied } from '../lib/hearings.mjs';
  export let api,
    user,
    record,
    ondenied,
    onnavigate,
    intent = null,
    onintent = () => {};
  const scoped = api.caseHearings(record.id),
    participants = api.caseParticipants(record.id),
    typed = api.caseTypedParticipants(record.id),
    documents = api.caseDocuments(record.id),
    administration = caseState();
  let context = null,
    rows = [],
    selected = null,
    historical = false,
    action = null,
    editorBase = null;
  let status = 'all',
    appliedStatus = 'all',
    cursors = [undefined],
    index = 0,
    nextId,
    more = false;
  let error = '',
    notice = '',
    busy = false,
    opening = false,
    editorBusy = false,
    resultBusy = false,
    resultIntent = null,
    alive = true;
  let listGeneration = 0,
    detailGeneration = 0,
    contextGeneration = 0,
    editorGeneration = 0,
    detailView,
    consumed,
    initialized = false;
  $: locked = busy || opening || editorBusy || resultBusy || !!action;
  $: manage =
    canHearings(user.role, 'manage') &&
    !$administration.closed &&
    context?.administrative_status === 'active';
  $: if (initialized && intent && intent !== consumed) {
    consumed = intent;
    if (intent.case_id === record.id) open(intent.hearing_id, intent.revision);
    onintent();
  }
  function deny(failure) {
    listGeneration++;
    detailGeneration++;
    contextGeneration++;
    rows = [];
    selected = null;
    resultIntent = null;
    action = null;
    ondenied(failure);
  }
  function observeContext(value) {
    if (alive) context = value;
  }
  async function loadContext() {
    const generation = ++contextGeneration;
    try {
      const result = await scoped.context();
      if (alive && generation === contextGeneration) {
        context = result;
        return result;
      }
    } catch (failure) {
      if (alive && generation === contextGeneration) {
        error = failure.message;
        if (hearingDenied(failure)) deny(failure);
      }
    }
  }
  async function load(position = 0) {
    const generation = ++listGeneration;
    busy = true;
    error = '';
    rows = [];
    more = false;
    index = position;
    try {
      const page = await scoped.list({ status: appliedStatus, afterId: cursors[position] });
      if (!alive || generation !== listGeneration) return;
      rows = page.hearings;
      if (
        selected &&
        rows.some((row) => row.id === selected.id && row.revision > selected.revision)
      )
        historical = true;
      more = page.has_more;
      nextId = page.next_after_id;
    } catch (failure) {
      if (alive && generation === listGeneration) {
        error = failure.message;
        if (hearingDenied(failure)) deny(failure);
      }
    } finally {
      if (alive && generation === listGeneration) busy = false;
    }
  }
  async function open(id, revision) {
    const generation = ++detailGeneration;
    opening = true;
    selected = null;
    error = '';
    notice = '';
    try {
      const result = revision ? await scoped.revision(id, revision) : await scoped.get(id);
      if (alive && generation === detailGeneration) {
        selected = result;
        historical = revision !== undefined;
      }
    } catch (failure) {
      if (alive && generation === detailGeneration) {
        error = failure.message;
        if (hearingDenied(failure)) deny(failure);
      }
    } finally {
      if (alive && generation === detailGeneration) opening = false;
    }
  }
  async function edit(next) {
    if (locked || !manage) return;
    opening = true;
    const generation = ++detailGeneration;
    const nextContext = await loadContext();
    if (alive && generation === detailGeneration) {
      opening = false;
      if (nextContext) {
        editorBase = next === 'schedule' ? null : selected;
        action = next;
        editorGeneration++;
      }
    }
  }
  async function confirmed(result, exact = false) {
    if (!alive) return;
    detailGeneration++;
    selected = result;
    historical = exact;
    cursors = [undefined];
    notice = 'Audiencia guardada. Su programaci\u00f3n no determina un resultado.';
    await Promise.all([load(), detailView?.refreshHistory()]);
    if (alive) action = null;
  }
  function apply() {
    appliedStatus = status;
    resetList();
  }
  function resetList() {
    detailGeneration++;
    selected = null;
    cursors = [undefined];
    load();
  }
  onMount(() => {
    Promise.all([loadContext(), load()]).then(() => {
      if (alive) initialized = true;
    });
  });
  onDestroy(() => {
    alive = false;
    listGeneration++;
    detailGeneration++;
    contextGeneration++;
    scoped.dispose();
    participants.dispose();
    typed.dispose();
    documents.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">PROGRAMACI&#211;N DEL EXPEDIENTE</span>
    <h1>Audiencias del expediente</h1>
    <p>Citas declaradas e historial de cambios.</p>
  </div>
  {#if canHearings(user.role, 'manage')}<button
      class="primary"
      disabled={locked || !manage || !context?.profile_complete || !context?.stage_revision}
      onclick={() => edit('schedule')}>Programar audiencia</button
    >{/if}
</div>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if notice}<p class="notice success" role="status">{notice}</p>{/if}
{#if context && (!context.profile_complete || !context.stage_revision)}<p class="notice">
    Completa la ficha penal y registra la etapa para programar una audiencia.
  </p>{/if}
{#if action}{#key editorGeneration}<HearingEditor
      api={scoped}
      participantsApi={participants}
      typedApi={typed}
      {documents}
      caseId={record.id}
      {user}
      {action}
      record={editorBase}
      initialContext={context}
      ondenied={deny}
      onconfirmed={confirmed}
      oncontext={observeContext}
      oncancel={() => (action = null)}
      bind:pending={editorBusy}
    />{/key}{/if}
<section class="card hearing-index" aria-label="Audiencias registradas" aria-busy={busy || opening}>
  <div class="section-heading">
    <h2>Audiencias registradas</h2>
    <button
      class="secondary"
      disabled={locked}
      onclick={() => {
        loadContext();
        resetList();
      }}>Actualizar audiencias</button
    >
  </div>
  <div class="hearing-filter action-row">
    <label
      >Estado de audiencia<select bind:value={status} disabled={locked}
        ><option value="all">Todas</option><option value="scheduled">Programadas</option><option
          value="cancelled">Canceladas</option
        ></select
      ></label
    ><button class="secondary" disabled={locked} onclick={apply}>Aplicar estado</button>
  </div>
  {#if busy}<p role="status">Consultando audiencias...</p>{/if}
  <HearingList {rows} disabled={locked} onselect={(row) => open(row.id)} />
  {#if !busy && !rows.length && !error}<p>No hay audiencias en esta consulta.</p>{/if}
  <div class="pagination">
    <span class="hint">{rows.length} registros en esta p&#225;gina</span>
    <div class="action-row">
      <button
        class="secondary"
        disabled={locked || !index}
        onclick={() => {
          detailGeneration++;
          selected = null;
          load(index - 1);
        }}>Anterior</button
      ><button
        class="secondary"
        disabled={locked || !more}
        onclick={() => {
          cursors = [...cursors.slice(0, index + 1), nextId];
          detailGeneration++;
          selected = null;
          load(index + 1);
        }}>Siguiente</button
      >
    </div>
  </div>
</section>
{#if selected}{#key selected.id}<HearingDetail
      record={selected}
      api={scoped}
      ondenied={deny}
      onedit={edit}
      oncurrent={() => open(selected.id)}
      canManage={manage}
      disabled={locked}
      {historical}
      bind:this={detailView}
    />
    <HearingResults
      {api}
      caseId={record.id}
      {user}
      hearing={selected}
      canManage={canHearings(user.role, 'manage')}
      ondenied={deny}
      disabled={busy || opening || editorBusy || !!action}
      bind:pending={resultBusy}
      intent={resultIntent}
      onintent={() => (resultIntent = null)}
      onopenhearing={(id, revision, next) => {
        resultIntent = next;
        open(id, revision ?? undefined);
      }}
    />{/key}{/if}
<button class="text-button" onclick={() => onnavigate('agenda')}>Ir a Agenda</button>
