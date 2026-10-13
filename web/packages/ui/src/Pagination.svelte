<script>
  export let offset = 0;
  export let limit = 50;
  export let count = 0;
  export let hasMore = false;
  export let busy = false;
  export let onchange;
</script>

<div class="pagination" aria-label="Paginaci&#243;n">
  <span class="hint"
    >{count ? `${offset + 1} a ${offset + count}` : 'Sin resultados'} en esta consulta</span
  >
  <div class="action-row">
    <button
      class="secondary"
      disabled={busy || offset === 0}
      onclick={() => onchange(Math.max(0, offset - limit))}>Anterior</button
    >
    <button class="secondary" disabled={busy || !hasMore} onclick={() => onchange(offset + limit)}
      >Siguiente</button
    >
  </div>
</div>
