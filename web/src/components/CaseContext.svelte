<script>
  import { canParticipants } from '../lib/participants.mjs';
  export let record;
  export let user;
  export let view;
  export let onnavigate;
  export let onchange;
</script>

<section class="card case-context">
  <div>
    <span class="eyebrow">EXPEDIENTE ACTUAL</span>
    <h2>{record.title}</h2>
    <p>{record.reference}</p>
  </div>
  <div class="action-row">
    <button class="secondary" onclick={onchange}>Cambiar expediente</button>
  </div>
</section>
{#if canParticipants(user.role, 'read')}<nav
    class="case-sections"
    aria-label="Secciones del expediente"
  >
    {#each [{ id: 'documents', label: 'Documentos' }, { id: 'participants', label: 'Participantes' }] as item}
      <a
        href={`#${item.id}`}
        class:active={view === item.id}
        aria-current={view === item.id ? 'page' : undefined}
        onclick={(event) => {
          event.preventDefault();
          onnavigate(item.id);
        }}>{item.label}</a
      >
    {/each}
  </nav>{/if}
