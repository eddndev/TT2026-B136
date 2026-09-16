<script>
  import { profileLabel } from '../lib/typed-participant-fields.mjs';
  import Icon from './Icon.svelte';
  export let rows;
  export let onselect;
  export let opening;
</script>

<div class="participant-list">
  {#each rows as record}<button
      class="participant-row"
      disabled={opening}
      aria-label={`Abrir ${record.display_name}`}
      onclick={() => onselect(record)}
    >
      <span class="tile-icon"><Icon name="users" /></span>
      <span class="participant-row-text"
        ><strong>{record.display_name}</strong><span
          >{record.canonical_format === 'part2'
            ? profileLabel(record.procedural_role)
            : record.procedural_role}</span
        >{#if record.organization}<small>{record.organization}</small>{/if}</span
      >
      <span class="badge" class:info={record.directory_status === 'active'}
        >{record.directory_status === 'active' ? 'Activo' : 'Archivado'}</span
      ><Icon name="arrow" size={18} />
    </button>{/each}
</div>
