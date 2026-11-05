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
    <h2>{incidentFailureLabels[incident.failure]}</h2>
    <span class="badge warning">Versi&#243;n {incident.document_version}</span>
  </div>
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
    <summary>Detalle t&#233;cnico</summary>
    <dl>
      <dt>Incidente</dt>
      <dd><code>{incident.id}</code></dd>
      <dt>Observaci&#243;n</dt>
      <dd><code>{incident.observation_id}</code></dd>
      <dt>Solicitado por</dt>
      <dd><code>{incident.requester_id}</code></dd>
      <dt>Huella esperada (SHA-256)</dt>
      <dd><code>{incident.expected_digest}</code></dd>
      <dt>Huella observada (SHA-256)</dt>
      <dd><code>{incident.observed_snapshot_digest}</code></dd>
    </dl>
  </details>
  <button class="secondary" {disabled} onclick={() => onopen(incident)}
    >Abrir esta versi&#243;n</button
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
