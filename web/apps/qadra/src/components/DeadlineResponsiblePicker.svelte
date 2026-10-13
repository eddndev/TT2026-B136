<script>
  import { onMount, onDestroy } from 'svelte';
  import { deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  export let api,
    caseId,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  const scoped = api.deadlines(caseId);
  let alive = true,
    rows = [],
    more = false,
    cursor,
    error = '';
  async function load(afterId) {
    if (busy || disabled) return;
    busy = true;
    error = '';
    try {
      const page = await scoped.responsibles({ limit: 20, afterId });
      if (!alive) return;
      rows = page.responsibles;
      more = page.has_more;
      cursor = page.next_after_id;
    } catch (failure) {
      if (alive) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    busy = false;
    scoped.dispose();
  });
</script>

<section class="case-comparison" aria-label="Elegir responsable" aria-busy={busy}>
  <h4>Personas con acceso al expediente</h4>
  <p class="hint">
    La selecci&#243;n no concede acceso. Al confirmar se comprueba que la cuenta siga activa y
    autorizada.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each rows as row}<div class="hearing-result-picker-row">
      <p>
        {row.email} / {row.role === 'owner'
          ? 'Owner'
          : row.role === 'litigator'
            ? 'Litigator'
            : 'Paralegal'}
      </p>
      <button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => onselected(row)}>Elegir responsable {row.email}</button
      >
    </div>{/each}
  {#if !rows.length && !busy}<p>No hay responsables elegibles en esta p&#225;gina.</p>{/if}
  <div class="action-row">
    <button type="button" class="secondary" disabled={disabled || busy} onclick={() => load()}
      >Primera p&#225;gina de responsables</button
    >
    {#if more}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => load(cursor)}>Siguientes responsables</button
      >{/if}
  </div>
  <button type="button" class="text-button" disabled={busy} onclick={oncancel}
    >Cerrar selector de responsable</button
  >
</section>
