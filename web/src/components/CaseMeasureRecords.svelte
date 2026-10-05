<script>
  import { onMount, onDestroy } from 'svelte';
  import MeasureRecordSummary from './MeasureRecordSummary.svelte';
  import { measureKinds } from '../lib/measure-presentation.mjs';
  import { factDenied, factFailure } from '../lib/procedural-fact-errors.mjs';
  export let api,
    caseId,
    ondenied,
    disabled = false;
  const scoped = api.caseMeasures(caseId);
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
  });
</script>

<section class="card hearing-index" aria-label="Registros de medidas" aria-busy={busy}>
  <h2>Registros de medidas</h2>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each rows as row (row.reference.id)}<div class="case-comparison">
      <p>
        {measureKinds[row.record.capture.result.values.kind]} / {row.record.capture.result
          .projection.subject.display_name}
      </p>
      <button
        class="secondary"
        disabled={disabled || busy}
        aria-label={`Consultar medida ${row.reference.id}`}
        onclick={() => open(row)}>Consultar medida</button
      >
    </div>{/each}
  {#if !rows.length && !busy && !error}<p>No hay medidas registradas.</p>{/if}
  <div class="pagination action-row">
    <button class="secondary" disabled={disabled || busy || !index} onclick={() => load(index - 1)}
      >Medidas anteriores</button
    >
    <button
      class="secondary"
      disabled={disabled || busy || !more}
      onclick={() => {
        cursors = [...cursors.slice(0, index + 1), nextId];
        load(index + 1);
      }}>Siguientes medidas</button
    >
  </div>
  {#if selected}<MeasureRecordSummary value={selected} />
    <details>
      <summary>Historia de la medida</summary>
      {#each captures as row (row.result.revision)}<button
          class="secondary"
          disabled={disabled || busy}
          onclick={() =>
            revision({
              id: row.result.id,
              revision: row.result.revision,
              capture_digest: row.capture_digest,
            })}
        >
          Consultar medida revision {row.result.revision}</button
        >{/each}
    </details>
    <button class="text-button" disabled={disabled || busy} onclick={() => (selected = head = null)}
      >Cerrar medida</button
    >
  {/if}
</section>
