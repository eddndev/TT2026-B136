<script>
  import DeadlineFieldsReview from './DeadlineFieldsReview.svelte';
  import ResourceValues from './ResourceValues.svelte';
  import ResourceSources from './ResourceSources.svelte';
  export let value, profile;
</script>

<section aria-label="Revision del plazo y vinculo">
  <h3>Revisar antes de confirmar</h3>
  <p>
    El plazo y su v&#237;nculo se guardar&#225;n juntos. Revisa el resultado del c&#243;mputo y las
    capturas exactas.
  </p>
  <DeadlineFieldsReview value={value.deadline} {profile} />
  <section class="case-comparison" aria-label="Capturas del nuevo vinculo">
    <h3>Capturas del nuevo v&#237;nculo</h3>
    <p>Recurso / Revisi&#243;n {value.association.resource.revision}</p>
    <ResourceValues values={value.association.resource.values} />
    <ResourceSources
      sources={value.association.resource.sources}
      act={value.association.resource.act}
    />
    {#if value.association.act}
      <h4>Acto elegido / Revisi&#243;n {value.association.act.act.revision}</h4>
      <p>Capturado en la revisi&#243;n {value.association.act.revision} del recurso.</p>
      <ResourceValues values={value.association.act.act.values} act />
      <ResourceSources sources={value.association.act.sources} act={value.association.act.act} />
    {/if}
    <p>
      Cabeza actual comprobada: revisi&#243;n {value.association.observed_resource_head.revision}
    </p>
    <p>Autor: {value.association.recorded_by.email}</p>
  </section>
</section>
