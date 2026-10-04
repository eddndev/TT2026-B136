<script>
  import ResourceValues from './ResourceValues.svelte';
  import ResourceSources from './ResourceSources.svelte';
  import FactSources from './FactSources.svelte';
  import { resourceKinds, resourceActKinds } from '../lib/procedural-resource-values.mjs';
  export let sources;
</script>

<section class="case-comparison" aria-label="Recurso y acto capturados">
  <h3>Recurso seleccionado / Revisi&#243;n {sources.resource.revision}</h3>
  <p><strong>{sources.resource.values.title}</strong></p>
  <p>
    {resourceKinds[sources.resource.values.kind]} / {sources.resource.status === 'archived'
      ? 'Archivado en esta captura'
      : 'Activo en esta captura'}
  </p>
  <details>
    <summary>Datos de la revisi&#243;n del recurso</summary>
    <p>Recurso: <code>{sources.resource.id}</code></p>
    <p>Captura: <code>{sources.resource.receipt.capture_digest}</code></p>
    <ResourceValues values={sources.resource.values} />
    <ResourceSources sources={sources.resource.sources} act={sources.resource.act} />
  </details>
  {#if sources.act}
    <h3>Acto seleccionado / Revisi&#243;n {sources.act.act.revision}</h3>
    <p>
      {resourceActKinds[sources.act.act.values.kind]} / Revisi&#243;n {sources.act.revision}
      del recurso
    </p>
    <p class="case-multiline">{sources.act.act.values.statement}</p>
    <details>
      <summary>Datos de la revisi&#243;n del acto</summary>
      <p>Acto: <code>{sources.act.act.id}</code></p>
      <p>Captura: <code>{sources.act.receipt.capture_digest}</code></p>
      <ResourceValues values={sources.act.act.values} act />
      <ResourceSources sources={sources.act.sources} act={sources.act.act} />
    </details>
  {:else}<p>Sin acto espec&#237;fico seleccionado.</p>{/if}
</section>
<FactSources
  sources={{
    resolution: null,
    participants: sources.participants,
    hearing_results: [],
    direct_supports: [sources.support],
  }}
/>
<p class="hint">Las fichas capturadas no acreditan asistencia a la audiencia.</p>
