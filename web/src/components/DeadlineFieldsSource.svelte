<script>
  import DeadlineSourcePicker from './DeadlineSourcePicker.svelte';
  import FactTimeFields from './FactTimeFields.svelte';
  import DeadlinePolicyChoice from './DeadlinePolicyChoice.svelte';
  export let value,
    api,
    caseId,
    ondenied,
    policy = '',
    policyPresent = false,
    disabled = false,
    pending = false;
  let choosing = false,
    sourceName = '';
  $: family = value.source?.kind === 'known' ? value.source.value.family : value.source?.kind || '';
  function selectFamily(next) {
    choosing = false;
    sourceName = '';
    value = {
      ...value,
      source:
        next === 'unknown'
          ? { kind: 'unknown', reason: '' }
          : next
            ? { kind: 'known', value: { family: next } }
            : { kind: '' },
      qualification: null,
    };
  }
</script>

<fieldset class="case-offenses" disabled={disabled || pending}>
  <legend>Fuente exacta del inicio</legend>
  <label
    >Tipo de fuente<select
      value={family}
      onchange={(event) => selectFamily(event.currentTarget.value)}
    >
      <option value="">Selecciona la fuente</option><option value="unknown"
        >Fuente no identificada</option
      >
      <option value="resolution">Resoluci&#243;n</option><option value="notification"
        >Notificaci&#243;n</option
      >
      <option value="hearing_result">Resultado de audiencia</option>
    </select></label
  >
  {#if family === 'unknown'}<label
      >Motivo de fuente no identificada<textarea
        rows="2"
        maxlength="1000"
        bind:value={value.source.reason}></textarea></label
    >
  {:else if family}
    {#if value.source.value.revision}<p>
        {sourceName || 'Fuente seleccionada'} / Revisi&#243;n {value.source.value.revision}
      </p>
      {#if family === 'notification'}<p>
          Resoluci&#243;n vinculada: revisi&#243;n {value.source.value.resolution.revision}
        </p>{/if}
      {#if family === 'hearing_result'}<p>
          {value.source.value.agreement_id === null
            ? 'Resultado completo, sin acuerdo especifico'
            : 'Acuerdo exacto seleccionado'}
        </p>{/if}
      <details>
        <summary>Identidad de la fuente</summary><code
          >{value.source.value.id || value.source.value.result_id}</code
        >
        {#if value.source.value.agreement_id !== null && family === 'hearing_result'}<p>
            Acuerdo: <code>{value.source.value.agreement_id}</code>
          </p>{/if}
      </details>
    {/if}
    <button type="button" class="secondary" onclick={() => (choosing = true)}
      >Elegir fuente exacta</button
    >
  {/if}
  <DeadlinePolicyChoice
    dependency="source"
    bind:value={policy}
    present={policyPresent}
    disabled={disabled || pending}
  />
</fieldset>
{#if choosing}{#key family}<DeadlineSourcePicker
      {api}
      {caseId}
      {family}
      {ondenied}
      {disabled}
      bind:busy={pending}
      oncancel={() => (choosing = false)}
      onselected={({ reference, record }) => {
        value = { ...value, source: { kind: 'known', value: reference }, qualification: null };
        sourceName = record.values.summary;
        choosing = false;
      }}
    />{/key}{/if}
<fieldset class="case-offenses" disabled={disabled || pending}>
  <legend>Inicio expresamente calificado en la misma fuente</legend>
  <label class="checkbox"
    ><input
      type="checkbox"
      checked={value.qualification !== null}
      onchange={(event) =>
        (value = {
          ...value,
          qualification: event.currentTarget.checked
            ? { purpose: '', at: { precision: '' }, statement: '', locator: '' }
            : null,
        })}
    />Declarar un inicio calificado</label
  >
  {#if value.qualification}<label
      >Finalidad del inicio<select bind:value={value.qualification.purpose}>
        <option value="">Selecciona la finalidad</option><option value="hearing_end"
          >Fin de audiencia</option
        >
        <option value="ordered_period_start">Inicio del periodo ordenado</option>
      </select></label
    >
    <FactTimeFields
      bind:value={value.qualification.at}
      label="inicio calificado"
      disabled={disabled || pending}
    />
    <label
      >Declaraci&#243;n del inicio<textarea
        maxlength="1000"
        rows="2"
        bind:value={value.qualification.statement}></textarea></label
    >
    <label
      >Localizador del inicio<input
        maxlength="200"
        bind:value={value.qualification.locator}
      /></label
    >
  {/if}
  <p class="hint">
    Se conserva lo declarado por la persona operadora. La narrativa no se convierte
    autom&#225;ticamente en una fecha.
  </p>
</fieldset>
