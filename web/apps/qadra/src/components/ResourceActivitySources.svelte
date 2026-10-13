<script>
  import ResourceValues from './ResourceValues.svelte';
  import ResourceSources from './ResourceSources.svelte';
  import ResourceActivityTarget from './ResourceActivityTarget.svelte';
  export let sources;
</script>

<section class="case-comparison" aria-label="Captura vinculada">
  <h3>Captura vinculada</h3>
  <p>Recurso / Revisi&#243;n {sources.resource.revision}</p>
  <ResourceValues values={sources.resource.values} />
  <ResourceSources sources={sources.resource.sources} act={sources.resource.act} />
  {#if sources.act}
    <h4>Acto elegido / Revisi&#243;n {sources.act.act.revision}</h4>
    <p>Capturado en la revisi&#243;n {sources.act.revision} del recurso.</p>
    <ResourceValues values={sources.act.act.values} act />
    <ResourceSources sources={sources.act.sources} act={sources.act.act} />
  {/if}
  <ResourceActivityTarget target={sources.target} />
</section>
