<script>
  import FactSources from './FactSources.svelte';
  export let sources,
    act = null;
  $: projection = {
    resolution: sources.resolution,
    participants: sources.appellants,
    hearing_results: [],
    direct_supports: sources.supports,
  };
</script>

<FactSources sources={projection} />
{#if act}
  <h3>Soportes del acto</h3>
  <FactSources
    sources={{
      resolution: null,
      participants: [],
      hearing_results: [],
      direct_supports: act.supports,
    }}
  />
{/if}
