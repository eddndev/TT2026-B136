<script>
  import { onMount, onDestroy } from 'svelte';
  import FactEditor from './FactEditor.svelte';
  import FactDetail from './FactDetail.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { canFacts, factDenied, factFailure } from '../lib/procedural-fact-errors.mjs';
  import { factDeclarationLabel, classLabels, outcomeLabels } from './fact-field-labels.mjs';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  export let api,
    user,
    caseId,
    family,
    resolution = null,
    ondenied,
    disabled = false,
    pending = false;
  const scoped =
    family === 'resolution'
      ? api.caseResolutions(caseId)
      : api.caseNotifications(caseId, resolution.id);
  const administration = caseState(),
    singular = family === 'resolution' ? 'resoluci\u00f3n' : 'notificaci\u00f3n';
  const plural = family === 'resolution' ? 'Resoluciones' : 'Notificaciones',
    collection = family === 'resolution' ? 'resolutions' : 'notifications';
  let rows = [],
    selected = null,
    historical = false,
    action = null,
    editorBase = null,
    editorKey = 0,
    notices = false;
  let status = 'all',
    applied = 'all',
    cursors = [undefined],
    index = 0,
    next,
    more = false,
    error = '',
    notice = '';
  let busy = false,
    opening = false,
    editorBusy = false,
    childPending = false,
    alive = true,
    listGeneration = 0,
    detailGeneration = 0;
  $: pending = busy || opening || editorBusy || childPending || !!action;
  $: manage = canFacts(user.role, 'manage') && !$administration.closed;
  function fail(failure) {
    error = factFailure(failure);
    if (factDenied(failure)) {
      listGeneration++;
      detailGeneration++;
      rows = [];
      selected = null;
      action = null;
      ondenied(failure);
    }
  }
  async function load(position = 0) {
    const request = ++listGeneration;
    busy = true;
    rows = [];
    error = '';
    index = position;
    more = false;
    try {
      const value = await scoped.list({ status: applied, afterId: cursors[position] });
      if (alive && request === listGeneration) {
        rows = value[collection];
        more = value.has_more;
        next = value.next_after_id;
        if (
          selected &&
          rows.some((row) => row.id === selected.id && row.revision > selected.revision)
        )
          historical = true;
      }
    } catch (failure) {
      if (alive && request === listGeneration) fail(failure);
    } finally {
      if (alive && request === listGeneration) busy = false;
    }
  }
  async function open(id, revision) {
    const request = ++detailGeneration;
    opening = true;
    selected = null;
    notices = false;
    error = '';
    notice = '';
    try {
      const value =
        revision === undefined ? await scoped.get(id) : await scoped.revision(id, revision);
      if (alive && request === detailGeneration) {
        selected = value;
        historical = revision !== undefined;
      }
    } catch (failure) {
      if (alive && request === detailGeneration) fail(failure);
    } finally {
      if (alive && request === detailGeneration) opening = false;
    }
  }
  function edit(nextAction) {
    if (pending || disabled || !manage) return;
    editorBase = nextAction === 'record' ? null : selected;
    action = nextAction;
    editorKey++;
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
    notices = false;
    detailGeneration++;
    cursors = [undefined];
    load();
  }
  function rowLabel(row) {
    const declaration = family === 'resolution' ? row.class : row.outcome;
    return factDeclarationLabel(declaration, family === 'resolution' ? classLabels : outcomeLabels);
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    listGeneration++;
    detailGeneration++;
    pending = false;
    scoped.dispose();
  });
</script>

<section class="fact-collection" aria-label={plural}>
  <div class="section-heading">
    <h2>{plural}</h2>
    {#if canFacts(user.role, 'manage')}<button
        class="primary"
        disabled={disabled || pending || !manage}
        onclick={() => edit('record')}>Registrar {singular}</button
      >{/if}
  </div>
  {#if family === 'notification'}<p class="hint">
      Cada pr&#225;ctica conserva una revisi&#243;n exacta de la misma resoluci&#243;n.
    </p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
  {#if action}{#key editorKey}<FactEditor
        {api}
        {caseId}
        {user}
        {family}
        {resolution}
        {action}
        record={editorBase}
        {ondenied}
        {disabled}
        onconfirmed={confirmed}
        oncancel={() => (action = null)}
        bind:pending={editorBusy}
      />{/key}{/if}
  <section class="card" aria-label={`${plural} registradas`} aria-busy={busy || opening}>
    <div class="section-heading">
      <h3>Registros disponibles</h3>
      <button class="secondary" disabled={disabled || pending} onclick={reset}
        >Actualizar registros</button
      >
    </div>
    <div class="action-row">
      <label
        >Estado del registro<select bind:value={status} disabled={disabled || pending}
          ><option value="all">Todos</option><option value="recorded">Registrados</option><option
            value="withdrawn">Retirados</option
          ></select
        ></label
      >
      <button
        class="secondary"
        disabled={disabled || pending}
        onclick={() => {
          applied = status;
          reset();
        }}>Aplicar estado</button
      >
    </div>
    <p class="hint">Orden por identificador; no indica el orden procesal de los actos.</p>
    {#each rows as row (row.id)}<button
        class="case-card fact-row"
        disabled={disabled || pending}
        aria-label={`Consultar ${singular} ${row.id}`}
        onclick={() => open(row.id)}
      >
        <span
          ><strong>{rowLabel(row)}</strong><span
            >{factTimeLabel(family === 'resolution' ? row.issued_at : row.practiced_at)}</span
          ><small>Revisi&#243;n {row.revision}</small></span
        >
        <span class="badge" class:info={row.status === 'recorded'}
          >{row.status === 'recorded' ? 'Registrado' : 'Retirado'}</span
        >
      </button>{/each}
    {#if !rows.length && !busy && !error}<p>No hay registros en esta consulta.</p>{/if}
    {#if busy}<p role="status">Consultando registros...</p>{/if}
    <div class="action-row">
      <button
        class="secondary"
        disabled={disabled || pending || !index}
        onclick={() => {
          selected = null;
          detailGeneration++;
          load(index - 1);
        }}>Registros anteriores</button
      >
      <button
        class="secondary"
        disabled={disabled || pending || !more}
        onclick={() => {
          selected = null;
          detailGeneration++;
          cursors = [...cursors.slice(0, index + 1), next];
          load(index + 1);
        }}>Siguientes registros</button
      >
    </div>
  </section>
  {#if selected}{#key `${selected.id}:${selected.revision}:${historical}`}<FactDetail
        record={selected}
        {family}
        api={scoped}
        {ondenied}
        disabled={disabled || pending}
        canManage={manage}
        {historical}
        onedit={edit}
        onselect={open}
      />{/key}
    {#if family === 'resolution'}<button
        class="secondary fact-notification-toggle"
        disabled={disabled || pending}
        aria-expanded={notices}
        onclick={() => (notices = !notices)}
        >{notices ? 'Ocultar notificaciones' : 'Ver notificaciones'}</button
      >
      {#if notices}{#key `${selected.id}:${selected.revision}`}<svelte:self
            {api}
            {caseId}
            {user}
            family="notification"
            resolution={selected}
            {ondenied}
            disabled={disabled || busy || opening || editorBusy || !!action}
            bind:pending={childPending}
          />{/key}{/if}
    {/if}
  {/if}
</section>
