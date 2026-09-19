<script>
  export let value,
    label,
    disabled = false;
  $: selected = value?.kind === 'unknown' ? 'unknown' : value?.kind === 'known' ? value.value : '';
  function choose(next) {
    value =
      next === 'unknown'
        ? { kind: 'unknown', reason: '' }
        : next
          ? { kind: 'known', value: next }
          : { kind: '' };
  }
</script>

<div class="fact-declaration-fields">
  <label
    >{label}<select
      value={selected}
      {disabled}
      onchange={(event) => choose(event.currentTarget.value)}
    >
      <option value="">Selecciona la modalidad declarada</option>
      <option value="unknown">No consta</option><option value="oral">Oral</option><option
        value="written">Escrita</option
      >
    </select></label
  >
  {#if value?.kind === 'unknown'}
    <label>Motivo: {label}<textarea rows="2" {disabled} bind:value={value.reason}></textarea></label
    >
  {/if}
</div>
