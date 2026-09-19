<script>
  import { onMount, onDestroy } from 'svelte';
  import DeadlineEditor from './DeadlineEditor.svelte';
  import DeadlineDetail from './DeadlineDetail.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { canDeadlines, deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { deadlineOperationalLabel, deadlineReviewLabels } from './deadline-view-labels.mjs';
  export let api, user, caseId, ondenied;
  export let intent = null,
    onintent = () => {};
  const scoped = api.deadlines(caseId),
    administration = caseState();
  let rows = [],
    selected = null,
    historical = false,
    mode = null,
    base = null,
    editorKey = 0;
  let status = 'active',
    applied = 'active',
    cursors = [undefined],
    index = 0,
    next,
    more = false,
    error = '',
    notice = '';
  let busy = false,
    opening = false,
    editorBusy = false,
    alive = true,
    listGeneration = 0,
    detailGeneration = 0;
  let initialized = false,
    consumed;
  $: manage = canDeadlines(user.role, 'manage') && !$administration.closed;
  $: pending = busy || opening || editorBusy || !!mode;
  $: if (initialized && intent && intent !== consumed) {
    consumed = intent;
    if (intent.case_id === caseId) open(intent.deadline_id, intent.revision);
    onintent();
  }
  function fail(failure) {
    error = deadlineFailure(failure);
    if (deadlineDenied(failure)) {
      listGeneration++;
      detailGeneration++;
      rows = [];
      selected = null;
      mode = null;
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
        rows = value.deadlines;
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
  function edit(action) {
    if (pending || !manage) return;
    base = action === 'register' ? null : selected;
    mode = action;
    editorKey++;
  }
  async function saved(value, exact = false) {
    if (!alive) return;
    selected = value;
    historical = exact;
    notice = 'Plazo guardado.';
    cursors = [undefined];
    await load();
    if (alive) mode = null;
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
    listGeneration++;
    detailGeneration++;
    scoped.dispose();
  });
</script>

<section class="fact-collection deadline-collection" aria-label="Plazos">
  <div class="section-heading">
    <h2>Seguimiento de plazos</h2>
    {#if canDeadlines(user.role, 'manage')}<button
        class="primary"
        disabled={pending || !manage}
        onclick={() => edit('register')}>Registrar plazo</button
      >{/if}
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
  {#if mode}{#key editorKey}<DeadlineEditor
        {api}
        {user}
        {caseId}
        {base}
        {mode}
        {ondenied}
        onsaved={saved}
        oncancel={() => (mode = null)}
        bind:pending={editorBusy}
      />{/key}{/if}
  <section class="card" aria-label="Plazos registrados" aria-busy={busy || opening}>
    <div class="section-heading">
      <h3>Registros disponibles</h3>
      <button class="secondary" disabled={pending} onclick={reset}>Actualizar plazos</button>
    </div>
    <div class="action-row">
      <label
        >Estado del plazo<select bind:value={status} disabled={pending}>
          <option value="active">Activos</option><option value="all">Todos</option><option
            value="retired">Retirados</option
          >
        </select></label
      >
      <button
        class="secondary"
        disabled={pending}
        onclick={() => {
          applied = status;
          reset();
        }}>Aplicar estado</button
      >
    </div>
    <p class="hint">
      Orden por identificador. El seguimiento consultado y el c&#225;lculo conservado se muestran
      por separado.
    </p>
    {#each rows as row (row.id)}<button
        class="case-card fact-row deadline-row"
        disabled={pending}
        aria-label={`Consultar plazo ${row.id}`}
        onclick={() => open(row.id)}
      >
        <span
          ><strong>{row.title}</strong><span>{row.responsible.email}</span>
          <small
            >Revisi&#243;n {row.revision} / {row.attention_recorded
              ? 'Atenci\u00f3n declarada'
              : 'Atenci\u00f3n pendiente'}</small
          >
          <span class="deadline-operational"
            >Vencimiento para seguimiento:
            {deadlineOperationalLabel(row.operational, row.status, row.review_state)}</span
          >
          <small
            >C&#225;lculo conservado: {row.calculation_due_at
              ? deadlineInstantLabel(row.calculation_due_at)
              : 'Sin vencimiento calculado'}</small
          >
          <small>{deadlineReviewLabels[row.review_state]}</small></span
        >
        <span class="badge" class:info={row.status === 'active'}
          >{row.status === 'retired' ? 'Retirado' : 'Activo'}</span
        >
      </button>{/each}
    {#if !rows.length && !busy && !error}<p>No hay plazos en esta consulta.</p>{/if}
    {#if busy}<p role="status">Consultando plazos...</p>{/if}
    <div class="action-row">
      <button
        class="secondary"
        disabled={pending || !index}
        onclick={() => {
          selected = null;
          detailGeneration++;
          load(index - 1);
        }}>Plazos anteriores</button
      >
      <button
        class="secondary"
        disabled={pending || !more}
        onclick={() => {
          selected = null;
          detailGeneration++;
          cursors = [...cursors.slice(0, index + 1), next];
          load(index + 1);
        }}>Plazos siguientes</button
      >
    </div>
  </section>
  {#if selected}{#key `${selected.id}:${selected.revision}`}<DeadlineDetail
        {api}
        {scoped}
        record={selected}
        {historical}
        canManage={manage}
        disabled={pending}
        onedit={edit}
        onselect={open}
        ondenied={fail}
      />{/key}{/if}
</section>
