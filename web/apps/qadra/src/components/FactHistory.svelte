<script>
  import { onMount, onDestroy } from 'svelte';
  import { factDenied, factFailure } from '../lib/procedural-fact-errors.mjs';
  export let api,
    family,
    id,
    onselect,
    ondenied,
    disabled = false,
    busy = false;
  const singular = family === 'resolution' ? 'resoluci\u00f3n' : 'notificaci\u00f3n';
  let rows = [],
    more = false,
    cursor,
    error = '',
    alive = true,
    generation = 0;
  async function load(beforeRevision) {
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const value = await api.history(id, { limit: 10, beforeRevision });
      if (alive && request === generation) {
        rows = beforeRevision ? [...rows, ...value.revisions] : value.revisions;
        more = value.has_more;
        cursor = value.next_before_revision;
      }
    } catch (failure) {
      if (alive && request === generation) {
        error = factFailure(failure);
        if (factDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
    busy = false;
  });
</script>

<section class="case-history" aria-label={`Historial de ${singular}`} aria-busy={busy}>
  <h3>Historial del registro</h3>
  <p class="hint">Consulta una revisi&#243;n para leer sus valores y fuentes exactos.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={busy || disabled} onclick={() => load()}
      >Consultar historial de nuevo</button
    >{/if}
  {#each rows as row (row.revision)}<div class="fact-history-row">
      <p>
        Revisi&#243;n {row.revision} / {row.status === 'recorded' ? 'Registrado' : 'Retirado'} / {row
          .recorded_by.email}
      </p>
      <p><time datetime={row.recorded_at}>{row.recorded_at}</time></p>
      {#if row.reason}<p class="case-multiline">{row.reason}</p>{/if}
      <button
        class="secondary"
        disabled={busy || disabled}
        onclick={() => onselect(id, row.revision)}
        >Consultar {singular} revisi&#243;n {row.revision}</button
      >
    </div>{/each}
  {#if more}<button class="secondary" disabled={busy || disabled} onclick={() => load(cursor)}
      >Cargar cambios anteriores</button
    >{/if}
</section>
