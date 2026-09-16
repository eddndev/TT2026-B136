<script>
  export let sources,
    usedIds = [],
    disabled = false;
  function add() {
    sources = [
      ...sources,
      {
        id: crypto.randomUUID(),
        title: '',
        issuer: '',
        official_url: '',
        published_on: '',
        consulted_on: '',
        locator: '',
      },
    ];
  }
</script>

<section class="calendar-fields-section" aria-label="Fuentes declaradas">
  <div class="section-heading">
    <h3>Fuentes p&#250;blicas declaradas</h3>
    <button class="secondary" disabled={disabled || sources.length >= 16} onclick={add}
      >Agregar fuente</button
    >
  </div>
  <p class="hint">
    Hasta 16 referencias. Declaras sus datos; la copia no se archiva ni se verifica su contenido
    remoto.
  </p>
  {#each sources as source, index (source.id)}<fieldset class="calendar-fieldset" {disabled}>
      <legend>Fuente {index + 1}</legend>
      <div class="calendar-form-grid">
        <label>T&#237;tulo de fuente<input bind:value={source.title} /></label>
        <label>Emisor<input bind:value={source.issuer} /></label>
        <label
          >Referencia HTTPS<input
            type="text"
            inputmode="url"
            bind:value={source.official_url}
            placeholder="https://"
          /></label
        >
        <label>Localizador<input bind:value={source.locator} /></label>
        <label
          >Publicaci&#243;n declarada (opcional)<input
            type="date"
            min="0001-01-01"
            max="9999-12-31"
            bind:value={source.published_on}
          /></label
        >
        <label
          >Consulta declarada<input
            type="date"
            min="0001-01-01"
            max="9999-12-31"
            bind:value={source.consulted_on}
          /></label
        >
      </div>
      <details><summary>Identidad de esta referencia</summary><code>{source.id}</code></details>
      {#if usedIds.includes(source.id)}<p class="hint">
          Esta fuente est&#225; vinculada a reglas. Quita esos v&#237;nculos antes de retirarla del
          borrador.
        </p>{/if}
      <button
        class="text-button"
        disabled={usedIds.includes(source.id)}
        onclick={() => (sources = sources.filter((row) => row.id !== source.id))}
        >Quitar fuente {index + 1}</button
      >
    </fieldset>{/each}
  {#if !sources.length}<p>Sin referencias en este borrador.</p>{/if}
</section>
