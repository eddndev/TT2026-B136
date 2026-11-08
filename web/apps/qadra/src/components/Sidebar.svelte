<script>
  import { tick } from 'svelte';
  import HeroIcon from './HeroIcon.svelte';
  import { canHearings } from '../lib/hearings.mjs';
  import { canAlerts } from '../lib/alerts-presentation.mjs';
  import { readSidebarCollapsed, storeSidebarCollapsed } from '../lib/sidebar-state.mjs';
  export let user;
  export let view;
  export let onnavigate;
  let drawer, panel, aside, collapseButton;
  // Desktop only: the mobile drawer always shows labels.
  let collapsed = readSidebarCollapsed();
  export function open() {
    drawer.showModal();
  }
  function go(destination) {
    drawer.close();
    onnavigate(destination);
  }
  // The control that was used becomes hidden or changes its role, so focus
  // moves to the control that reverses the change. The brand button is also
  // rendered in the mobile drawer, so it is looked up inside the desktop aside.
  async function toggle() {
    collapsed = !collapsed;
    storeSidebarCollapsed(collapsed);
    await tick();
    (collapsed ? aside?.querySelector('[data-sidebar-brand]') : collapseButton)?.focus();
  }
  const expandLabel = 'Expandir men\u00fa';
  const client = user.role === 'client';
  const caseViews = [
    'documents',
    'participants',
    'case-summary',
    'case-members',
    'stages',
    'hearings',
    'case-measures',
    'resolutions',
    'resources',
    'deadlines',
    'case-contracts',
  ];
  const work = client
    ? [
        { id: 'contracts', label: 'Mis contratos', icon: 'pencil' },
        { id: 'signing-certificates', label: 'Mi firma digital', icon: 'fingerprint' },
        { id: 'evidence-validation', label: 'Validar evidencia', icon: 'badge' },
      ]
    : [
        { id: 'overview', label: 'Inicio', icon: 'home' },
        { id: 'cases', label: 'Expedientes', icon: 'briefcase' },
        ...(['owner', 'litigator'].includes(user.role)
          ? [
              { id: 'contracts', label: 'Por firmar', icon: 'pencil' },
              { id: 'reports', label: 'Informes', icon: 'chart' },
            ]
          : []),
        ...(canHearings(user.role, 'read')
          ? [{ id: 'agenda', label: 'Agenda', icon: 'calendar' }]
          : []),
        ...(canAlerts(user.role) ? [{ id: 'alerts', label: 'Alertas', icon: 'bell' }] : []),
      ];
  const admin = [
    { id: 'team', label: 'Equipo', icon: 'users' },
    { id: 'audit', label: 'Bit\u00e1cora', icon: 'audit' },
    { id: 'judicial-calendars', label: 'Calendarios', icon: 'calendar' },
  ];
  const account = [
    ...(['owner', 'litigator'].includes(user.role)
      ? [{ id: 'signing-certificates', label: 'Mi firma digital', icon: 'fingerprint' }]
      : []),
    ...(client ? [] : [{ id: 'guide', label: 'Gu\u00eda de uso', icon: 'help' }]),
  ];
  const isCurrent = (id, current) =>
    current === id ||
    (id === 'cases' && caseViews.includes(current)) ||
    (id === 'audit' && current === 'integrity-incidents') ||
    (id === 'signing-certificates' && current === 'owner-certificates');
  // Both states share one structure: the width animates and the text only
  // fades, so no row changes its height or position while the menu toggles.
  const fade = (compact) =>
    `whitespace-nowrap transition-opacity duration-200 motion-reduce:transition-none ${
      compact ? 'opacity-0' : 'opacity-100'
    }`;
  // Brand sidebar: navy chrome with light text. Every control keeps the
  // shared focus ring, recolored white so it stays visible on navy.
  const itemClass = (active) =>
    `group flex w-full items-center gap-x-3 overflow-hidden rounded-md p-2 text-left text-label leading-6 focus-visible:outline-white ${
      active ? 'bg-white/10 text-white' : 'text-brand-100 hover:bg-white/5 hover:text-white'
    }`;
  const iconClass = (active) =>
    `size-6 ${active ? 'text-white' : 'text-brand-300 group-hover:text-white'}`;
</script>

