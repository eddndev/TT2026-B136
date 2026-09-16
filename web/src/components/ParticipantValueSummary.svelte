<script>
  import ParticipantLocatorSummary from './ParticipantLocatorSummary.svelte';
  export let value,
    field = {};
  $: inner = value?.state === 'known' ? value.value : value;
</script>

{#if ['unknown', 'not_recorded', 'none_declared'].includes(value?.state)}
  <span
    >{value.state === 'unknown'
      ? 'Dato desconocido'
      : value.state === 'not_recorded'
        ? 'No registrado'
        : 'Ninguna declarada'}: {value.reason}</span
  >
{:else if value?.state === 'documented'}<span>Documentado</span><ParticipantLocatorSummary
    value={value.support}
  />
{:else if value?.state === 'unidentified'}<span
    >{value.label} / Sin identificar: {value.reason}</span
  >
{:else if field.type === 'license'}<span>{inner.number} / {inner.issuer}</span>
{:else}<span>{field.options?.[inner] || inner}</span>{/if}
