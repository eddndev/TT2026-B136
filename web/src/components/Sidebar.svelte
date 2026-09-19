<script>
  import Icon from './Icon.svelte';
  import { canHearings } from '../lib/hearings.mjs';
  import { canAlerts } from '../lib/alerts-presentation.mjs';
  import Brand from './Brand.svelte';
  import { roles } from '../lib/documents.mjs';
  export let user;
  export let view;
  export let onnavigate;
  export let onlogout;
  export let busy = false;
  let drawer;
  export function open() {
    drawer.showModal();
  }
  function go(destination) {
    drawer.close();
    onnavigate(destination);
  }
  const work = [
    { id: 'overview', label: 'Inicio', icon: 'home' },
    { id: 'cases', label: 'Expedientes', icon: 'briefcase' },
    { id: 'documents', label: 'Documentos', icon: 'folder' },
    ...(canHearings(user.role, 'read')
      ? [{ id: 'agenda', label: 'Agenda', icon: 'calendar' }]
      : []),
    ...(canAlerts(user.role) ? [{ id: 'alerts', label: 'Alertas', icon: 'clock' }] : []),
  ];
  const admin = [
    { id: 'team', label: 'Equipo', icon: 'users' },
    { id: 'audit', label: 'Auditor\u00eda', icon: 'shield' },
    { id: 'judicial-calendars', label: 'Calendarios jurisdiccionales', icon: 'calendar' },
  ];
</script>

{#snippet content()}
  <button class="brand" onclick={() => go('overview')} aria-label="Qadra, ir al inicio"
    ><Brand /></button
  >
  <div class="workspace-label">
    <span class="workspace-monogram"><Icon name="briefcase" size={18} /></span>
    <div><strong>Mi despacho</strong><small>Espacio de trabajo</small></div>
  </div>
  <nav aria-label="Navegaci&#243;n principal">
    <span class="eyebrow nav-label">ESPACIO DE TRABAJO</span>
    {#each work as item}<button
        class:active={view === item.id ||
          (item.id === 'cases' &&
            [
              'participants',
              'case-summary',
              'stages',
              'hearings',
              'resolutions',
              'resources',
              'deadlines',
            ].includes(view))}
        aria-current={view === item.id ||
        (item.id === 'cases' &&
          [
            'participants',
            'case-summary',
            'stages',
            'hearings',
            'resolutions',
            'resources',
            'deadlines',
          ].includes(view))
          ? 'page'
          : undefined}
        onclick={() => go(item.id)}><Icon name={item.icon} />{item.label}</button
      >{/each}
    {#if user.role === 'owner'}<span class="eyebrow nav-label admin-label">ADMINISTRACI&#211;N</span
      >{#each admin as item}<button
          class:active={view === item.id}
          aria-current={view === item.id ? 'page' : undefined}
          onclick={() => go(item.id)}><Icon name={item.icon} />{item.label}</button
        >{/each}{/if}
  </nav>
  <div class="sidebar-bottom">
    <button class="guide-link" class:active={view === 'guide'} onclick={() => go('guide')}
      ><Icon name="help" size={19} />Gu&#237;a de uso<Icon name="arrow" size={15} /></button
    >
    <div class="profile">
      <span class="avatar">{user.email.slice(0, 2).toUpperCase()}</span>
      <div>
        <strong title={user.email}>{user.email}</strong><small
          >{roles[user.role] || user.role}</small
        >
      </div>
    </div>
    <button class="logout" disabled={busy} onclick={onlogout}
      ><Icon name="logout" size={17} />{busy ? 'Cerrando...' : 'Cerrar sesi\u00f3n'}</button
    >
  </div>
{/snippet}

<aside class="sidebar desktop-sidebar">{@render content()}</aside>
<dialog class="sidebar mobile-sidebar" bind:this={drawer} aria-label="Men&#250; del despacho">
  <button
    class="mobile-close icon-button"
    onclick={() => drawer.close()}
    aria-label="Cerrar men&#250;"><Icon name="close" /></button
  >{@render content()}
</dialog>
