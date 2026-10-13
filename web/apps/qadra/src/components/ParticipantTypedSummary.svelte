<script>
  import { profileKinds } from '../lib/typed-participant-fields.mjs';
  import ParticipantValueSummary from './ParticipantValueSummary.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import ParticipantLocatorSummary from './ParticipantLocatorSummary.svelte';
  export let record;
  $: schema = profileKinds.find((item) => item.key === record.profile.kind);
</script>

<div class="participant-comparison">
  <h3>Perfil tipificado</h3>
  <dl class="participant-values">
    {#each schema?.fields || [] as field}<div>
        <dt>{field.label}</dt>
        <dd><ParticipantValueSummary {field} value={record.profile[field.key]} /></dd>
      </div>{/each}
  </dl>
  <h4>Soporte del rol</h4>
  <ParticipantLocatorSummary value={record.role_support} />
  <h4>Identidad vinculada: revisi&#243;n {record.subject.revision}</h4>
  <ParticipantSubjectSummary record={record.subject} />
  <p class="hint">
    Estos datos corresponden a la identidad vinculada al registrar esta ficha. Consultar su
    identidad actual no cambia este registro.
  </p>
  {#if record.credential_origin}<p class="notice">
      Firma personal registrada para la revisi&#243;n {record.credential_origin
        .participant_revision} de la ficha. Consulta su evidencia para revisar la comprobaci&#243;n con
      la CA interna.
    </p>
  {:else}<p class="hint">Sin firma personal registrada en esta ficha.</p>{/if}
</div>
