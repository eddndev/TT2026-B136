<script>
  import { onMount, onDestroy } from 'svelte';
  import MeasureRecordSummary from './MeasureRecordSummary.svelte';
  import { measureKinds } from '../lib/measure-presentation.mjs';
  import { factDenied, factFailure } from '../lib/procedural-fact-errors.mjs';
  export let api,
    onselected,
    oncancel,
    ondenied,
    selectedIds = [],
    disabled = false,
    busy = false,
    canApply = () => true;
  let alive = true,
    generation = 0,
    rows = [],
    exact = null,
    error = '',
    more = false,
    nextId,
    cursors = [undefined],
    index = 0;
  async function work(operation) {
    if (disabled || busy || !canApply()) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const result = await operation();
      return alive && canApply() && request === generation ? result : null;
    } catch (failure) {
      if (alive && canApply() && request === generation) {
        error = factFailure(failure);
        if (factDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function load(position = 0) {
    const query = { limit: 10 };
    if (cursors[position] !== undefined) query.afterId = cursors[position];
    const page = await work(() => api.list(query));
    if (page) {
      rows = page.items;
      more = page.has_more;
      nextId = page.next_after_id;
      index = position;
      exact = null;
    }
  }
  async function open(row) {
    exact = null;
    const value = await work(() => api.exact(row.reference));
    if (value) exact = value;
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
    busy = false;
  });
</script>

<section
  class="case-comparison hearing-picker"
  aria-label="Seleccionar medida exacta"
  aria-busy={busy}
>
  <h4>Seleccionar medida exacta</h4>
  <p>Consulta la captura elegida antes de utilizarla como referencia.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each rows as row (row.reference.id)}
    <div class="case-comparison">
      <p>
        {measureKinds[row.record.capture.result.values.kind]} / {row.record.capture.result
          .projection.subject.display_name}
      </p>
      <p>Revisi&#243;n {row.reference.revision}</p>
      {#if row.validity === 'entered_in_error'}<p>Captura registrada por error</p>{/if}
      <button
        class="secondary"
        disabled={disabled || busy}
        aria-label={`Consultar medida ${row.reference.id}`}
        onclick={() => open(row)}>Consultar medida</button
      >
    </div>
  {/each}
  {#if !rows.length && !busy && !error}<p>No hay medidas en esta consulta.</p>{/if}
  <div class="action-row">
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
  {#if exact}<MeasureRecordSummary value={exact} />
    <button
      class="primary"
      disabled={disabled ||
        busy ||
        exact.validity !== 'valid' ||
        selectedIds.includes(exact.reference.id)}
      onclick={() => {
        if (canApply()) onselected(exact);
      }}>Vincular esta revision</button
    >
  {/if}
  <button class="text-button" disabled={busy} onclick={oncancel}>Cerrar selector de medidas</button>
</section>
