<script>
  import { getContext, onDestroy } from 'svelte';
  import { pendingHearingDrafts, resultDraftIntent } from '../lib/hearing-draft.mjs';
  import HearingResultEditor from './HearingResultEditor.svelte';
  import HearingDerivedDeadlineEditor from './HearingDerivedDeadlineEditor.svelte';
  import HearingDerivedDeadlineConfirmation from './HearingDerivedDeadlineConfirmation.svelte';
  import { pendingHearingDerivedDeadlineDrafts } from '../lib/hearing-derived-deadline-draft.mjs';
  import { canDeadlines } from '../lib/deadline-errors.mjs';
  import HearingResultDetail from './HearingResultDetail.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { hearingResultTimeLabel } from '../lib/hearing-result-time.mjs';
  import { resultStatus, resultOccurrence, resultExtent } from '../lib/hearing-result-errors.mjs';
  import { hearingDenied } from '../lib/hearings.mjs';
  export let api,
    caseId,
    user,
    hearing,
    canManage = false,
    ondenied,
    onopenhearing,
    disabled = false,
    pending = false,
    intent = null,
    onintent = () => {};
  const scoped = api.caseHearingResults(caseId, hearing.id),
    administration = caseState(),
    session = getContext('session-drafts');
  let savedDraft = null,
    drafts = pendingHearingDrafts(session, caseId, true, hearing.id);
  let derivedDrafts = pendingHearingDerivedDeadlineDrafts(session, caseId, hearing.id),
    compound = null;
  let expanded = false,
    rows = [],
    selected = null,
    historical = false,
    action = null,
    editorBase = null,
    continuation = null,
    editorKey = 0;
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
    alive = true,
    listGeneration = 0,
    detailGeneration = 0,
    consumed;
  $: pending = busy || opening || editorBusy || !!action;
  $: manage = canManage && !$administration.closed;
  $: if (intent && intent !== consumed) {
    consumed = intent;
    expanded = true;
    consume(intent);
    onintent();
  }
  async function consume(value) {
    await load();
    if (alive) open(value.result_id, value.revision);
  }
  function fail(failure) {
    error = failure.message;
    if (hearingDenied(failure)) {
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
        rows = value.results;
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
  function edit(next, previous = null) {
    if (pending || disabled || !manage) return;
    editorBase = next === 'record' ? null : selected;
    continuation = previous
      ? {
          hearing_id: previous.hearing_id,
          result_id: previous.id,
          revision: previous.revision,
          values_digest: previous.values_digest,
          submission_digest: previous.receipt.submission_digest,
          status: previous.status,
        }
      : null;
    savedDraft =
      drafts.find(
        (row) =>
          row.action === next &&
          row.resourceId === (editorBase?.id ?? null) &&
          (next !== 'record' || row.instanceId === resultDraftIntent(hearing.id, continuation)),
      ) ?? null;
    action = next;
    editorKey++;
  }
  function createDerived() {
    if (pending || disabled || !manage || !canDeadlines(user.role, 'manage')) return;
    savedDraft = derivedDrafts[0] ?? null;
    action = 'derived';
    compound = null;
    editorKey++;
  }
  async function resume(saved) {
    if (pending || disabled) return;
    opening = true;
    try {
      if (saved.editorKind === 'hearing-derived-deadline') {
        if (!alive || !session?.canAdmit() || !canDeadlines(user.role, 'manage')) return;
        savedDraft = saved;
        action = 'derived';
        compound = null;
        editorKey++;
        return;
      }
      editorBase = saved.resourceId ? await scoped.get(saved.resourceId) : null;
      if (!alive || !session?.canAdmit()) return;
      savedDraft = saved;
      continuation = null;
      action = saved.action;
      editorKey++;
    } catch (failure) {
      if (alive) {
        if (failure.status === 404 && failure.code === 'hearing_result_not_found') {
          session?.registry.closeEditor(saved.key);
          drafts = pendingHearingDrafts(session, caseId, true, hearing.id);
        }
        fail(failure);
      }
    } finally {
      if (alive) opening = false;
    }
  }
  function closedEditor() {
    action = null;
    savedDraft = null;
    drafts = pendingHearingDrafts(session, caseId, true, hearing.id);
    derivedDrafts = pendingHearingDerivedDeadlineDrafts(session, caseId, hearing.id);
  }
  async function confirmedCompound(record) {
    compound = record;
    await confirmed(record.result, true);
    if (alive) notice = 'Resultado y plazo guardados.';
  }
  async function confirmed(value, exact) {
    if (!alive) return;
    if (value.hearing_id !== hearing.id) {
      onopenhearing(value.hearing_id, value.anchor.revision, {
        result_id: value.id,
        revision: value.revision,
      });
      return;
    }
    selected = value;
    historical = exact;
    notice = 'Resultado declarado guardado.';
    cursors = [undefined];
    await load();
    if (alive) closedEditor();
  }
  function previous(value) {
    onopenhearing(value.hearing_id, null, { result_id: value.result_id, revision: value.revision });
  }
  function reset() {
    selected = null;
    detailGeneration++;
    cursors = [undefined];
    load();
  }
  onDestroy(() => {
    alive = false;
    listGeneration++;
    detailGeneration++;
    pending = false;
    scoped.dispose();
  });
</script>

<div class="hearing-result-toggle">
  <button
    class="secondary"
    disabled={disabled || pending}
    aria-expanded={expanded}
    onclick={() => {
      expanded = !expanded;
      if (expanded) load();
    }}>{expanded ? 'Ocultar sesiones y resultados' : 'Ver sesiones y resultados'}</button
  >
</div>
{#if expanded}<section class="hearing-results" aria-label="Sesiones y resultados declarados">
    <div class="section-heading">
      <div>
        <h2>Sesiones y resultados declarados</h2>
        <p>Cada sesi&#243;n o acto tiene su propio registro y revisiones.</p>
      </div>
      {#if canManage}<button
          class="primary"
          disabled={disabled || pending || !manage}
          onclick={() => edit('record')}>Registrar sesi&#243;n o acto</button
        >{/if}
      {#if canManage && canDeadlines(user.role, 'manage')}<button
          class="primary"
          disabled={disabled || pending || !manage}
          onclick={createDerived}>Registrar resultado y plazo</button
        >{/if}
    </div>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}{#if notice}<p
        class="notice success"
        role="status"
      >
        {notice}
      </p>{/if}
    {#if compound}<HearingDerivedDeadlineConfirmation record={compound} />{/if}
    {#if action === 'derived'}{#key editorKey}<HearingDerivedDeadlineEditor
          {api}
          {caseId}
          {user}
          {hearing}
          {savedDraft}
          {ondenied}
          {disabled}
          onconfirmed={confirmedCompound}
          oncancel={closedEditor}
          bind:pending={editorBusy}
        />{/key}
    {:else if action}{#key editorKey}<HearingResultEditor
          {api}
          {caseId}
          {user}
          {hearing}
          {action}
          {savedDraft}
          record={editorBase}
          {continuation}
          {ondenied}
          {disabled}
          onconfirmed={confirmed}
          oncancel={closedEditor}
          bind:pending={editorBusy}
        />{/key}{/if}
    {#if !action}{#each derivedDrafts as saved (saved.key)}<button
          class="secondary"
          disabled={disabled || pending}
          onclick={() => resume(saved)}>Recuperar resultado y plazo</button
        >{/each}{/if}
    {#if !action}{#each drafts as saved (saved.key)}<button
          class="secondary"
          disabled={disabled || pending}
          onclick={() => resume(saved)}>Retomar borrador de resultado</button
        >{/each}{/if}
    <section class="card" aria-label="Registros de sesiones y actos" aria-busy={busy || opening}>
      <div class="section-heading">
        <h3>Registros de esta audiencia</h3>
        <button class="secondary" disabled={disabled || pending} onclick={reset}
          >Actualizar resultados</button
        >
      </div>
      <div class="action-row">
        <label
          >Estado del registro<select bind:value={status} disabled={disabled || pending}
            ><option value="all">Todos</option><option value="recorded">Registrados</option><option
              value="withdrawn">Retirados</option
            ></select
          ></label
        ><button
          class="secondary"
          disabled={disabled || pending}
          onclick={() => {
            applied = status;
            reset();
          }}>Aplicar estado del resultado</button
        >
      </div>
      <p class="hint">Orden por identificador; no representa el orden procesal de los actos.</p>
      {#each rows as row (row.id)}<button
          class="case-card hearing-result-row"
          disabled={disabled || pending}
          aria-label={`Consultar resultado ${row.id}`}
          onclick={() => open(row.id)}
        >
          <span
            ><strong>{resultOccurrence[row.occurrence]} / {resultExtent[row.extent]}</strong><span
              >{hearingResultTimeLabel(row.event_time)}</span
            ><small
              >Revisi&#243;n {row.revision} / {row.attendee_count} comparecencias / {row.agreement_count}
              acuerdos</small
            ></span
          ><span class="badge" class:info={row.status === 'recorded'}
            >{resultStatus[row.status]}</span
          >
        </button>{/each}
      {#if !rows.length && !busy && !error}<p>
          No hay sesiones o actos registrados en esta consulta.
        </p>{/if}
      {#if busy}<p role="status">Consultando registros de sesiones...</p>{/if}
      <div class="action-row">
        <button
          class="secondary"
          disabled={disabled || pending || !index}
          onclick={() => {
            selected = null;
            detailGeneration++;
            load(index - 1);
          }}>Resultados anteriores</button
        >
        <button
          class="secondary"
          disabled={disabled || pending || !more}
          onclick={() => {
            selected = null;
            detailGeneration++;
            cursors = [...cursors.slice(0, index + 1), next];
            load(index + 1);
          }}>Siguientes resultados</button
        >
      </div>
    </section>
    {#if selected}{#key `${selected.id}:${selected.revision}:${historical}`}<HearingResultDetail
          record={selected}
          api={scoped}
          {ondenied}
          disabled={disabled || pending}
          canManage={manage}
          {historical}
          onedit={edit}
          oncontinue={() => edit('record', selected)}
          onprevious={previous}
          onselect={open}
        />{/key}{/if}
  </section>{/if}
