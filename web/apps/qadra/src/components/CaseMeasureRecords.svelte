<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import MeasureAdministrationEditor from './MeasureAdministrationEditor.svelte';
  import MeasureAdministrationDetail from './MeasureAdministrationDetail.svelte';
  import { pendingMeasureAdministrationDrafts } from '../lib/measure-administration-draft.mjs';
  import { factSame } from '../lib/procedural-fact-primitives.mjs';
  import { canFacts } from '../lib/procedural-fact-errors.mjs';
  import MeasureRecordSummary from './MeasureRecordSummary.svelte';
  import { measureKinds } from '../lib/measure-presentation.mjs';
  import { factDenied, factFailure } from '../lib/procedural-fact-errors.mjs';
  export let api,
    caseId,
    ondenied,
    user,
    manage = false,
    pending = false,
    disabled = false;
  const scoped = api.caseMeasures(caseId),
    operations = api.caseMeasureAdministrations(caseId),
    session = getContext('session-drafts');
  const actions = {
    correct: 'Rectificar registro',
    entered_in_error: 'Marcar registro por error',
    replace_entered_in_error: 'Corregir identidad registrada',
  };
  let editing = false,
    action = null,
    base = null,
    savedDraft = null,
    editorBusy = false,
    editorGeneration = 0,
    detail = null,
    drafts = pendingMeasureAdministrationDrafts(session, caseId);
  let alive = true,
    busy = false,
    rows = [],
    selected = null,
    head = null,
    error = '',
    cursors = [undefined],
    index = 0,
    more = false,
    nextId;
  $: pending = editing || busy || editorBusy;
  $: locked = disabled || pending;
  $: current =
    selected?.validity === 'valid' && head && factSame(selected.reference, head.reference);
  function edit(kind, saved = null) {
    if (locked || (saved ? !session?.canAdmit() : !manage || !current)) return;
    action = kind;
    savedDraft = saved;
    base = saved ? null : structuredClone(selected);
    detail = null;
    editing = true;
    editorGeneration++;
  }
  function closeEditor() {
    editing = false;
    savedDraft = null;
    drafts = pendingMeasureAdministrationDrafts(session, caseId);
  }
  async function confirmed(value) {
    if (!alive) return;
    detail = value;
    selected = head = null;
    cursors = [undefined];
    await load();
    if (alive) closeEditor();
  }
  async function original(operationId) {
    const value = await work(() => operations.get(operationId));
    if (value) detail = value;
  }
  async function work(fn) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const result = await fn();
      return alive ? result : null;
    } catch (failure) {
      if (alive) {
        error = factFailure(failure);
        if (factDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function load(position = 0) {
    const query = { limit: 10 };
    if (cursors[position] !== undefined) query.afterId = cursors[position];
    const page = await work(() => scoped.list(query));
    if (page) {
      rows = page.items;
      index = position;
      more = page.has_more;
      nextId = page.next_after_id;
    }
  }
  async function open(row) {
    const value = await work(() => scoped.exact(row.reference));
    if (value) head = selected = value;
  }
  async function revision(reference) {
    const value = await work(() => scoped.exact(reference));
    if (value) selected = value;
  }
  $: captures = head
    ? [
        ...head.record_history.records.judicial.groups.flatMap((entry) => entry.capture.measures),
        ...head.record_history.decisions.flatMap((entry) => entry.capture.measures),
        ...head.record_history.records.administrative.flatMap((entry) => entry.capture.records),
      ]
        .filter((row) => row.result.id === head.reference.id)
        .sort((a, b) => a.result.revision - b.result.revision)
    : [];
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    scoped.dispose();
    operations.dispose();
    pending = false;
  });
</script>

<section class="card hearing-index" aria-label="Registros de medidas" aria-busy={busy}>
  <h2>Registros de medidas</h2>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !editing}{#each drafts as saved (saved.key)}<button
        class="secondary"
        disabled={locked}
        onclick={() => edit(saved.action, saved)}>Retomar rectificacion de medida</button
      >{/each}{/if}
  {#each rows as row (row.reference.id)}<div class="case-comparison">
      <p>
        {measureKinds[row.record.capture.result.values.kind]} / {row.record.capture.result
          .projection.subject.display_name}
      </p>
      <button
        class="secondary"
        disabled={locked}
        aria-label={`Consultar medida ${row.reference.id}`}
        onclick={() => open(row)}>Consultar medida</button
      >
    </div>{/each}
  {#if !rows.length && !busy && !error}<p>No hay medidas registradas.</p>{/if}
  <div class="pagination action-row">
    <button class="secondary" disabled={locked || !index} onclick={() => load(index - 1)}
      >Medidas anteriores</button
    >
    <button
      class="secondary"
      disabled={locked || !more}
      onclick={() => {
        cursors = [...cursors.slice(0, index + 1), nextId];
        load(index + 1);
      }}>Siguientes medidas</button
    >
  </div>
  {#if selected}<MeasureRecordSummary value={selected} />
    {#if canFacts(user.role, 'manage')}<div class="action-row">
        {#each Object.entries(actions) as [kind, label]}<button
            class="secondary"
            disabled={locked || !manage || !current}
            onclick={() => edit(kind)}>{label}</button
          >{/each}
      </div>{/if}
    <details>
      <summary>Historia de la medida</summary>
      {#each captures as row (row.result.revision)}<button
          class="secondary"
          disabled={locked}
          onclick={() =>
            revision({
              id: row.result.id,
              revision: row.result.revision,
              capture_digest: row.capture_digest,
            })}
        >
          Consultar medida revision {row.result.revision}</button
        >{/each}
      {#each head.record_history.records.administrative as row (row.origin.operation_id)}
        <button
          class="secondary"
          disabled={locked}
          onclick={() => original(row.origin.operation_id)}
          aria-label={`Consultar rectificacion ${row.origin.operation_id}`}
          >Consultar rectificacion</button
        >
      {/each}
    </details>
    <button class="text-button" disabled={locked} onclick={() => (selected = head = null)}
      >Cerrar medida</button
    >
  {/if}
</section>

{#if editing}{#key editorGeneration}<MeasureAdministrationEditor
      {api}
      {caseId}
      {user}
      {action}
      {base}
      {savedDraft}
      {disabled}
      bind:pending={editorBusy}
      {ondenied}
      onconfirmed={confirmed}
      oncancel={closeEditor}
    />{/key}{:else if detail}<MeasureAdministrationDetail
    value={detail}
    onclose={() => (detail = null)}
  />{/if}
