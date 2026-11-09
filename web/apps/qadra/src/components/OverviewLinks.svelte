<script>
  // Shortcuts of the overview: the case index, the documents filtered by seal
  // state for the roles that read documents (the usage guide otherwise) and
  // the personal alerts of the roles that receive them.
  import HeroIcon from './HeroIcon.svelte';
  import { can } from '../lib/documents.mjs';
  import { canAlerts } from '../lib/alerts-presentation.mjs';
  export let user;
  export let onnavigate;
  export let ondocument;
  $: links = [
    { label: 'Expedientes', icon: 'briefcase', open: () => onnavigate('cases') },
    ...(can(user.role, 'documents')
      ? [
          {
            label: 'Pendientes de sello',
            icon: 'clock',
            open: () => ondocument({ filter: 'pending' }),
          },
          {
            label: 'Documentos sellados',
            icon: 'shield',
            open: () => ondocument({ filter: 'sealed' }),
          },
        ]
      : [{ label: 'Gu\u00eda de uso', icon: 'help', open: () => onnavigate('guide') }]),
    ...(canAlerts(user.role)
      ? [{ label: 'Mis alertas', icon: 'bell', open: () => onnavigate('alerts') }]
      : []),
  ];
</script>

<section class="overflow-hidden surface-card" aria-labelledby="overview-links-title">
  <h2 id="overview-links-title" class="px-6 pt-5 pb-2 text-card-title text-ink">Accesos</h2>
  <ul role="list" class="divide-y divide-line">
    {#each links as link (link.label)}
      <li>
        <button
          type="button"
          class="flex w-full items-center gap-x-3 px-6 py-3 text-left text-body text-ink transition-colors hover:bg-canvas focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-brand-800"
          onclick={link.open}
        >
          <HeroIcon name={link.icon} class="size-5 text-muted" />
          <span class="min-w-0 flex-auto">{link.label}</span>
          <HeroIcon name="chevronRight" class="size-4 text-muted" />
        </button>
      </li>
    {/each}
  </ul>
</section>
