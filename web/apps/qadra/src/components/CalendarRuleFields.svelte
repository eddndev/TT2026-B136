<script>
  import { calendarStates } from '../lib/judicial-calendar-labels.mjs';
  export let rule,
    sources,
    disabled = false;
</script>

<label
  >Clasificaci&#243;n<select bind:value={rule.classification} {disabled}
    ><option value="">Selecciona una clasificaci&#243;n</option
    >{#each ['countable', 'excluded', 'unresolved'] as value}<option {value}
        >{calendarStates[value]}</option
      >{/each}</select
  ></label
>
<label
  >Explicaci&#243;n<textarea rows="2" bind:value={rule.explanation} {disabled}></textarea></label
>
<fieldset class="calendar-source-selection" {disabled}>
  <legend>Fuentes de esta regla</legend>
  {#each sources as source}<label class="calendar-check"
      ><input type="checkbox" value={source.id} bind:group={rule.source_ids} />{source.title ||
        'Fuente sin t\u00edtulo'}</label
    >
    <details><summary>Identidad de la fuente</summary><code>{source.id}</code></details>{/each}
  {#if !sources.length}<p class="hint">
      Agrega una fuente para declarar una fecha computable o excluida.
    </p>{/if}
</fieldset>