{#snippet item(entry, compact)}
  {@const active = isCurrent(entry.id, view)}
  <li>
    <button
      class={itemClass(active)}
      aria-current={active ? 'page' : undefined}
      title={compact ? entry.label : undefined}
      onclick={() => go(entry.id)}
      ><HeroIcon name={entry.icon} class={iconClass(active)} /><span class={fade(compact)}
        >{entry.label}</span
      ></button
    >
  </li>
{/snippet}

{#snippet heading(text, compact)}
  <div class="relative mb-1 h-6 overflow-hidden">
    <div
      aria-hidden="true"
      class={`absolute top-1/2 left-0 h-px w-6 bg-white/10 transition-opacity duration-200 motion-reduce:transition-none ${
        compact ? 'opacity-100' : 'opacity-0'
      }`}
    ></div>
    <div class={`text-caption leading-6 text-brand-300 ${fade(compact)}`}>{text}</div>
  </div>
{/snippet}

{#snippet content(compact, desktop)}
  <div class="flex h-16 shrink-0 items-center">
    <button
      class={`group -mx-2 flex shrink-0 items-center gap-x-3 overflow-hidden rounded-md p-2 transition-[max-width] duration-200 focus-visible:outline-white motion-reduce:transition-none ${
        compact ? 'max-w-10 hover:bg-white/5' : 'max-w-40'
      }`}
      data-sidebar-brand
      onclick={compact ? toggle : () => go(client ? 'contracts' : 'overview')}
      aria-label={compact ? expandLabel : 'Qadra, ir al inicio'}
      aria-expanded={compact ? 'false' : undefined}
      title={compact ? expandLabel : undefined}
    >
      <span class="relative size-6 shrink-0">
        <img
          src="/brand/qadra/mark-light.svg"
          alt=""
          class={`size-6 max-w-none ${compact ? 'group-hover:opacity-0 group-focus-visible:opacity-0' : ''}`}
        />
        <span
          class={`absolute inset-0 opacity-0 ${
            compact ? 'group-hover:opacity-100 group-focus-visible:opacity-100' : ''
          }`}><HeroIcon name="sidebar" class="size-6 text-brand-300" /></span
        >
      </span>
      <span class={`text-section-title tracking-tight text-white ${fade(compact)}`}>Qadra</span>
    </button>
    <!-- Visibility only animates while hiding: when shown, the button must be
    focusable at once. -->
    {#if desktop}<button
        class={`-mr-2 ml-auto shrink-0 rounded-md p-2 text-brand-300 duration-200 hover:bg-white/5 hover:text-white focus-visible:outline-white motion-reduce:transition-none ${
          compact
            ? 'invisible opacity-0 transition-[opacity,visibility]'
            : 'opacity-100 transition-opacity'
        }`}
        bind:this={collapseButton}
        onclick={toggle}
        aria-label="Contraer men&#250;"
        aria-expanded="true"
        title="Contraer men&#250;"><HeroIcon name="sidebar" class="size-6" /></button
      >{/if}
  </div>
  <nav aria-label="Navegaci&#243;n principal" class="flex flex-1 flex-col">
    <ul role="list" class="flex flex-1 flex-col gap-y-6">
      <li>
        <ul role="list" class="-mx-2 space-y-1">
          {#each work as entry}{@render item(entry, compact)}{/each}
        </ul>
      </li>
      {#if user.role === 'owner'}
        <li>
          {@render heading('Administraci\u00f3n', compact)}
          <ul role="list" class="-mx-2 space-y-1">
            {#each admin as entry}{@render item(entry, compact)}{/each}
          </ul>
        </li>
      {/if}
      {#if account.length}
        <li class="mt-auto">
          <ul role="list" class="-mx-2 space-y-1">
            {#each account as entry}{@render item(entry, compact)}{/each}
          </ul>
        </li>
      {/if}
    </ul>
  </nav>
{/snippet}

<!-- Collapsed width: 1.5rem icon + 2 x 1.25rem padding + the 1px right border. -->
<aside
  bind:this={aside}
  class={`desktop-sidebar hidden transition-[width] duration-200 lg:fixed lg:inset-y-0 lg:z-50 lg:flex lg:flex-col ${collapsed ? 'lg:w-[4.0625rem]' : 'lg:w-64'}`}
  data-collapsed={collapsed ? '' : undefined}
>
  <div
    class="flex grow flex-col gap-y-4 overflow-x-hidden overflow-y-auto border-r border-white/10 bg-navy px-5 pb-4"
  >
    {@render content(collapsed, true)}
  </div>
</aside>

<dialog
  bind:this={drawer}
  aria-label="Men&#250; del despacho"
  class="m-0 h-full max-h-none w-full max-w-none bg-transparent p-0 backdrop:bg-ink/80 lg:hidden"
  onclick={(event) => {
    if (!panel.contains(event.target)) drawer.close();
  }}
>
  <div class="flex h-full">
    <div class="relative mr-16 flex w-full max-w-xs flex-1">
      <div class="absolute top-0 left-full flex w-16 justify-center pt-5">
        <button class="-m-2.5 p-2.5" onclick={() => drawer.close()} aria-label="Cerrar men&#250;"
          ><HeroIcon name="close" class="size-6 text-white" /></button
        >
      </div>
      <div bind:this={panel} class="flex grow flex-col gap-y-4 overflow-y-auto bg-navy px-5 pb-4">
        {@render content(false, false)}
      </div>
    </div>
  </div>
</dialog>
