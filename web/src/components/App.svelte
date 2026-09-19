<script>
  import { onMount, tick } from 'svelte';
  import Auth from './Auth.svelte';
  import Sidebar from './Sidebar.svelte';
  import Overview from './Overview.svelte';
  import Guide from './Guide.svelte';
  import Icon from './Icon.svelte';
  import Cases from './Cases.svelte';
  import CaseWorkspace from './CaseWorkspace.svelte';
  import Admin from './Admin.svelte';
  import Agenda from './Agenda.svelte';
  import Alerts from './Alerts.svelte';
  import JudicialCalendars from './JudicialCalendars.svelte';
  import '../styles/judicial-calendars.css';
  import '../styles/alerts.css';
  import { createApi } from '../lib/api.mjs';
  import { roles } from '../lib/documents.mjs';
  import { normalizeView, viewLabels } from '../lib/workspace.mjs';
  let user = null;
  let selectedCase = null;
  let hearingIntent = null,
    deadlineIntent = null,
    agendaFilters = null;
  let alertFilters = null,
    alertReturn = false;

  let view = 'overview';
  let documentIntent = null;
  let notice = '';
  let logoutBusy = false;
  let error = '';
  let sidebar;
  let main;
  function reset(message = '') {
    user = null;
    selectedCase = null;
    hearingIntent = null;
    deadlineIntent = null;
    agendaFilters = null;
    alertFilters = null;
    alertReturn = false;

    documentIntent = null;
    view = 'overview';
    notice = message;
    history.replaceState(null, '', '#overview');
  }
  const api = createApi(globalThis.fetch, () =>
    reset('Tu sesi\u00f3n termin\u00f3. Vuelve a iniciar sesi\u00f3n.'),
  );
  async function logout() {
    logoutBusy = true;
    error = '';
    try {
      await api.logout();
      reset();
    } catch (failure) {
      error = failure.message;
    } finally {
      logoutBusy = false;
    }
  }
  async function go(destination) {
    view = normalizeView(destination, user?.role);
    if (view !== 'hearings') hearingIntent = null;
    if (view !== 'deadlines') deadlineIntent = null;
    if (
      ![
        'case-summary',
        'documents',
        'participants',
        'stages',
        'hearings',
        'resolutions',
        'deadlines',
      ].includes(view)
    )
      alertReturn = false;
    if (location.hash !== `#${view}`) location.hash = view;
    await tick();
    main?.focus({ preventScroll: true });
    window.scrollTo(0, 0);
  }
  function openDocument(intent) {
    documentIntent = intent;
    go(selectedCase ? 'documents' : 'cases');
  }
  onMount(() => {
    const onHash = () => {
      if (user && location.hash !== `#${view}`) go(location.hash);
    };
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  });
</script>

{#if !user}<Auth
    {api}
    {notice}
    onlogin={(principal) => {
      user = principal;
      notice = '';
      error = '';
      go(location.hash);
    }}
  />
{:else}
  <a
    class="skip-link"
    href="#main-content"
    onclick={(event) => {
      event.preventDefault();
      main?.focus();
    }}>Saltar al contenido</a
  >
  <div class="app-layout">
    <Sidebar
      bind:this={sidebar}
      {user}
      {view}
      onnavigate={(next) => {
        documentIntent = null;
        alertReturn = false;
        go(next);
      }}
      onlogout={logout}
      busy={logoutBusy}
    />
    <div class="app-content">
      <header class="topbar">
        <div class="topbar-location">
          <button
            class="icon-button menu-toggle"
            aria-label="Abrir men&#250;"
            onclick={() => sidebar.open()}><Icon name="menu" /></button
          ><span>Mi despacho<span class="breadcrumb">/ {viewLabels[view]}</span></span>
        </div>
        <div class="topbar-identity">
          <span class="badge info"><Icon name="shield" size={13} />Acceso con MFA</span><span
            class="role-label">{roles[user.role] || user.role}</span
          ><span class="avatar" title={user.email}>{user.email.slice(0, 2).toUpperCase()}</span>
        </div>
      </header>
      <main id="main-content" tabindex="-1" bind:this={main}>
        {#if error}<p class="notice error" role="alert">{error}</p>{/if}
        {#if view === 'overview'}<Overview
            {user}
            {selectedCase}
            onnavigate={go}
            ondocument={openDocument}
          />
        {:else if view === 'judicial-calendars'}<JudicialCalendars {api} {user} />
        {:else if view === 'alerts'}<Alerts
            {api}
            {user}
            bind:filters={alertFilters}
            onopen={(record, intent) => {
              selectedCase = record;
              alertReturn = true;
              hearingIntent = intent.kind === 'hearing' ? intent : null;
              deadlineIntent = intent.kind === 'deadline' ? intent : null;
              documentIntent = null;
              go(intent.kind === 'deadline' ? 'deadlines' : 'hearings');
            }}
          />
        {:else if view === 'agenda'}<Agenda
            {api}
            bind:filters={agendaFilters}
            oncalendars={() => go('judicial-calendars')}
            onopen={(record, intent) => {
              selectedCase = record;
              hearingIntent = intent.kind === 'hearing' ? intent : null;
              deadlineIntent = intent.kind === 'deadline' ? intent : null;
              documentIntent = null;
              go(intent.kind === 'deadline' ? 'deadlines' : 'hearings');
            }}
          />
        {:else if view === 'cases'}<Cases
            {api}
            {user}
            onselect={(record) => {
              selectedCase = record;

              go(documentIntent ? 'documents' : 'case-summary');
            }}
          />
        {:else if ['case-summary', 'documents', 'participants', 'stages', 'hearings', 'resolutions', 'deadlines'].includes(view)}
          {#if alertReturn}<button class="text-button alerts-return" onclick={() => go('alerts')}
              >Volver a Alertas</button
            >{/if}
          {#if selectedCase}{#key selectedCase.id}<CaseWorkspace
                {api}
                {user}
                record={selectedCase}
                {view}
                onnavigate={go}
                onupdate={(record) => (selectedCase = record)}
                onchange={() => {
                  selectedCase = null;
                  go('cases');
                }}
                {hearingIntent}
                onhearingintent={() => (hearingIntent = null)}
                {deadlineIntent}
                ondeadlineintent={() => (deadlineIntent = null)}
                intent={documentIntent}
                onintent={() => (documentIntent = null)}
              />{/key}
          {:else}<section class="card empty-state">
              <span class="empty-icon"><Icon name="briefcase" size={35} /></span>
              <h1>Selecciona un expediente</h1>
              <p>Abre un expediente para consultar sus datos.</p>
              <button class="primary" onclick={() => go('cases')}>Ver expedientes</button>
            </section>{/if}
        {:else if view === 'guide'}<Guide onnavigate={go} />
        {:else if user.role === 'owner'}{#key view}<Admin
              {api}
              view={view === 'team' ? 'users' : 'audit'}
            />{/key}{/if}
        <footer class="workspace-footer">
          <span>Qadra / Despacho digital</span><span
            >Entorno local <span class="footer-separator">/</span> TT2026-B136</span
          >
        </footer>
      </main>
    </div>
  </div>
{/if}
