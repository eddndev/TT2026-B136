<script>
  // Rows of contracts or signature requests, each with a plain state label.
  export let title,
    description = '',
    rows = [],
    selected = null,
    busy = false,
    loaded = false,
    empty = 'Todav\u00eda no hay contratos.',
    onopen,
    onrefresh = null;
</script>

<section class="card contract-list" aria-label={title} aria-busy={busy}>
  <div class="section-heading">
    <div>
      <h2>{title}</h2>
      {#if description}<p class="hint">{description}</p>{/if}
    </div>
    {#if onrefresh}<button class="secondary" disabled={busy} onclick={onrefresh}>Actualizar</button
      >{/if}
  </div>
  {#if busy}<p role="status">Consultando contratos...</p>{/if}
  {#each rows as row (row.id)}
    <article class="contract-row" class:selected={selected === row.id}>
      <div>
        <span class="badge {row.tone}">{row.label}</span>
        <h3>{row.title}</h3>
        {#if row.detail}<p class="hint">{row.detail}</p>{/if}
      </div>
      <button
        class="secondary"
        disabled={busy}
        aria-label={`Abrir ${row.title}`}
        aria-pressed={selected === row.id}
        onclick={() => onopen(row.id)}>Abrir</button
      >
    </article>{/each}
  {#if loaded && !busy && !rows.length}<p class="hint">{empty}</p>{/if}
</section>

<style>
  .contract-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    border-top: 1px solid var(--line);
    padding: 12px 0;
    min-width: 0;
  }
  .contract-row > div {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .contract-row.selected h3 {
    text-decoration: underline;
  }
  .contract-row h3 {
    margin: 6px 0 2px;
  }
</style>
