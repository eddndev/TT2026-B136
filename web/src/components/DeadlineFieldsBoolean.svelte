<script>
  export let value,
    label,
    disabled = false;
  $: choice =
    value?.kind === 'unknown'
      ? 'unknown'
      : value?.kind === 'known'
        ? value.value
          ? 'yes'
          : 'no'
        : '';
  function choose(next) {
    value =
      next === 'unknown'
        ? { kind: 'unknown', reason: '' }
        : next
          ? { kind: 'known', value: next === 'yes' }
          : { kind: '' };
  }
</script>

<div class="fact-declaration-fields">
  <label
    >{label}<select
      value={choice}
      {disabled}
      onchange={(event) => choose(event.currentTarget.value)}
    >
      <option value="">Selecciona lo declarado</option>
      <option value="yes">S&#237;</option><option value="no">No</option>
      <option value="unknown">No consta</option>
    </select></label
  >
  {#if value?.kind === 'unknown'}<label
      >Motivo: {label}<textarea rows="2" maxlength="1000" {disabled} bind:value={value.reason}
      ></textarea></label
    >{/if}
</div>
