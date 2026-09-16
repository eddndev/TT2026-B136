<script>
  export let field,
    value,
    disabled = false;
</script>

{#if field.type === 'license'}
  <label>{field.label}<input bind:value={value.number} {disabled} /></label>
  <label>Emisor de {field.label}<input bind:value={value.issuer} {disabled} /></label>
{:else if field.type === 'choice'}
  <label
    >{field.label}<select aria-label={field.label} bind:value {disabled}
      ><option value="">Selecciona una opci&#243;n</option
      >{#each Object.entries(field.options) as [key, label]}<option value={key}>{label}</option
        >{/each}</select
    ></label
  >
{:else}<label>{field.label}<input aria-label={field.label} bind:value {disabled} /></label>{/if}
