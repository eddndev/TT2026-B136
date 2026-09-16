<script>
  import { onMount, onDestroy } from 'svelte';
  import HearingReference from './HearingReference.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import { hearingDenied } from '../lib/hearings.mjs';
  export let api,
    typedApi,
    onselected,
    oncancel,
    ondenied,
    selectedIds = [],
    disabled = false,
    busy = false;
  let rows = [],
    exact = null,
    name = '',
    query = '',
    cursor,
    more = false,
    alive = true,
    error = '',
    generation = 0;
  async function work(operation) {
    if (disabled || busy) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const result = await operation();
      if (alive && request === generation) return result;
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if (hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function load(afterId) {
    const page = await work(() =>
      api.list({ status: 'active', name: query || undefined, limit: 20, afterId }),
    );
    if (page && alive) {
      rows = page.participants;
      exact = null;
      more = page.has_more;
      cursor = page.next_after_id;
    }
  }
  async function read(row) {
    exact = null;
    const result = await work(() => typedApi.participantRevision(row.id, row.revision));
    if (result && alive) {
      if (result.directory_status !== 'active')
        error = 'Selecciona una ficha activa del directorio actual.';
      else exact = result;
    }
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
  aria-label="Seleccionar participante exacto"
  aria-busy={busy}
>
  <h4>Seleccionar participante exacto</h4>
  <div class="action-row">
    <label>Nombre del participante<input bind:value={name} disabled={busy || disabled} /></label>
    <button
      type="button"
      class="secondary"
      disabled={busy || disabled}
      onclick={() => {
        query = name.trim();
        load();
      }}>Buscar participante</button
    >
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <div class="hearing-pick-list">
    {#each rows as row (row.id)}<div class="hearing-pick-reference">
        <button
          type="button"
          class="secondary"
          disabled={busy || disabled || selectedIds.includes(row.id)}
          onclick={() => read(row)}>Consultar ficha: {row.display_name}</button
        ><HearingReference {row} label="Referencia exacta" />
      </div>{/each}
  </div>
  {#if !rows.length && !busy && !error}<p>No hay fichas activas en esta consulta.</p>{/if}
  {#if more}<button
      type="button"
      class="secondary"
      disabled={busy || disabled}
      onclick={() => load(cursor)}>Siguientes participantes</button
    >{/if}
  {#if exact}<ParticipantSummary record={exact} />
    <HearingReference row={exact} />
    <button
      type="button"
      class="primary"
      disabled={busy || disabled}
      onclick={() => onselected(exact)}>Vincular esta revisi&#243;n</button
    >{/if}
  <button type="button" class="text-button" onclick={oncancel} disabled={busy}
    >Cerrar selector de participantes</button
  >
</section>
