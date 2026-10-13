<script>
  export let value,
    label,
    choices = null,
    disabled = false;
  $: selected =
    value?.kind === 'unknown'
      ? 'unknown'
      : value?.kind === 'known'
        ? choices
          ? value.value?.kind || ''
          : 'known'
        : '';
  function choose(kind) {
    if (!kind) value = { kind: '' };
    else if (kind === 'unknown') value = { kind, reason: '' };
    else
      value = {
        kind: 'known',
        value: choices ? (kind === 'other' ? { kind, label: '' } : { kind }) : '',
      };
  }
</script>

<div class="fact-declaration-fields">
  <label
    >{label}<select
      value={selected}
      {disabled}
      onchange={(event) => choose(event.currentTarget.value)}
    >
      <option value="">Selecciona lo declarado</option>
      <option value="unknown">No consta</option>
      {#if choices}{#each Object.entries(choices) as [key, text]}<option value={key}>{text}</option
          >{/each}
      {:else}<option value="known">Conocido</option>{/if}
    </select></label
  >
  {#if value?.kind === 'unknown'}
    <label>Motivo: {label}<textarea rows="2" {disabled} bind:value={value.reason}></textarea></label
    >
  {:else if value?.kind === 'known'}
    {#if !choices}<label>{label} declarado<input {disabled} bind:value={value.value} /></label>
    {:else if value.value?.kind === 'other'}<label
        >Descripci&#243;n: {label}<input {disabled} bind:value={value.value.label} /></label
      >{/if}
  {/if}
</div>
