<script>
  import StageDate from './StageDate.svelte';
  import StageSupport from './StageSupport.svelte';
  import { stageLabels } from '../lib/case-stages.mjs';
  export let action,
    draft,
    api,
    caseId,
    ondenied,
    disabled = false,
    pending = false;
  let firstBusy = false,
    secondBusy = false;
  $: pending = firstBusy || secondBusy;
</script>

<div class="stack stage-fields">
  {#if action === 'adoption'}
    <p class="notice">
      Registra la etapa que declaras conocer. No se reconstruir&#225;n transiciones anteriores.
    </p>
    <label
      >Etapa conocida<select
        aria-label="Etapa conocida"
        bind:value={draft.stage}
        disabled={disabled || pending}
      >
        {#each Object.entries(stageLabels) as [key, label]}<option value={key}>{label}</option
          >{/each}
      </select></label
    >
    <StageDate
      label="Fecha de la etapa conocida"
      bind:value={draft.known_at}
      disabled={disabled || pending}
    />
    <label
      >Motivo de adopci&#243;n<textarea
        rows="3"
        bind:value={draft.reason}
        disabled={disabled || pending}></textarea></label
    >
    <StageSupport
      {api}
      {caseId}
      {ondenied}
      label="Soporte de adopci&#243;n"
      bind:value={draft.support}
      bind:pending={firstBusy}
      {disabled}
    />
  {:else if action === 'intermediate'}
    <StageDate
      label="Fecha de la acusaci&#243;n"
      bind:value={draft.accusation_declared_at}
      disabled={disabled || pending}
    />
    <StageSupport
      {api}
      {caseId}
      {ondenied}
      label="Acusaci&#243;n"
      bind:value={draft.accusation}
      bind:pending={firstBusy}
      {disabled}
    />
  {:else if action === 'trial'}
    <h3>Emisi&#243;n del auto</h3>
    <StageDate
      label="Fecha de emisi&#243;n del auto"
      bind:value={draft.opening_order_issued_at}
      disabled={disabled || pending}
    />
    <StageSupport
      {api}
      {caseId}
      {ondenied}
      label="Auto de apertura"
      bind:value={draft.opening_order}
      bind:pending={firstBusy}
      disabled={disabled || secondBusy}
    />
    <h3>Recepci&#243;n por el tribunal</h3>
    <StageDate
      label="Fecha de recepci&#243;n"
      bind:value={draft.received_at}
      disabled={disabled || pending}
    />
    <label
      >Tribunal receptor<input
        bind:value={draft.receiving_court}
        disabled={disabled || pending}
      /></label
    >
    <label
      >Referencia de recepci&#243;n (opcional)<input
        bind:value={draft.receipt_reference}
        disabled={disabled || pending}
      /></label
    >
    <StageSupport
      {api}
      {caseId}
      {ondenied}
      label="Constancia de recepci&#243;n"
      bind:value={draft.receipt_support}
      optional
      bind:pending={secondBusy}
      disabled={disabled || firstBusy}
    />
    <p class="hint">
      Puedes elegir expl&#237;citamente la misma versi&#243;n para ambos soportes si documenta ambos
      actos.
    </p>
  {/if}
  {#if action !== 'adoption'}<label
      >Nota (opcional)<textarea rows="3" bind:value={draft.note} disabled={disabled || pending}
      ></textarea></label
    >{/if}
</div>
