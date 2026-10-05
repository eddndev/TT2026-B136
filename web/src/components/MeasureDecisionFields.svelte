<script>
  import { onDestroy } from 'svelte';
  import MeasureTimeFields from './MeasureTimeFields.svelte';
  import MeasureEffectFields from './MeasureEffectFields.svelte';
  import MeasureDecisionAnchor from './MeasureDecisionAnchor.svelte';
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import { newMeasureEffect } from './measure-decision-editor-values.mjs';
  export let api,
    caseId,
    fields,
    support,
    anchor,
    effects,
    ondenied,
    inputs = null,
    disabled = false,
    pending = false,
    canApply = () => true;
  const documents = api.caseDocuments(caseId);
  let choosingSupport = inputs?.choosingSupport ?? false,
    supportInputs = inputs?.supportPicker ?? null,
    supportPicker,
    time,
    anchorView,
    effectViews = {},
    effectBusy = {},
    supportBusy = false,
    anchorBusy = false;
  $: pending = supportBusy || anchorBusy || Object.values(effectBusy).some(Boolean);
  export function captureInputs() {
    return {
      choosingSupport,
      supportPicker: supportPicker?.captureDraft() ?? supportInputs,
      time: time?.captureInputs() ?? inputs?.time ?? null,
      anchor: anchorView?.captureInputs() ?? inputs?.anchor ?? null,
      effects: Object.fromEntries(
        effects.map((row) => [
          row.key,
          effectViews[row.key]?.captureInputs() ?? inputs?.effects?.[row.key] ?? null,
        ]),
      ),
    };
  }
  function closeSupport() {
    supportInputs = supportPicker?.captureDraft() ?? supportInputs;
    choosingSupport = false;
  }
  onDestroy(() => {
    documents.dispose();
    pending = false;
  });
</script>

<div class="stack">
  <label>Autoridad<input maxlength="1000" bind:value={fields.authority} {disabled} /></label>
  <MeasureTimeFields
    bind:this={time}
    bind:value={fields.declaredAt}
    label="decision"
    inputs={inputs?.time}
    {disabled}
  />
  <label
    >Justificacion<textarea rows="3" maxlength="1000" bind:value={fields.justification} {disabled}
    ></textarea></label
  >
  <label
    >Localizador<textarea rows="2" maxlength="1000" bind:value={fields.locator} {disabled}
    ></textarea></label
  >
  <StageSupportSummary record={support} />
  <button class="secondary" disabled={disabled || pending} onclick={() => (choosingSupport = true)}
    >Elegir soporte</button
  >
  {#if choosingSupport}<StagePicker
      api={documents}
      {caseId}
      {ondenied}
      {canApply}
      bind:this={supportPicker}
      draft={supportInputs}
      disabled={disabled || anchorBusy}
      bind:busy={supportBusy}
      oncancel={closeSupport}
      onselected={(row) => {
        if (canApply() && row.case_id === caseId) {
          support = structuredClone(row);
          closeSupport();
        }
      }}
    />{/if}
  <MeasureDecisionAnchor
    {api}
    {caseId}
    {ondenied}
    {canApply}
    bind:value={anchor}
    bind:this={anchorView}
    inputs={inputs?.anchor}
    disabled={disabled || supportBusy}
    bind:pending={anchorBusy}
  />
  <label
    >Resultado<select bind:value={fields.outcome} disabled={disabled || pending}>
      <option value="changes">Cambios en medidas</option><option value="no_measure_change"
        >Sin cambio en medidas</option
      >
    </select></label
  >
  {#if fields.outcome === 'no_measure_change'}
    <label
      >Declaracion sin cambios<textarea
        rows="3"
        maxlength="1000"
        bind:value={fields.statement}
        {disabled}></textarea></label
    >
  {:else}
    {#each effects as row, index (row.key)}
      <MeasureEffectFields
        {api}
        {caseId}
        {ondenied}
        {canApply}
        bind:row={effects[index]}
        index={index + 1}
        inputs={inputs?.effects?.[row.key]}
        bind:this={effectViews[row.key]}
        bind:pending={effectBusy[row.key]}
        disabled={disabled || supportBusy || anchorBusy}
      />
      <button
        class="text-button"
        disabled={disabled || pending}
        onclick={() => {
          effects = effects.filter((item) => item.key !== row.key);
          delete effectBusy[row.key];
        }}>Quitar efecto {index + 1}</button
      >
    {/each}
    <button
      class="secondary"
      disabled={disabled || pending || effects.length >= 32}
      onclick={() => (effects = [...effects, newMeasureEffect()])}>Agregar efecto</button
    >
    <p class="hint">Hasta 32 identidades entre anteriores y nuevas, confirmadas juntas.</p>
  {/if}
</div>
