<script>
  import { onMount, tick, setContext, onDestroy } from 'svelte';
  import Auth from './Auth.svelte';
  import SessionNotice from './SessionNotice.svelte';
  import Sidebar from './Sidebar.svelte';
  import Overview from './Overview.svelte';
  import CaseReports from './CaseReports.svelte';
  import Guide from './Guide.svelte';
  import Icon from './Icon.svelte';
  import Cases from './Cases.svelte';
  import CaseWorkspace from './CaseWorkspace.svelte';
  import Admin from './Admin.svelte';
  import Agenda from './Agenda.svelte';
  import Alerts from './Alerts.svelte';
  import DocumentIntegrityNotice from './DocumentIntegrityNotice.svelte';
  import DocumentIntegrityIncidents from './DocumentIntegrityIncidents.svelte';
  import JudicialCalendars from './JudicialCalendars.svelte';
  import OwnerCertificates from './OwnerCertificates.svelte';
  import '../styles/judicial-calendars.css';
  import '../styles/alerts.css';
  import { createApi } from '../lib/api.mjs';
  import { takePasswordResetLink } from '../lib/password-reset-link.mjs';
  import { createDraftRegistry } from '../lib/draft-registry.mjs';
  import { createSessionLifecycle } from '../lib/session-lifecycle.mjs';
  import { createSessionDialogs } from '../lib/session-dialogs.mjs';
  import { roles } from '../lib/documents.mjs';
  import { normalizeView, viewLabels } from '../lib/workspace.mjs';
  let user = null;
  let pendingUser = null;
  let selectedCase = null;
  let hearingIntent = null,
    deadlineIntent = null,
    agendaFilters = null;
  let alertFilters = null,
    alertReturn = false;
  let resourceIntent = null,
    activityReturn = null;
  let view = 'overview';
  let documentIntent = null;
  let incidentReturn = false,
    integrityNotice;
  let notice = '';
  let recoveryLink = null;
  let error = '';
  let sidebar;
  let main;
  function reset(message = '') {
    user = null;
    pendingUser = null;
    selectedCase = null;
    hearingIntent = null;
    deadlineIntent = null;
    agendaFilters = null;
    alertFilters = null;
    alertReturn = false;
    resourceIntent = null;
    activityReturn = null;
    documentIntent = null;
    incidentReturn = false;
    view = 'overview';
    notice = message;
    history.replaceState(null, '', '#overview');
  }
  let appLayout;
  let sessionState = { phase: 'signed-out' };
  const dialogs = createSessionDialogs({ root: () => appLayout, settled: tick });
  const drafts = createDraftRegistry();
  const api = createApi(globalThis.fetch, () => lifecycle.expire('rejected'));
  const lifecycle = createSessionLifecycle({
    api,
    drafts,
    onState(state) {
      sessionState = state;
      dialogs.transition(state.phase);
      if (state.phase === 'active' && pendingUser) {
        const confirmed = pendingUser;
        pendingUser = null;
        if (confirmed.id !== state.principalId) {
          lifecycle.expire('invalid-state');
          return;
        }
        user = confirmed;
        notice = '';
        error = '';
        go(location.hash);
      } else if (state.phase === 'expired') {
        reset('Tu sesi\u00f3n termin\u00f3. Vuelve a iniciar sesi\u00f3n.');
        if (state.captureFailures)
          notice += ' No se pudieron conservar algunos cambios pendientes.';
      } else if (state.phase === 'signed-out' && state.reason !== 'initial') {
        reset(
          state.logoutUncertain
            ? 'Saliste de esta pantalla. No se pudo confirmar el cierre en el servidor.'
            : '',
        );
      }
    },
  });
  setContext('session-drafts', {
    registry: drafts,
    principal: () => user,
    canAdmit: () => lifecycle.canAdmit(),
    async authorizeCase(caseId) {
      const scoped = api.caseAdministration(caseId);
      try {
        return await scoped.get();
      } catch (failure) {
        if ([403, 404].includes(failure.status)) drafts.denyContext(caseId);
        throw failure;
      } finally {
        scoped.dispose();
      }
    },
  });
  function logout() {
    return lifecycle.logout();
  }
  function login(session, receipt) {
    pendingUser = session?.user ?? null;
    if (!lifecycle.acceptSession(session, receipt)) pendingUser = null;
  }
  onDestroy(() => {
    pendingUser = null;
    lifecycle.dispose();
    dialogs.dispose();
  });
  async function go(destination) {
    if (!lifecycle.canAdmit()) {
      if (user) history.replaceState(null, '', `#${view}`);
      return;
    }
    view = normalizeView(destination, user?.role);
    if (view !== 'resources') {
      resourceIntent = null;
      activityReturn = null;
    }
    if (view !== 'hearings') hearingIntent = null;
    if (view !== 'deadlines') deadlineIntent = null;
    if (view !== 'documents') incidentReturn = false;
    if (
      ![
        'case-summary',
        'documents',
        'participants',
        'stages',
        'hearings',
        'case-measures',
        'resolutions',
        'resources',
        'deadlines',
      ].includes(view)
    )
      alertReturn = false;
    if (location.hash !== `#${view}`) location.hash = view;
    await tick();
    main?.focus({ preventScroll: true });
    window.scrollTo(0, 0);
  }
  function openRelatedResource(intent) {
    if (!lifecycle.canAdmit()) return;
    if (!user || intent.case_id !== selectedCase?.id) return;
    resourceIntent = intent;
    activityReturn = intent.origin;
    go('resources');
  }
  function returnToActivity() {
    if (!lifecycle.canAdmit()) return;
    const origin = activityReturn;
    if (!origin || origin.case_id !== selectedCase?.id) return;
    hearingIntent = origin.kind === 'hearing' ? origin : null;
    deadlineIntent = origin.kind === 'deadline' ? origin : null;
    go(origin.kind === 'hearing' ? 'hearings' : 'deadlines');
  }
  function openDocument(intent) {
    if (!lifecycle.canAdmit()) return;
    documentIntent = intent;
    go(selectedCase ? 'documents' : 'cases');
  }
  onMount(() => {
    const onHash = () => {
      if (user || pendingUser) {
        const link = takePasswordResetLink();
        if (link) {
          recoveryLink = link;
          lifecycle.expire('rejected');
          return;
        }
      }
      if (user && location.hash !== `#${view}`) go(location.hash);
    };
    const onVisibility = () => lifecycle.visibilityChanged(document.visibilityState);
    const onActivity = (event) =>
      lifecycle.activity({ isTrusted: event.isTrusted, type: event.type });
    window.addEventListener('hashchange', onHash);
    document.addEventListener('visibilitychange', onVisibility);
    onVisibility();
    const events = ['pointerdown', 'keydown', 'input'];
    events.forEach((name) => document.addEventListener(name, onActivity, true));
    return () => {
      window.removeEventListener('hashchange', onHash);
      document.removeEventListener('visibilitychange', onVisibility);
      events.forEach((name) => document.removeEventListener(name, onActivity, true));
    };
  });
