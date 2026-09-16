<script>
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import ParticipantFilters from './ParticipantFilters.svelte';
  import ParticipantList from './ParticipantList.svelte';
  import ParticipantDetail from './ParticipantDetail.svelte';
  import ParticipantEditor from './ParticipantEditor.svelte';
  import TypedParticipantEditor from './TypedParticipantEditor.svelte';
  import { canParticipants } from '../lib/participants.mjs';
  export let api;
  export let user;
  export let caseRecord;
  const scoped = api.caseParticipants(caseRecord.id);
  const typedApi = api.caseTypedParticipants(caseRecord.id);
  const docs = api.caseDocuments(caseRecord.id);
  let typedEditor;
  let rows = [],
    selected = null,
    filters = { status: 'active' };
  let cursors = [undefined],
    index = 0,
    nextId,
    hasMore = false;
  let busy = false,
    opening = false,
    error = '',
    notice = '',
    alive = true;
  let listGeneration = 0,
    detailGeneration = 0,
    editorGeneration = 0,
    editor,
    participantDetail;
  let refreshing = 0;
  function invalidateDetail() {
    detailGeneration++;
    opening = false;
    selected = null;
  }
  function denied(failure) {
    listGeneration++;
    invalidateDetail();
    editorGeneration++;
    rows = [];
    hasMore = false;
    busy = false;
    error = failure.message;
    notice = '';
  }
  async function refreshReaders(id) {
    refreshing++;
    try {
      await Promise.all([load(0), id ? participantDetail?.refreshHistory(id) : undefined]);
    } finally {
      if (alive) refreshing--;
    }
  }
  function observed(record) {
    if (!alive) return;
    const changed = selected?.id === record.id && selected.revision < record.revision;
    if (selected?.id === record.id && selected.revision <= record.revision) selected = record;
    cursors = [undefined];
    return refreshReaders(changed ? record.id : null);
  }
  async function load(nextIndex = index, afterId = cursors[nextIndex]) {
    const generation = ++listGeneration;
    busy = true;
    error = '';
    rows = [];
    hasMore = false;
    index = nextIndex;
    try {
      const page = await scoped.list({ ...filters, afterId });
      if (!alive || generation !== listGeneration) return;
      rows = page.participants.map((row) =>
        selected?.id === row.id && selected.revision > row.revision ? selected : row,
      );
      hasMore = page.has_more;
      nextId = page.next_after_id;
    } catch (failure) {
      if (alive && generation === listGeneration) {
        if ([403, 404].includes(failure.status)) denied(failure);
        else error = failure.message;
      }
    } finally {
      if (alive && generation === listGeneration) busy = false;
    }
  }
  function apply(values) {
    filters = values;
    cursors = [undefined];
    notice = '';
    invalidateDetail();
    load(0);
  }
  function next() {
    if (!hasMore || busy) return;
    cursors = [...cursors.slice(0, index + 1), nextId];
    invalidateDetail();
    load(index + 1);
  }
  function previous() {
    invalidateDetail();
    load(index - 1);
  }
  async function open(record) {
    invalidateDetail();
    const generation = detailGeneration;
    opening = true;
    error = '';
    notice = '';
    try {
      const detail = await scoped.get(record.id);
      if (alive && generation === detailGeneration) selected = detail;
    } catch (failure) {
      if (alive && generation === detailGeneration) {
        if ([403, 404].includes(failure.status)) denied(failure);
        else error = failure.message;
      }
    } finally {
      if (alive && generation === detailGeneration) opening = false;
    }
  }
  function confirmed(record) {
    if (!alive) return;
    const existing = selected?.id === record.id;
    invalidateDetail();
    selected = record;
    cursors = [undefined];
    notice = 'Participante guardado.';
    return refreshReaders(existing ? record.id : null);
  }
  function statusChanged(record) {
    if (!alive) return;
    invalidateDetail();
    cursors = [undefined];
    notice =
      record.directory_status === 'active' ? 'Participante reactivado.' : 'Participante archivado.';
    return refreshReaders(null);
  }
  onMount(() => {
    if (canParticipants(user.role, 'read')) load();
  });
  onDestroy(() => {
    alive = false;
    listGeneration++;
    detailGeneration++;
    scoped.dispose();
    typedApi.dispose();
    docs.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">PERSONAS DEL EXPEDIENTE</span>
    <h1>Participantes</h1>
    <p>Personas registradas en este expediente.</p>
  </div>
  {#if canParticipants(user.role, 'manage')}<button
      class="primary"
      disabled={$administration.closed || !!refreshing}
      onclick={() => typedEditor.open()}><Icon name="plus" size={18} />Agregar participante</button
    >{/if}
</div>
{#if notice}<p class="notice success" role="status">{notice}</p>{/if}
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
<section class="card participant-directory" aria-label="Directorio del expediente" aria-busy={busy}>
  <div class="section-heading">
    <div>
      <h2>Directorio del expediente</h2>
      <p class="hint">Este registro organiza personas; no administra cuentas ni permisos.</p>
    </div>
    <button class="secondary" disabled={busy || !!refreshing} onclick={() => load()}
      >Actualizar</button
    >
  </div>
  {#if canParticipants(user.role, 'manage')}<button
      class="text-button"
      disabled={$administration.closed || !!refreshing}
      onclick={() => editor.open()}>Registrar ficha pendiente</button
    >{/if}
  <ParticipantFilters onapply={apply} busy={busy || !!refreshing} />
  {#if busy}<p class="hint" role="status">Consultando participantes...</p>
  {:else if rows.length}<ParticipantList {rows} onselect={open} opening={opening || !!refreshing} />
  {:else if !error}<div class="empty-state">
      <span class="empty-icon"><Icon name="users" size={35} /></span>
      <h3>A&#250;n no hay participantes en esta consulta</h3>
      <p>
        {canParticipants(user.role, 'manage')
          ? 'Revisa los filtros o registra una persona en este expediente.'
          : 'Revisa los filtros para consultar las personas registradas.'}
      </p>
    </div>{/if}
  {#if opening}<p class="hint" role="status">Consultando participante...</p>{/if}
  <div class="pagination">
    <span class="hint"
      >{rows.length}
      {rows.length === 1 ? 'participante' : 'participantes'} en esta p&#225;gina</span
    >
    <div class="action-row">
      <button class="secondary" disabled={busy || !!refreshing || !index} onclick={previous}
        >Anterior</button
      ><button class="secondary" disabled={busy || !!refreshing || !hasMore} onclick={next}
        >Siguiente</button
      >
    </div>
  </div>
</section>
{#if selected}{#key selected.id}<ParticipantDetail
      bind:this={participantDetail}
      api={scoped}
      {user}
      {typedApi}
      {docs}
      caseId={caseRecord.id}
      record={selected}
      onedit={(record) => (record.profile ? typedEditor.open(record) : editor.open(record))}
      oncomplete={(record) => typedEditor.open(record)}
      onobserved={observed}
      onstatus={statusChanged}
      ondenied={denied}
      disabled={!!refreshing}
    />{/key}{/if}
{#if canParticipants(user.role, 'manage')}{#key editorGeneration}<ParticipantEditor
      bind:this={editor}
      api={scoped}
      onconfirmed={confirmed}
      onobserved={observed}
      ondenied={denied}
    />{/key}{/if}

{#if canParticipants(user.role, 'manage')}{#key editorGeneration}<TypedParticipantEditor
      bind:this={typedEditor}
      api={typedApi}
      manualApi={scoped}
      {docs}
      caseId={caseRecord.id}
      onconfirmed={confirmed}
      onobserved={observed}
      ondenied={denied}
    />{/key}{/if}
