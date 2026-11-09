<script>
  import Button from './Button.svelte';
  import { getContext, onMount, onDestroy } from 'svelte';
  import MeasureDecisionEditor from './MeasureDecisionEditor.svelte';
  import MeasureDecisionDetail from './MeasureDecisionDetail.svelte';
  import CaseMeasureRecords from './CaseMeasureRecords.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { canFacts, factDenied } from '../lib/procedural-fact-errors.mjs';
  import { decisionEditorFailure } from './measure-decision-editor-values.mjs';
  import { pendingMeasureDecisionDrafts } from '../lib/measure-decision-draft.mjs';
  import { measureTimeLabel } from '../lib/measure-presentation.mjs';
  export let api, user, record, ondenied;
  const scoped = api.caseMeasureDecisions(record.id),
    contextApi = api.casePrecautionaryHearings(record.id),
    administration = caseState(),
    session = getContext('session-drafts');
  let alive = true,
    generation = 0,
    context = null,
    rows = [],
    selected = null,
    busy = false,
    editorBusy = false,
    recordsBusy = false,
    editing = false,
    savedDraft = null,
    error = '',
    editorGeneration = 0,
    refreshRecords = 0,
    cursors = [undefined],
    index = 0,
    more = false,
    nextId;
  let drafts = pendingMeasureDecisionDrafts(session, record.id);
  $: locked = busy || editorBusy || editing || recordsBusy;
  $: manage =
    canFacts(user.role, 'manage') &&
    !$administration.closed &&
    context?.administration.administrative_status === 'active';
  function fail(failure) {
    error = decisionEditorFailure(failure);
    if (factDenied(failure)) {
      rows = [];
      selected = null;
      editing = false;
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
      const [current, page] = await Promise.all([contextApi.context(), scoped.list(query)]);
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
      const value = await scoped.get(id);
      if (alive && request === generation) selected = value;
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function edit(saved = null) {
    if (locked || (saved ? !session?.canAdmit() : !manage)) return;
    savedDraft = saved;
    editing = true;
    editorGeneration++;
  }
  function closeEditor() {
    editing = false;
    savedDraft = null;
    drafts = pendingMeasureDecisionDrafts(session, record.id);
  }
  async function confirmed(value) {
    if (!alive) return;
    selected = value;
    cursors = [undefined];
    await load();
    if (alive) {
      closeEditor();
      refreshRecords++;
    }
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
    contextApi.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">Expediente</span>
    <h1>Medidas cautelares</h1>
  </div>
</div>
<section class="card hearing-index" aria-label="Medidas cautelares" aria-busy={busy}>
  <div class="section-heading">
    <h2>Decisiones declaradas</h2>
    {#if canFacts(user.role, 'manage')}<Button
        variant="primary"
        disabled={locked || !manage}
        onclick={() => edit()}>Registrar decisi&#243;n cautelar</Button
      >{/if}
  </div>
  <p>
    Conserva la &#250;ltima declaraci&#243;n registrada y sus fuentes. El estado del expediente no
    determina la vigencia jur&#237;dica.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !editing}{#each drafts as saved (saved.key)}<Button
        variant="secondary"
        disabled={locked}
        onclick={() => edit(saved)}>Retomar decisi&#243;n cautelar</Button
      >{/each}{/if}
  {#if busy}<p role="status">Consultando decisiones...</p>{/if}
  {#each rows as row (row.origin.decision_id)}
    <div class="case-comparison">
      <h3>{row.group.decision.values.authority}</h3>
      <p>{measureTimeLabel(row.group.decision.values.declared_at)}</p>
      <p class="case-multiline">{row.group.decision.values.justification}</p>
      <Button
        variant="secondary"
        disabled={locked}
        aria-label={`Consultar decisi\u00f3n cautelar ${row.origin.decision_id}`}
        onclick={() => open(row.origin.decision_id)}>Consultar decisi&#243;n cautelar</Button
      >
    </div>
  {/each}
  {#if !rows.length && !busy && !error}<p>No hay decisiones cautelares registradas.</p>{/if}
  <div class="pagination action-row">
    <Button variant="secondary" disabled={locked || !index} onclick={() => load(index - 1)}
      >Decisiones anteriores</Button
    >
    <Button
      variant="secondary"
      disabled={locked || !more}
      onclick={() => {
        cursors = [...cursors.slice(0, index + 1), nextId];
        load(index + 1);
      }}>Siguientes decisiones</Button
    >
    <Button
      variant="secondary"
      disabled={locked}
      onclick={() => {
        cursors = [undefined];
        load();
        refreshRecords++;
      }}>Actualizar decisiones</Button
    >
  </div>
</section>
{#if editing}{#key editorGeneration}<MeasureDecisionEditor
      {api}
      caseId={record.id}
      {user}
      {savedDraft}
      bind:pending={editorBusy}
      ondenied={fail}
      onconfirmed={confirmed}
      oncancel={closeEditor}
    />{/key}
{:else if selected}<MeasureDecisionDetail
    value={selected}
    {record}
    onclose={() => (selected = null)}
  />{/if}
{#key refreshRecords}<CaseMeasureRecords
    {api}
    caseId={record.id}
    {user}
    {manage}
    bind:pending={recordsBusy}
    disabled={busy || editorBusy || editing}
    ondenied={fail}
  />{/key}
