<script>
  import Button from './Button.svelte';
  import { canParticipants } from '../lib/participants.mjs';
  import SectionTabs from './SectionTabs.svelte';
  export let record;
  export let user;
  export let view;
  export let onnavigate;
  export let onchange;
  $: sections = [
    { id: 'case-summary', label: 'Resumen' },
    { id: 'documents', label: 'Documentos' },
    { id: 'participants', label: 'Participantes' },
    { id: 'stages', label: 'Etapas' },
    { id: 'hearings', label: 'Audiencias' },
    { id: 'case-measures', label: 'Medidas cautelares' },
    { id: 'resolutions', label: 'Resoluciones' },
    { id: 'resources', label: 'Recursos' },
    { id: 'deadlines', label: 'Plazos' },
    { id: 'case-contracts', label: 'Contratos' },
    ...(user.role === 'owner' ? [{ id: 'case-members', label: 'Asignaciones' }] : []),
  ];
</script>

<section class="card case-context">
  <div>
    <span class="eyebrow">EXPEDIENTE ACTUAL</span>
    <h2>{record.title}</h2>
    <p>{record.reference}</p>
  </div>
  <div class="action-row">
    <Button variant="secondary" onclick={onchange}>Cambiar expediente</Button>
  </div>
</section>
{#if canParticipants(user.role, 'read')}
  <!-- Eleven sections do not fit one row on tablets, so the select stays up to lg. -->
  <SectionTabs
    items={sections}
    current={view}
    label="Secciones del expediente"
    onselect={onnavigate}
    breakpoint="lg"
  />
{/if}
