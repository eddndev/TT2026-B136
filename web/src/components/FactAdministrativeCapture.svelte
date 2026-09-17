<script>
  export let value,
    label = 'Administraci\u00f3n consultada';
</script>

<div class="fact-administration">
  <h4>{label}</h4>
  <p>{value.title} / {value.reference}</p>
  {#if value.kind === 'unrevised'}<p>Ficha sin revisi&#243;n administrativa registrada.</p>
  {:else}<p>Revisi&#243;n {value.revision} / {value.changed_by.email}</p>
    <time datetime={value.changed_at}>{value.changed_at}</time>{/if}
</div>
