<script>
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import ParticipantFilters from './ParticipantFilters.svelte';
  import ParticipantList from './ParticipantList.svelte';
  import ParticipantDetail from './ParticipantDetail.svelte';
  import ParticipantEditor from './ParticipantEditor.svelte';
  import { canParticipants } from '../lib/participants.mjs';
  export let api;
  export let user;
  export let caseRecord;
  const scoped = api.caseParticipants(caseRecord.id);
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
    editor;
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
  function observed(record) {
    if (!alive) return;
    if (selected?.id === record.id && selected.revision <= record.revision) selected = record;
    cursors = [undefined];
    load(0);
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
    invalidateDetail();
    selected = record;
    cursors = [undefined];
    notice = 'Participante guardado.';
    load(0);
  }
  function statusChanged(record) {
    if (!alive) return;
    invalidateDetail();
    cursors = [undefined];
    notice =
      record.directory_status === 'active' ? 'Participante reactivado.' : 'Participante archivado.';
    load(0);
  }
  onMount(() => {
    if (canParticipants(user.role, 'read')) load();
  });
  onDestroy(() => {
    alive = false;
    listGeneration++;
    detailGeneration++;
    scoped.dispose();
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
      disabled={$administration.closed}
      onclick={() => editor.open()}><Icon name="plus" size={18} />Agregar participante</button
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
    <button class="secondary" disabled={busy} onclick={() => load()}>Actualizar</button>
  </div>
  <ParticipantFilters onapply={apply} {busy} />
  {#if busy}<p class="hint" role="status">Consultando participantes...</p>
  {:else if rows.length}<ParticipantList {rows} onselect={open} {opening} />
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
      <button class="secondary" disabled={busy || !index} onclick={previous}>Anterior</button
      ><button class="secondary" disabled={busy || !hasMore} onclick={next}>Siguiente</button>
    </div>
  </div>
</section>
{#if selected}{#key selected.id}<ParticipantDetail
      api={scoped}
      {user}
      record={selected}
      onedit={(record) => editor.open(record)}
      onobserved={observed}
      onstatus={statusChanged}
      ondenied={denied}
    />{/key}{/if}
{#if canParticipants(user.role, 'manage')}{#key editorGeneration}<ParticipantEditor
      bind:this={editor}
      api={scoped}
      onconfirmed={confirmed}
      onobserved={observed}
      ondenied={denied}
    />{/key}{/if}
