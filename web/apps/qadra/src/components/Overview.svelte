<script>
  import Button from './Button.svelte';
  import Icon from '@qadra/ui/Icon.svelte';
  import Dashboard from './Dashboard.svelte';
  import OverviewHeading from './OverviewHeading.svelte';
  import { can } from '../lib/documents.mjs';
  import { canAlerts } from '../lib/alerts-presentation.mjs';
  export let user;
  export let api;
  export let selectedCase = null;
  export let onnavigate;
  export let ondocument;
</script>

{#if ['owner', 'litigator'].includes(user.role)}
  <Dashboard {api} {user} {onnavigate} />
{:else}
  <OverviewHeading {user} {onnavigate} />
{/if}
<section class="welcome-card">
  <div>
    <span class="eyebrow">MENOS PASOS. M&#193;S CLARIDAD.</span>
    <h2>Tu trabajo, en un solo lugar.</h2>
    <p>Organiza los documentos de cada expediente y conserva su evidencia.</p>
    <div class="action-row">
      <Button variant="primary" onclick={() => onnavigate('cases')}
        ><Icon name="briefcase" size={18} />Ver expedientes</Button
      ><Button variant="text" onclick={() => onnavigate('guide')}
        >C&#243;mo funciona<Icon name="arrow" size={16} /></Button
      >
    </div>
  </div>
  <div class="welcome-symbol" aria-hidden="true">
    <Icon name="folder" size={84} /><span><Icon name="shield" size={32} /></span>
  </div>
</section>
<div class="section-heading session-heading">
  <h2>Contin&#250;a tu trabajo</h2>
  <span class="hint">Accesos a tus expedientes y documentos</span>
</div>
<section class="stats-grid overview-actions" aria-label="Acciones del despacho">
  <button class="stat-card" onclick={() => onnavigate('cases')}
    ><div>
      <span>Expedientes</span><strong>Consultar</strong><small
        >Archivo disponible para tu cuenta</small
      >
    </div>
    <span class="stat-icon"><Icon name="briefcase" size={23} /></span></button
  >
  {#if can(user.role, 'documents')}
    <button class="stat-card" onclick={() => ondocument({ filter: 'pending' })}
      ><div>
        <span>Pendientes de sello</span><strong>Revisar</strong><small
          >Selecciona el expediente</small
        >
      </div>
      <span class="stat-icon warning"><Icon name="clock" size={23} /></span></button
    >
    <button class="stat-card" onclick={() => ondocument({ filter: 'sealed' })}
      ><div>
        <span>Documentos sellados</span><strong>Consultar</strong><small
          >Comprueba y descarga evidencia</small
        >
      </div>
      <span class="stat-icon success"><Icon name="shield" size={23} /></span></button
    >
  {:else}<button class="stat-card" onclick={() => onnavigate('guide')}
      ><div>
        <span>Gu&#237;a de uso</span><strong>Conocer</strong><small>Funciones de tu cuenta</small>
      </div>
      <span class="stat-icon"><Icon name="help" size={23} /></span></button
    >{/if}
  {#if canAlerts(user.role)}<button class="stat-card" onclick={() => onnavigate('alerts')}>
      <div>
        <span>Mis alertas</span><strong>Consultar</strong><small
          >Avisos personales y preferencias</small
        >
      </div>
      <span class="stat-icon warning"><Icon name="clock" size={23} /></span>
    </button>{/if}
</section>
<div class="overview-grid">
  <section class="card recent-panel">
    <div class="section-heading">
      <h2>Tu expediente actual</h2>
      <Button variant="text" onclick={() => onnavigate('cases')}
        >Ver expedientes<Icon name="arrow" size={15} /></Button
      >
    </div>
    {#if selectedCase}<div class="empty-state">
        <span class="empty-icon"><Icon name="briefcase" size={33} /></span>
        <h3>{selectedCase.title}</h3>
        <p>{selectedCase.reference}</p>
        <Button variant="secondary" onclick={() => onnavigate('documents')}>Abrir expediente</Button
        >
      </div>
    {:else}<div class="empty-state">
        <span class="empty-icon"><Icon name="folder" size={33} /></span>
        <h3>Aqu&#237; empieza tu trabajo</h3>
        <p>Selecciona un expediente para consultar sus datos y documentos.</p>
        <Button variant="secondary" onclick={() => onnavigate('cases')}
          >Seleccionar expediente</Button
        >
      </div>{/if}
    <p class="panel-footnote">
      Los expedientes y documentos permanecen guardados al cerrar sesi&#243;n.
    </p>
  </section>
  <aside class="card next-panel">
    <span class="tile-icon"><Icon name="checklist" size={23} /></span>
    <h2>Tu siguiente paso</h2>
    <p>Cada archivo sigue un recorrido sencillo dentro de su expediente.</p>
    <ol class="mini-steps">
      <li>
        <span>1</span>
        <div><strong>Carga</strong><small>El archivo se guarda cifrado.</small></div>
      </li>
      <li>
        <span>2</span>
        <div><strong>Sella</strong><small>Genera la firma y el sello local.</small></div>
      </li>
      <li>
        <span>3</span>
        <div>
          <strong>Verifica y descarga</strong><small>Revisa y conserva tu evidencia.</small>
        </div>
      </li>
    </ol>
    {#if selectedCase && can(user.role, 'documents')}<Button
        variant="secondary"
        onclick={() => ondocument({ type: 'upload' })}
        >Subir documento<Icon name="plus" size={16} /></Button
      >{/if}
  </aside>
</div>
