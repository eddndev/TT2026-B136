<script>
  import ResourceHearingValues from './ResourceHearingValues.svelte';
  import ResourceHearingSources from './ResourceHearingSources.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  export let value;
</script>

<section class="case-comparison" aria-label="Revision de audiencia y vinculo">
  <h3>Revisar programaci&#243;n y v&#237;nculo inicial</h3>
  <ResourceHearingValues values={value.command.values} />
  <ResourceHearingSources
    sources={{ resource: value.resource, act: value.act, ...value.sources }}
  />
  <p>Cabeza esperada del recurso: revisi&#243;n {value.observed_resource_head.revision}.</p>
  <p>Autor: {value.recorded_by.email}</p>
  <FactAdministrativeCapture
    value={value.observed_administration}
    label="Administracion observada"
  />
  <details>
    <summary>Identidades y recibo de preparaci&#243;n</summary>
    <p>Audiencia: <code>{value.command.hearing_id}</code></p>
    <p>Asociaci&#243;n inicial: <code>{value.command.association_id}</code></p>
    <p>Operaci&#243;n: <code>{value.command.operation_id}</code></p>
    <p>Recibo: <code>{value.submission_digest}</code></p>
  </details>
  <p class="hint">
    La programaci&#243;n declarada y su asociaci&#243;n inicial se registran juntas. No determinan
    procedencia jur&#237;dica ni asistencia.
  </p>
</section>
