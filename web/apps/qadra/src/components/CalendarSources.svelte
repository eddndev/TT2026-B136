<script>
  export let sources;
</script>

<section class="calendar-sources" aria-label="Referencias del calendario">
  <h3>Referencias declaradas</h3>
  <p class="hint">
    La copia de estas fuentes no est&#225; archivada. La URL no acredita autenticidad ni
    aplicabilidad; solo se abre por tu acci&#243;n.
  </p>
  {#each sources as source (source.id)}<article class="calendar-source-row">
      <h4>{source.title}</h4>
      <p>{source.issuer} / {source.locator}</p>
      <p>
        Publicaci&#243;n declarada: {source.published_on || 'Sin fecha'} / Consulta declarada: {source.consulted_on}
      </p>
      <a
        class="calendar-source-link"
        href={source.official_url}
        target="_blank"
        rel="noopener noreferrer">Abrir referencia: {source.title}</a
      >
      <details>
        <summary>URL e identidad de referencia</summary>
        <p>{source.official_url}</p>
        <code>{source.id}</code>
      </details>
    </article>{/each}
  {#if !sources.length}<p>Esta revisi&#243;n no declara referencias.</p>{/if}
</section>
