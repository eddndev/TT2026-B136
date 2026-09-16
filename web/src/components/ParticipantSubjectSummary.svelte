<script>
  import ParticipantValueSummary from './ParticipantValueSummary.svelte';
  import ParticipantLocatorSummary from './ParticipantLocatorSummary.svelte';
  export let record;
</script>

<dl class="participant-values">
  <div>
    <dt>Identidad</dt>
    <dd>{record.values.kind === 'natural_person' ? 'Persona' : '\u00d3rgano institucional'}</dd>
  </div>
  <div>
    <dt>Nombre registrado</dt>
    <dd><ParticipantValueSummary value={record.values.name} /></dd>
  </div>
  <div>
    <dt>{record.values.kind === 'natural_person' ? 'CURP' : 'Identificador institucional'}</dt>
    <dd>
      <ParticipantValueSummary
        value={record.values.curp || record.values.institutional_identifier}
      />
    </dd>
  </div>
</dl>
<p class="hint participant-provenance">
  Identidad {record.id} / revisi&#243;n {record.revision}<br />SHA-256: {record.values_digest}
</p>
<ParticipantLocatorSummary value={record.values.identity_support} />
