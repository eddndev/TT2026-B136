<script>
  import { onMount, onDestroy } from 'svelte';
  import MeasureRecordPicker from './MeasureRecordPicker.svelte';
  import MeasureRecordSummary from './MeasureRecordSummary.svelte';
  import { factDenied, factFailure } from '../lib/procedural-fact-errors.mjs';
  export let api,
    caseId,
    references,
    ondenied,
    disabled = false,
    pending = false,
    canApply = () => true,
    inputs = null;
  const scoped = api.caseMeasures(caseId);
  let alive = true,
    choosing = inputs?.choosingReviewTarget ?? false,
    pickerBusy = false,
    loading = false,
    selected = new Map(),
    error = '';
  $: pending = pickerBusy || loading;
  export function captureInputs() {
    return { choosingReviewTarget: choosing };
  }
  async function refresh() {
    if (!canApply() || loading) return;
    loading = true;
    error = '';
    try {
      const next = new Map();
      for (const reference of references) {
        const exact = await scoped.exact(reference);
        if (!alive || !canApply()) return;
        next.set(reference.id, exact);
      }
      if (alive && canApply()) selected = next;
    } catch (failure) {
      if (alive && canApply()) {
        error = factFailure(failure);
        if (factDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) loading = false;
    }
  }
  function choose(value) {
    if (
      !canApply() ||
      value.validity !== 'valid' ||
      references.length >= 32 ||
      references.some((row) => row.id === value.reference.id)
    )
      return;
    references = [...references, structuredClone(value.reference)].sort((a, b) =>
      a.id.localeCompare(b.id),
    );
    selected = new Map(selected).set(value.reference.id, value);
    choosing = false;
  }
  function remove(id) {
    if (disabled || pending || !canApply()) return;
    references = references.filter((row) => row.id !== id);
    selected.delete(id);
    selected = new Map(selected);
  }
  onMount(refresh);
  onDestroy(() => {
    alive = false;
    scoped.dispose();
    pending = false;
  });
</script>

<section
  class="case-comparison"
  aria-label="Medidas seleccionadas para revision"
  aria-busy={pending}
>
  <h3>Medidas seleccionadas para revisi&#243;n ({references.length}/32)</h3>
  <p>Las referencias conservan su revisi&#243;n aunque existan declaraciones posteriores.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={disabled || pending} onclick={refresh}
      >Consultar medidas seleccionadas</button
    >{/if}
  {#each references as reference (reference.id)}
    {#if selected.has(reference.id)}<MeasureRecordSummary value={selected.get(reference.id)} />
    {:else}<p>Medida: <code>{reference.id}</code> / Revisi&#243;n {reference.revision}</p>
      <p>Captura: <code>{reference.capture_digest}</code></p>{/if}
    <button
      class="text-button"
      disabled={disabled || pending}
      onclick={() => remove(reference.id)}
      aria-label={`Quitar medida ${reference.id}`}>Quitar medida</button
    >
  {/each}
  {#if !references.length}<p>Selecciona al menos una medida exacta para la revisi&#243;n.</p>{/if}
  <button
    class="secondary"
    disabled={disabled || pending || references.length >= 32}
    onclick={() => (choosing = true)}>Elegir medida</button
  >
  {#if choosing}<MeasureRecordPicker
      api={scoped}
      {ondenied}
      {canApply}
      selectedIds={references.map((row) => row.id)}
      disabled={disabled || loading}
      bind:busy={pickerBusy}
      onselected={choose}
      oncancel={() => (choosing = false)}
    />{/if}
</section>
