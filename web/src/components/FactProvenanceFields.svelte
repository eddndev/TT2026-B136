<script>
  import FactSupportFields from './FactSupportFields.svelte';
  import FactResultPicker from './FactResultPicker.svelte';
  export let value,
    label,
    api,
    caseId,
    ondenied,
    disabled = false,
    pending = false;
  let picking = false,
    resultBusy = false,
    supportBusy = false,
    selected = null;
  $: pending = resultBusy || supportBusy;
  function kind(next) {
    picking = false;
    selected = null;
    value =
      next === 'operator_note'
        ? { kind: next, note: '' }
        : next === 'external_reference'
          ? { kind: next, reference: '', support: null }
          : next === 'hearing_result'
            ? { kind: next, reference: null, locator: '', support: null }
            : { kind: '' };
  }
  function select(selection) {
    value = { ...value, reference: selection.reference };
    selected = selection.record;
    picking = false;
  }
</script>

<fieldset class="case-offenses fact-provenance-fields" {disabled}>
  <legend>{label}</legend>
  <label
    >{label}<select
      value={value?.kind || ''}
      onchange={(event) => kind(event.currentTarget.value)}
      disabled={pending}
    >
      <option value="">Selecciona la procedencia</option><option value="operator_note"
        >Nota del operador</option
      >
      <option value="external_reference">Referencia externa</option><option value="hearing_result"
        >Resultado de audiencia</option
      >
    </select></label
  >
  {#if value?.kind === 'operator_note'}<label
      >Nota: {label}<textarea rows="3" bind:value={value.note} disabled={pending}></textarea></label
    >
  {:else if value?.kind === 'external_reference'}<label
      >Referencia externa: {label}<textarea rows="3" bind:value={value.reference} disabled={pending}
      ></textarea></label
    >
  {:else if value?.kind === 'hearing_result'}
    {#if value.reference}<p>
        Resultado en la revisi&#243;n {value.reference.revision}{value.reference.agreement_id
          ? ', con acuerdo seleccionado'
          : ', sin acuerdo espec\u00edfico'}.
      </p>
      {#if selected}<p class="case-multiline">{selected.values.summary}</p>{/if}
      <details>
        <summary>Referencia exacta del resultado</summary>
        <p>Audiencia: <code>{value.reference.hearing_id}</code></p>
        <p>Resultado: <code>{value.reference.result_id}</code></p>
        {#if value.reference.agreement_id}<p>
            Acuerdo: <code>{value.reference.agreement_id}</code>
          </p>{/if}
      </details>
    {/if}
    <button type="button" class="secondary" disabled={pending} onclick={() => (picking = true)}
      >Elegir resultado: {label}</button
    >
    {#if picking}<FactResultPicker
        {api}
        {caseId}
        {ondenied}
        disabled={disabled || supportBusy}
        bind:busy={resultBusy}
        onselected={select}
        oncancel={() => (picking = false)}
      />{/if}
    <label
      >Localizador en el resultado: {label}<input
        bind:value={value.locator}
        disabled={pending}
      /></label
    >
  {/if}
  {#if ['external_reference', 'hearing_result'].includes(value?.kind)}<FactSupportFields
      bind:value={value.support}
      {api}
      {caseId}
      {label}
      {ondenied}
      disabled={disabled || resultBusy}
      bind:pending={supportBusy}
    />{/if}
</fieldset>
