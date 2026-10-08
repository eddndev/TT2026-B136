<script>
  import FactTimeFields from './FactTimeFields.svelte';
  export let value,
    label,
    disabled = false,
    inputs = null;
  let fields,
    unknownReason = value?.reason ?? inputs?.reason ?? '';
  $: if (value?.precision === 'unknown') {
    if (typeof value.reason === 'string') unknownReason = value.reason;
    else value = { ...value, reason: unknownReason };
  }
  export function captureInputs() {
    return { time: fields?.captureDraft() ?? inputs?.time ?? null, reason: unknownReason };
  }
</script>

<FactTimeFields
  bind:this={fields}
  bind:value
  {label}
  {disabled}
  recoverable={true}
  draft={inputs?.time ?? null}
/>
{#if value?.precision === 'unknown'}
  <label
    >Motivo de tiempo desconocido de {label}<textarea
      rows="2"
      maxlength="1000"
      bind:value={value.reason}
      {disabled}></textarea></label
  >
{/if}
