<script>
  export let value,
    label,
    declared = true;
  $: person = declared ? value?.value : value;
</script>

<div class="fact-person-value">
  <h4>{label}</h4>
  {#if declared && value?.kind === 'unknown'}<p class="case-multiline">No consta: {value.reason}</p>
  {:else if person?.kind === 'unlinked'}<p><strong>{person.label}</strong></p>
    <p class="case-multiline">{person.description}</p>
    <p class="hint">Sin ficha vinculada.</p>
  {:else if person?.kind === 'participant'}<p>
      Ficha del expediente / Revisi&#243;n {person.revision}
    </p>
    <details><summary>Referencia exacta de {label}</summary><code>{person.id}</code></details>
  {/if}
</div>
