<script>
  import FactTimeFields from './FactTimeFields.svelte';
  export let value,
    disabled = false,
    recoverable = false,
    savedInputs = null;
  let timeFields;
  export function captureInputs() {
    return { time: timeFields?.captureDraft() ?? savedInputs?.time ?? null };
  }
  function choose(status) {
    savedInputs = null;
    value =
      status === 'recorded'
        ? { status, occurred_at: { precision: '' }, statement: '', locator: '' }
        : { status };
  }
</script>

<fieldset class="case-offenses" {disabled}>
  <legend>Declaraci&#243;n de atenci&#243;n</legend>
  <label
    >Estado de atenci&#243;n<select
      value={value.status}
      onchange={(event) => choose(event.currentTarget.value)}
    >
      <option value="">Selecciona el estado</option><option value="pending">Pendiente</option>
      <option value="recorded">Atenci&#243;n declarada</option>
    </select></label
  >
  {#if value.status === 'recorded'}
    <FactTimeFields
      bind:this={timeFields}
      bind:value={value.occurred_at}
      label="atenci&#243;n"
      {disabled}
      {recoverable}
      draft={savedInputs?.time}
    />
    <label
      >Declaraci&#243;n de atenci&#243;n<textarea
        rows="3"
        maxlength="1000"
        bind:value={value.statement}></textarea></label
    >
    <label>Localizador de atenci&#243;n<input maxlength="200" bind:value={value.locator} /></label>
  {/if}
  <p class="hint">
    Declarar atenci&#243;n no acredita presentaci&#243;n v&#225;lida ni cumplimiento oportuno. El
    c&#225;lculo hist&#243;rico se conserva.
  </p>
</fieldset>
