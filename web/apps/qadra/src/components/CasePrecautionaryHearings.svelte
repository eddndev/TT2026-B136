<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import PrecautionaryHearingEditor from './PrecautionaryHearingEditor.svelte';
  import PrecautionaryHearingDetail from './PrecautionaryHearingDetail.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { canFacts, factDenied } from '../lib/procedural-fact-errors.mjs';
  import { precautionaryEditorFailure } from './precautionary-hearing-editor-values.mjs';
  import { precautionaryHearingPurposes } from '../lib/precautionary-hearing-presentation.mjs';
  import { hearingTimeLabel, hearingStatus } from '../lib/hearings.mjs';
  import { pendingPrecautionaryHearingDrafts } from '../lib/precautionary-hearing-draft.mjs';
  export let api,
    user,
    record,
    ondenied,
    disabled = false,
    pending = false;
  const scoped = api.casePrecautionaryHearings(record.id),
    administration = caseState(),
    session = getContext('session-drafts');
  let alive = true,
    generation = 0,
    context = null,
    rows = [],
    selected = null,
    busy = false,
    editorBusy = false,
    action = null,
    base = null,
    savedDraft = null,
    error = '',
    editorGeneration = 0,
    cursors = [undefined],
    index = 0,
    more = false,
    nextId;
  let drafts = pendingPrecautionaryHearingDrafts(session, record.id);
  $: pending = busy || editorBusy || !!action;
  $: locked = disabled || pending;
  $: manage =
    canFacts(user.role, 'manage') &&
    !$administration.closed &&
    context?.administration.administrative_status === 'active';
  $: editable = manage && selected?.capture.review.status === 'scheduled';
  function fail(failure) {
    error = precautionaryEditorFailure(failure);
    if (factDenied(failure)) {
      rows = [];
      selected = null;
      action = null;
      ondenied(failure);
    }
  }
  async function load(position = 0) {
    const request = ++generation;
    busy = true;
    error = '';
    index = position;
    rows = [];
    more = false;
    try {
      const query = { limit: 10 };
      if (cursors[position] !== undefined) query.afterId = cursors[position];
      const [current, page] = await Promise.all([scoped.context(), scoped.list(query)]);
      if (!alive || request !== generation) return;
      context = current;
      rows = page.items;
      more = page.has_more;
      nextId = page.next_after_id;
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function open(id) {
    if (locked) return;
    const request = ++generation;
    busy = true;
    selected = null;
    error = '';
    try {
      const result = await scoped.get(id);
      if (alive && request === generation) selected = result;
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function edit(next) {
    if (locked || !manage || (next !== 'schedule' && !editable)) return;
    base = next === 'schedule' ? null : selected;
    savedDraft = null;
    action = next;
    editorGeneration++;
  }
  function resume(saved) {
    if (locked || !session?.canAdmit()) return;
    savedDraft = saved;
    base = null;
    action = saved.action;
    editorGeneration++;
  }
  function closedEditor() {
    action = null;
    savedDraft = null;
    drafts = pendingPrecautionaryHearingDrafts(session, record.id);
  }
  async function confirmed(value) {
    if (!alive) return;
    selected = value;
    cursors = [undefined];
    await load();
    if (alive) closedEditor();
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
    pending = false;
  });
</script>

<section class="card hearing-index" aria-label="Audiencias cautelares" aria-busy={busy}>
  <div class="section-heading">
    <h2>Audiencias cautelares</h2>
    {#if canFacts(user.role, 'manage')}<button
        class="primary"
        disabled={locked || !manage}
        onclick={() => edit('schedule')}>Programar audiencia cautelar</button
      >{/if}
  </div>
  <p>Convocatorias de imposici&#243;n y revisi&#243;n con soporte declarado.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !action}{#each drafts as saved (saved.key)}<button
        class="secondary"
        disabled={locked}
        onclick={() => resume(saved)}>Retomar convocatoria cautelar</button
      >{/each}{/if}
  {#if busy}<p role="status">Consultando convocatorias...</p>{/if}
  {#each rows as row (row.capture.review.command.hearing_id)}
    <div class="case-comparison">
      <h3>{precautionaryHearingPurposes[row.capture.review.resolved_values.purpose]}</h3>
      <p>
        {hearingTimeLabel(row.capture.review.resolved_values.scheduled_at)} / {hearingStatus[
          row.capture.review.status
        ]}
      </p>
      <p>{row.capture.review.resolved_values.venue}</p>
      <button
        class="secondary"
        disabled={locked}
        aria-label={`Consultar audiencia cautelar ${row.capture.review.command.hearing_id}`}
        onclick={() => open(row.capture.review.command.hearing_id)}
        >Consultar audiencia cautelar</button
      >
    </div>
  {/each}
  {#if !rows.length && !busy && !error}<p>No hay convocatorias cautelares registradas.</p>{/if}
  <div class="pagination action-row">
    <button class="secondary" disabled={locked || !index} onclick={() => load(index - 1)}
      >Convocatorias anteriores</button
    >
    <button
      class="secondary"
      disabled={locked || !more}
      onclick={() => {
        cursors = [...cursors.slice(0, index + 1), nextId];
        load(index + 1);
      }}>Siguientes convocatorias</button
    >
    <button
      class="secondary"
      disabled={locked}
      onclick={() => {
        cursors = [undefined];
        load();
      }}>Actualizar convocatorias</button
    >
  </div>
</section>
{#if action}{#key editorGeneration}<PrecautionaryHearingEditor
      {api}
      caseId={record.id}
      {user}
      {action}
      {base}
      {savedDraft}
      {disabled}
      bind:pending={editorBusy}
      ondenied={fail}
      onconfirmed={confirmed}
      oncancel={closedEditor}
    />{/key}{/if}
{#if selected && !action}
  <PrecautionaryHearingDetail
    value={selected}
    caseRecord={record}
    onclose={() => (selected = null)}
  />
  {#if canFacts(user.role, 'manage')}<div class="action-row">
      <button class="secondary" disabled={locked || !editable} onclick={() => edit('replace')}
        >Reprogramar audiencia cautelar</button
      >
      <button class="secondary" disabled={locked || !editable} onclick={() => edit('cancel')}
        >Cancelar audiencia cautelar</button
      >
    </div>{/if}
{/if}
