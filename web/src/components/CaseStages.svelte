<script>
  import { onMount, onDestroy } from 'svelte';
  import StageEntry from './StageEntry.svelte';
  import StageHistory from './StageHistory.svelte';
  import StageForm from './StageForm.svelte';
  import { stageAction, stageLabels } from '../lib/case-stages.mjs';
  import { manageCase } from '../lib/case-administration.mjs';
  import { caseState } from '../lib/case-state.mjs';
  export let api,
    user,
    record,
    ondenied,
    onnavigate,
    loading = false;
  const state = caseState(),
    scoped = api.caseStages(record.id),
    documents = api.caseDocuments(record.id);
  let alive = true,
    mounted = false,
    started = false,
    ready = false,
    busy = false,
    error = '';
  let current = null,
    editing = false,
    history = false,
    historyView,
    historyBusy = false,
    formBusy = false;
  $: pending = busy || historyBusy || formBusy || loading;
  $: canManage = manageCase(user.role) && record.administration.profile && !$state.closed;
  $: action = stageAction(current);
  $: if (mounted && !started && !loading) {
    started = true;
    load();
  }
  function observe(result) {
    if (
      !alive ||
      result.case_id !== record.id ||
      (current?.stage_revision || 0) > (result.current?.stage_revision || 0)
    )
      return;
    current = result.current;
  }
  async function load() {
    if (!alive || pending) return;
    busy = true;
    error = '';
    try {
      const result = await scoped.get();
      if (alive) {
        observe(result);
        ready = true;
      }
    } catch (failure) {
      if (!alive) return;
      error = failure.message;
      if ([403, 404].includes(failure.status)) ondenied(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function confirmed(result) {
    if (!alive) return;
    observe(result);
    if (historyView) await historyView.refresh();
    if (alive) editing = false;
  }
  onMount(() => {
    mounted = true;
  });
  onDestroy(() => {
    alive = false;
    scoped.dispose();
    documents.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">SEGUIMIENTO PROCESAL</span>
    <h1>Etapas del expediente</h1>
    <p>Etapa registrada, actos declarados y soportes exactos.</p>
  </div>
</div>
<section class="case-stages" aria-busy={pending}>
  <div class="card case-stage stage-current">
    <div class="section-heading">
      <h2>Etapa actual registrada</h2>
      <button class="secondary" disabled={pending || editing} onclick={load}
        >Actualizar etapa</button
      >
    </div>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if busy}<p class="hint" role="status">Consultando etapa...</p>{/if}
    {#if ready}
      {#if current}<StageEntry record={current} />{:else}<h3>Sin etapa registrada</h3>
        <p>Completar la ficha penal no reconstruye etapas anteriores.</p>{/if}
      {#if !record.administration.profile}<p class="notice">
          Completa la ficha penal para registrar una etapa.
        </p>
        <button class="secondary" onclick={() => onnavigate('case-summary')}>Ir al resumen</button>
      {:else if canManage && action && !editing}<button
          class="primary"
          disabled={pending}
          onclick={() => (editing = true)}
          >{action === 'adoption'
            ? 'Registrar etapa actual'
            : `Registrar paso a ${stageLabels[action]}`}</button
        >
      {:else if current?.stage === 'trial'}<p class="hint">
          No hay otro avance ordinario disponible.
        </p>{/if}
      <button
        class="text-button"
        disabled={pending}
        aria-expanded={history}
        onclick={() => (history = !history)}
        >{history ? 'Ocultar historial de etapas' : 'Ver historial de etapas'}</button
      >
    {/if}
  </div>
  {#if editing}<StageForm
      api={scoped}
      {documents}
      caseId={record.id}
      {current}
      {ondenied}
      onobserved={observe}
      onconfirmed={confirmed}
      oncancel={() => (editing = false)}
      disabled={!canManage || busy || historyBusy || loading}
      bind:pending={formBusy}
    />{/if}
  {#if history}<section class="card case-stage">
      <StageHistory
        api={scoped}
        {ondenied}
        disabled={busy || formBusy || loading}
        bind:busy={historyBusy}
        bind:this={historyView}
      />
    </section>{/if}
</section>