</script>

{#if !user}
  {#if pendingUser}
    <SessionNotice
      phase={sessionState.phase}
      pending
      oncheck={() => lifecycle.visibilityChanged('visible')}
      onlogout={logout}
    />
  {:else}<Auth
      {api}
      {notice}
      onlogin={login}
      initialResetLink={recoveryLink}
      onresetconsumed={() => {
        recoveryLink = null;
      }}
    />{/if}
{:else}
  {#if sessionState.phase === 'checking'}
    <SessionNotice
      phase={sessionState.phase}
      oncheck={() => lifecycle.visibilityChanged('visible')}
      onlogout={logout}
    />
  {/if}
  <a
    class="skip-link"
    href="#main-content"
    onclick={(event) => {
      event.preventDefault();
      main?.focus();
    }}>Saltar al contenido</a
  >
  <div class="app-layout" bind:this={appLayout} inert={sessionState.phase !== 'active'}>
    <Sidebar
      bind:this={sidebar}
      {user}
      {view}
      onnavigate={(next) => {
        if (!lifecycle.canAdmit()) return;
        documentIntent = null;
        incidentReturn = false;
        alertReturn = false;
        activityReturn = null;
        resourceIntent = null;
        go(next);
      }}
      onlogout={logout}
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
        {#if user.role === 'owner'}<DocumentIntegrityNotice
            bind:this={integrityNotice}
            {api}
            onopen={() => go('integrity-incidents')}
          />{/if}
        {#if error}<p class="notice error" role="alert">{error}</p>{/if}
        {#if view === 'overview'}<Overview
            {api}
            {user}
            {selectedCase}
            onnavigate={go}
            ondocument={openDocument}
          />
        {:else if view === 'integrity-incidents' && user.role === 'owner'}<DocumentIntegrityIncidents
            {api}
            onknown={(exists) => integrityNotice?.observed(exists)}
            onopen={(record, intent) => {
              if (!lifecycle.canAdmit()) return;
              selectedCase = record;
              documentIntent = intent;
              hearingIntent = null;
              deadlineIntent = null;
              alertReturn = false;
              incidentReturn = true;
              go('documents');
            }}
          />
        {:else if view === 'reports'}<CaseReports {api} {user} />
        {:else if view === 'judicial-calendars'}<JudicialCalendars {api} {user} />
        {:else if view === 'alerts'}<Alerts
            {api}
            {user}
            bind:filters={alertFilters}
            onopen={(record, intent) => {
              if (!lifecycle.canAdmit()) return;
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
              if (!lifecycle.canAdmit()) return;
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
              if (!lifecycle.canAdmit()) return;
              selectedCase = record;
              go(documentIntent ? 'documents' : 'case-summary');
            }}
          />
        {:else if ['case-summary', 'case-members', 'documents', 'participants', 'stages', 'hearings', 'case-measures', 'resolutions', 'resources', 'deadlines'].includes(view)}
          {#if incidentReturn}<button
              class="text-button alerts-return"
              onclick={() => go('integrity-incidents')}>Volver a incidentes</button
            >{/if}
          {#if alertReturn}<button class="text-button alerts-return" onclick={() => go('alerts')}
              >Volver a Alertas</button
            >{/if}
          {#if activityReturn && activityReturn.case_id === selectedCase?.id}<button
              class="text-button alerts-return"
              onclick={returnToActivity}>Volver a actividad</button
            >{/if}
          {#if selectedCase}{#key selectedCase.id}<CaseWorkspace
                {api}
                {user}
                record={selectedCase}
                {view}
                onnavigate={go}
                onupdate={(record) => (selectedCase = record)}
                onchange={() => {
                  if (!lifecycle.canAdmit()) return;
                  selectedCase = null;
                  go('cases');
                }}
                {resourceIntent}
                onresourceintent={() => (resourceIntent = null)}
                onresource={openRelatedResource}
                onrelateddenied={() => {
                  resourceIntent = null;
                  activityReturn = null;
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
        {:else if view === 'owner-certificates' && user.role === 'owner'}<OwnerCertificates
            {api}
            {user}
          />
        {:else if view === 'guide'}<Guide onnavigate={go} />
        {:else if user.role === 'owner'}{#key view}<Admin
              {api}
              {user}
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
