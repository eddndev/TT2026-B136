<script>
  import CalendarRuleFields from './CalendarRuleFields.svelte';
  export let exceptions,
    sources,
    disabled = false;
  function add() {
    exceptions = [
      ...exceptions,
      {
        id: crypto.randomUUID(),
        from: '',
        through: '',
        classification: '',
        source_ids: [],
        explanation: '',
      },
    ];
  }
</script>

<section class="calendar-fields-section" aria-label="Excepciones declaradas">
  <div class="section-heading">
    <h3>Excepciones por intervalo</h3>
    <button class="secondary" disabled={disabled || exceptions.length >= 64} onclick={add}
      >Agregar excepci&#243;n</button
    >
  </div>
  <p class="hint">
    Hasta 64 intervalos inclusivos, sin solapamientos y dentro de la cobertura. Cada excepci&#243;n
    sustituye la regla semanal completa.
  </p>
  {#each exceptions as rule, index (rule.id)}<fieldset class="calendar-fieldset" {disabled}>
      <legend>Excepci&#243;n {index + 1}</legend>
      <div class="calendar-form-grid">
        <label
          >Excepci&#243;n desde<input
            type="date"
            min="0001-01-01"
            max="9999-12-31"
            bind:value={rule.from}
          /></label
        ><label
          >Excepci&#243;n hasta<input
            type="date"
            min="0001-01-01"
            max="9999-12-31"
            bind:value={rule.through}
          /></label
        >
      </div>
      <CalendarRuleFields bind:rule {sources} {disabled} />
      <details><summary>Identidad de la excepci&#243;n</summary><code>{rule.id}</code></details>
      <button
        class="text-button"
        onclick={() => (exceptions = exceptions.filter((row) => row.id !== rule.id))}
        >Quitar excepci&#243;n {index + 1}</button
      >
    </fieldset>{/each}
  {#if !exceptions.length}<p>Sin excepciones declaradas.</p>{/if}
</section>
