<script>
  import { onMount, onDestroy } from 'svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import HearingReference from './HearingReference.svelte';
  import { hearingDenied } from '../lib/hearings.mjs';
  export let api,
    typedApi,
    candidates = [],
    selectedIds = [],
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  let roots = [],
    revisions = [],
    exact = null,
    selectedId = null,
    name = '',
    applied = '',
    rootCursor,
    historyCursor,
    rootMore = false,
    historyMore = false,
    error = '',
    alive = true,
    generation = 0;
  async function work(fn) {
    if (busy || disabled) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const value = await fn();
      if (alive && generation === request) return value;
    } catch (failure) {
      if (alive && generation === request) {
        error = failure.message;
        if (hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && generation === request) busy = false;
    }
  }
  async function list(afterId) {
    const value = await work(() =>
      api.list({ status: 'all', name: applied || undefined, afterId, limit: 20 }),
    );
    if (value) {
      roots = value.participants;
      rootMore = value.has_more;
      rootCursor = value.next_after_id;
      revisions = [];
      selectedId = null;
      exact = null;
    }
  }
  async function history(id, beforeRevision) {
    exact = null;
    const value = await work(() => api.history(id, { limit: 20, beforeRevision }));
    if (value) {
      selectedId = id;
      revisions = value.revisions;
      historyMore = value.has_more;
      historyCursor = value.next_before_revision;
    }
  }
  async function read(id, revision) {
    exact = null;
    const value = await work(() => typedApi.participantRevision(id, revision));
    if (value) exact = value;
  }
  onMount(() => list());
  onDestroy(() => {
    alive = false;
    generation++;
    busy = false;
  });
</script>

<section class="case-comparison" aria-label="Elegir ficha hist&#243;rica" aria-busy={busy}>
  <h4>Elegir ficha hist&#243;rica</h4>
  {#if candidates.length}<details>
      <summary>Participantes de la programaci&#243;n de origen</summary>
      <p class="hint">Elige expresamente cada comparecencia que deseas informar.</p>
      {#each candidates as row}<div class="hearing-result-picker-row">
          <p>{row.display_name} / Revisi&#243;n {row.revision}</p>
          <HearingReference {row} />
          <button
            type="button"
            class="secondary"
            disabled={busy || disabled || selectedIds.includes(row.id)}
            onclick={() => read(row.id, row.revision)}
            aria-label={`Consultar participante de origen ${row.id}`}
            >Consultar esta ficha de origen</button
          >
        </div>{/each}
    </details>{/if}
  <label
    >Nombre para buscar en el directorio<input
      bind:value={name}
      disabled={busy || disabled}
    /></label
  >
  <button
    class="secondary"
    type="button"
    disabled={busy || disabled}
    onclick={() => {
      applied = name.trim();
      list();
    }}>Buscar ficha hist&#243;rica</button
  >
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each roots as row}<div class="hearing-result-picker-row">
      <p>
        {row.display_name} / {row.directory_status === 'archived' ? 'Archivada' : 'Activa'} actualmente
      </p>
      <HearingReference {row} />
      <button
        type="button"
        class="secondary"
        disabled={busy || disabled || selectedIds.includes(row.id)}
        onclick={() => history(row.id)}
        aria-label={`Consultar historia de ficha ${row.id}`}
        >Consultar revisiones de la ficha</button
      >
    </div>{/each}
  {#if !roots.length && !busy && !error}<p>No hay fichas en esta consulta.</p>{/if}
  {#if rootMore}<button
      type="button"
      class="secondary"
      disabled={busy || disabled}
      onclick={() => list(rootCursor)}>Siguientes fichas</button
    >{/if}
  {#if selectedId}<h4>Revisiones de la ficha</h4>
    {#each revisions as row}<div class="hearing-result-picker-row">
        <p>{row.display_name} / Revisi&#243;n {row.revision}</p>
        <button
          type="button"
          class="secondary"
          disabled={busy || disabled}
          onclick={() => read(row.id, row.revision)}
          >Consultar ficha revisi&#243;n {row.revision}</button
        >
      </div>{/each}
    {#if historyMore}<button
        type="button"
        class="secondary"
        disabled={busy || disabled}
        onclick={() => history(selectedId, historyCursor)}>Revisiones anteriores de ficha</button
      >{/if}
  {/if}
  {#if exact}<ParticipantSummary record={exact} /><HearingReference row={exact} />
    <button
      type="button"
      class="primary"
      disabled={busy || disabled || selectedIds.includes(exact.id)}
      onclick={() => onselected(exact)}>Informar comparecencia de esta revisi&#243;n</button
    >{/if}
  <button type="button" class="text-button" disabled={busy} onclick={oncancel}
    >Cerrar selector hist&#243;rico</button
  >
</section>
