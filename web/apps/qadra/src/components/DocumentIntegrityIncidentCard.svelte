<script>
  import {
    incidentFailureLabels,
    incidentTimeLabel,
  } from '../lib/document-integrity-incident-values.mjs';
  export let incident,
    onopen,
    disabled = false;
</script>

<article class="card incident-card" data-incident-id={incident.id}>
  <div class="section-heading">
    <h2>Validaci&#243;n documental fallida</h2>
    <span class="badge warning">Versi&#243;n {incident.document_version}</span>
  </div>
  <p>{incidentFailureLabels[incident.failure]}</p>
  <dl>
    <dt>Documento</dt>
    <dd><code>{incident.document_id}</code></dd>
    <dt>Expediente</dt>
    <dd><code>{incident.case_id}</code></dd>
    <dt>Detectado</dt>
    <dd>{incidentTimeLabel(incident.detected_at)}</dd>
    <dt>Registrado</dt>
    <dd>{incidentTimeLabel(incident.recorded_at)}</dd>
  </dl>
  <details>
    <summary>Referencia t&#233;cnica del incidente</summary>
    <dl>
      <dt>Incidente</dt>
      <dd><code>{incident.id}</code></dd>
      <dt>Observaci&#243;n</dt>
      <dd><code>{incident.observation_id}</code></dd>
      <dt>Cuenta que solicit&#243; la operaci&#243;n</dt>
      <dd><code>{incident.requester_id}</code></dd>
      <dt>Resumen esperado del contenido</dt>
      <dd><code>{incident.expected_digest}</code></dd>
      <dt>Resumen de la captura observada</dt>
      <dd><code>{incident.observed_snapshot_digest}</code></dd>
    </dl>
  </details>
  <button class="secondary" {disabled} onclick={() => onopen(incident)}
    >Abrir versi&#243;n exacta</button
  >
</article>

<style>
  .incident-card {
    min-width: 0;
    margin-block: 16px;
  }
  .incident-card h2 {
    font-size: 16px;
  }
  .section-heading {
    flex-wrap: wrap;
  }
  dl {
    display: grid;
    grid-template-columns: minmax(0, 150px) minmax(0, 1fr);
    gap: 8px 16px;
    margin-block: 16px;
  }
  dt {
    font-size: 12px;
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  code {
    overflow-wrap: anywhere;
    white-space: normal;
  }
  details {
    margin-block: 16px;
  }
  @media (max-width: 600px) {
    dl {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
