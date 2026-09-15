<script>
  import Icon from './Icon.svelte';
  import { can } from '../lib/documents.mjs';
  export let user;
  export let selectedCase = null;
  export let onnavigate;
  export let ondocument;
  const today = new Intl.DateTimeFormat('es-MX', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(new Date());
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">TU DESPACHO DIGITAL</span>
    <h1>Tu mesa de trabajo</h1>
    <p>Todo listo para continuar con tus expedientes.</p>
  </div>
  <span class="today"><Icon name="calendar" size={16} />{today}</span>
</div>
<section class="welcome-card">
  <div>
    <span class="eyebrow">MENOS PASOS. M&#193;S CLARIDAD.</span>
    <h2>Tu trabajo, en un solo lugar.</h2>
    <p>Organiza los documentos de cada expediente y conserva su evidencia.</p>
    <div class="action-row">
      <button class="primary" onclick={() => onnavigate('cases')}
        ><Icon name="briefcase" size={18} />Ver expedientes</button
      ><button class="text-button" onclick={() => onnavigate('guide')}
        >C&#243;mo funciona<Icon name="arrow" size={16} /></button
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
</section>
<div class="overview-grid">
  <section class="card recent-panel">
    <div class="section-heading">
      <h2>Tu expediente actual</h2>
      <button class="text-button" onclick={() => onnavigate('cases')}
        >Ver expedientes<Icon name="arrow" size={15} /></button
      >
    </div>
    {#if selectedCase}<div class="empty-state">
        <span class="empty-icon"><Icon name="briefcase" size={33} /></span>
        <h3>{selectedCase.title}</h3>
        <p>{selectedCase.reference}</p>
        <button class="secondary" onclick={() => onnavigate('documents')}>Abrir expediente</button>
      </div>
    {:else}<div class="empty-state">
        <span class="empty-icon"><Icon name="folder" size={33} /></span>
        <h3>Aqu&#237; empieza tu trabajo</h3>
        <p>Selecciona un expediente para consultar sus datos y documentos.</p>
        <button class="secondary" onclick={() => onnavigate('cases')}>Seleccionar expediente</button
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
    {#if selectedCase && can(user.role, 'documents')}<button
        class="secondary"
        onclick={() => ondocument({ type: 'upload' })}
        >Subir documento<Icon name="plus" size={16} /></button
      >{/if}
  </aside>
</div>
