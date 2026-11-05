<script>
  import { tick } from 'svelte';
  import HeroIcon from './HeroIcon.svelte';
  import { canHearings } from '../lib/hearings.mjs';
  import { canAlerts } from '../lib/alerts-presentation.mjs';
  import { readSidebarCollapsed, storeSidebarCollapsed } from '../lib/sidebar-state.mjs';
  export let user;
  export let view;
  export let recent = [];
  export let onnavigate;
  export let onopencase = () => {};
  let drawer, panel, expandButton, collapseButton;
  // Desktop only: the mobile drawer always shows labels.
  let collapsed = readSidebarCollapsed();
  export function open() {
    drawer.showModal();
  }
  function go(destination) {
    drawer.close();
    onnavigate(destination);
  }
  function openCase(record) {
    drawer.close();
    onopencase(record);
  }
  // The toggle is replaced by its counterpart, so focus follows it.
  async function toggle() {
    collapsed = !collapsed;
    storeSidebarCollapsed(collapsed);
    await tick();
    (collapsed ? expandButton : collapseButton)?.focus();
  }
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
  const itemClass = (active) =>
    `group flex w-full items-center gap-x-3 rounded-md p-2 text-left text-sm/6 font-semibold ${
      active
        ? 'bg-brand-50 text-brand-800'
        : 'text-slate-700 hover:bg-slate-50 hover:text-brand-800'
    }`;
  const iconClass = (active) =>
    `size-6 ${active ? 'text-brand-800' : 'text-slate-400 group-hover:text-brand-800'}`;
  const initial = (record) =>
    (record.title || record.reference || '?').trim().charAt(0).toUpperCase();
</script>

{#snippet item(entry, compact)}
  {@const active = isCurrent(entry.id, view)}
  <li>
    <button
      class={itemClass(active)}
      aria-current={active ? 'page' : undefined}
      title={compact ? entry.label : undefined}
      onclick={() => go(entry.id)}
      ><HeroIcon name={entry.icon} class={iconClass(active)} /><span class:sr-only={compact}
        >{entry.label}</span
      ></button
    >
  </li>
{/snippet}

{#snippet heading(text, compact)}
  {#if compact}
    <div aria-hidden="true" class="mx-auto mb-3 h-px w-6 bg-line"></div>
    <div class="sr-only">{text}</div>
  {:else}
    <div class="mb-1 text-xs/6 font-semibold text-slate-400">{text}</div>
  {/if}
{/snippet}

{#snippet content(compact, desktop)}
  <div class="flex h-16 shrink-0 items-center justify-between">
    {#if compact}
      <button
        class="group -mx-2 rounded-md p-2 hover:bg-slate-50"
        bind:this={expandButton}
        onclick={toggle}
        aria-label="Expandir men&#250;"
        aria-expanded="false"
        title="Expandir men&#250;"
      >
        <img
          src="/brand/qadra/favicon.svg"
          alt=""
          class="size-6 max-w-none shrink-0 group-hover:hidden group-focus-visible:hidden"
        />
        <span class="hidden group-hover:block group-focus-visible:block"
          ><HeroIcon name="sidebar" class="size-6 text-slate-500" /></span
        >
      </button>
    {:else}
      <button
        class="-mx-2 flex items-center gap-x-3 rounded-md p-2"
        onclick={() => go(client ? 'contracts' : 'overview')}
        aria-label="Qadra, ir al inicio"
        ><img src="/brand/qadra/favicon.svg" alt="" class="size-6 max-w-none shrink-0" /><span
          class="text-lg font-semibold tracking-tight text-navy">Qadra</span
        ></button
      >
      {#if desktop}<button
          class="-mr-2 rounded-md p-2 text-slate-400 hover:bg-slate-50 hover:text-slate-600"
          bind:this={collapseButton}
          onclick={toggle}
          aria-label="Contraer men&#250;"
          aria-expanded="true"
          title="Contraer men&#250;"><HeroIcon name="sidebar" class="size-6" /></button
        >{/if}
    {/if}
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
      {#if !client && recent.length}
        <li>
          {@render heading('Expedientes recientes', compact)}
          <ul role="list" class="-mx-2 space-y-1">
            {#each recent as record (record.id)}
              <li>
                <a
                  href="#case-summary"
                  class="group flex w-full items-center gap-x-3 rounded-md p-2 text-sm/6 font-semibold text-slate-700 no-underline hover:bg-slate-50 hover:text-brand-800"
                  title={`${record.title} (${record.reference})`}
                  aria-label={compact ? record.title : undefined}
                  onclick={(event) => {
                    event.preventDefault();
                    openCase(record);
                  }}
                  ><span
                    aria-hidden="true"
                    class="flex size-6 shrink-0 items-center justify-center rounded-lg border border-line bg-white text-[0.625rem] font-medium text-slate-400 group-hover:border-brand-800 group-hover:text-brand-800"
                    >{initial(record)}</span
                  >{#if !compact}<span class="truncate">{record.title}</span>{/if}</a
                >
              </li>
            {/each}
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
  class={`desktop-sidebar hidden transition-[width] duration-200 lg:fixed lg:inset-y-0 lg:z-50 lg:flex lg:flex-col ${collapsed ? 'lg:w-[4.0625rem]' : 'lg:w-64'}`}
  data-collapsed={collapsed ? '' : undefined}
>
  <div
    class="flex grow flex-col gap-y-4 overflow-x-hidden overflow-y-auto border-r border-line bg-white px-5 pb-4"
  >
    {@render content(collapsed, true)}
  </div>
</aside>

<dialog
  bind:this={drawer}
  aria-label="Men&#250; del despacho"
  class="m-0 h-full max-h-none w-full max-w-none bg-transparent p-0 backdrop:bg-slate-900/80 lg:hidden"
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
      <div bind:this={panel} class="flex grow flex-col gap-y-4 overflow-y-auto bg-white px-5 pb-4">
        {@render content(false, false)}
      </div>
    </div>
  </div>
</dialog>
