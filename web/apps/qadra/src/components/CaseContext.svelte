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
    { id: 'case-summary', label: 'Resumen', icon: 'document' },
    { id: 'documents', label: 'Documentos', icon: 'folder' },
    { id: 'participants', label: 'Participantes', icon: 'users' },
    { id: 'stages', label: 'Etapas', icon: 'flag' },
    { id: 'hearings', label: 'Audiencias', icon: 'scale' },
    { id: 'case-measures', label: 'Medidas cautelares', icon: 'hand' },
    { id: 'resolutions', label: 'Resoluciones', icon: 'documentCheck' },
    { id: 'resources', label: 'Recursos', icon: 'appeal' },
    { id: 'deadlines', label: 'Plazos', icon: 'clock' },
    { id: 'case-contracts', label: 'Contratos', icon: 'pencil' },
    ...(user.role === 'owner'
      ? [{ id: 'case-members', label: 'Asignaciones', icon: 'userGroup' }]
      : []),
  ];
</script>

<section class="card case-context">
  <div>
    <span class="eyebrow">Expediente actual</span>
    <h2>{record.title}</h2>
    <p class="font-mono">{record.reference}</p>
  </div>
  <div class="action-row">
    <Button variant="secondary" onclick={onchange}>Cambiar expediente</Button>
  </div>
</section>
{#if canParticipants(user.role, 'read')}
  <SectionTabs
    items={sections}
    current={view}
    label="Secciones del expediente"
    onselect={onnavigate}
  />
{/if}
