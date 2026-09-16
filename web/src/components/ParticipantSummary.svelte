<script>
  import ParticipantTypedSummary from './ParticipantTypedSummary.svelte';
  import { profileLabel } from '../lib/typed-participant-fields.mjs';
  export let record;
  export let showName = true;
</script>

<dl class="participant-values">
  {#if showName}<div>
      <dt>Nombre</dt>
      <dd>{record.display_name}</dd>
    </div>{/if}
  <div>
    <dt>Rol en el expediente</dt>
    <dd>{record.profile ? profileLabel(record.procedural_role) : record.procedural_role}</dd>
  </div>
  <div>
    <dt>Organizaci&#243;n</dt>
    <dd>{record.organization || 'Sin organizaci\u00f3n registrada'}</dd>
  </div>
  <div>
    <dt>Situaci&#243;n jur&#237;dica registrada</dt>
    <dd>{record.legal_status || 'Sin situaci\u00f3n registrada'}</dd>
  </div>
  <div>
    <dt>Estado en el directorio</dt>
    <dd>
      <span class="badge" class:info={record.directory_status === 'active'}
        >{record.directory_status === 'active' ? 'Activo' : 'Archivado'}</span
      >
    </dd>
  </div>
</dl>

{#if record.profile && record.subject}<ParticipantTypedSummary {record} />{:else}<p class="hint">
    Ficha pendiente de tipificar.
  </p>{/if}
