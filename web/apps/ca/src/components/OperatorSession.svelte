<script>
  import Icon from '@qadra/ui/Icon.svelte';
  import CaOperations from './CaOperations.svelte';

  export let operator;
  export let remainingMs = 0;
  export let error = '';
  export let onlogout;
  export let oncommand;

  $: totalSeconds = Math.max(0, Math.ceil(remainingMs / 1000));
  $: minutes = Math.floor(totalSeconds / 60);
  $: seconds = String(totalSeconds % 60).padStart(2, '0');

  function focus(node) {
    node.focus();
  }
</script>

<div class="operator-workspace">
  <div class="page-heading">
    <div>
      <span class="eyebrow">AUTORIDAD CERTIFICADORA</span>
      <h1 tabindex="-1" use:focus>Operaciones de Qadra CA.</h1>
      <p>Revisa solicitudes y consulta los certificados emitidos.</p>
    </div>
    <button class="secondary" type="button" data-no-activity onclick={onlogout}>
      <Icon name="logout" size={18} />Cerrar sesi&oacute;n
    </button>
  </div>

  {#if error}<div class="notice error" role="alert">{error}</div>{/if}

  <details class="card operator-summary">
    <summary>{operator.email} &middot; Sesi&oacute;n {minutes}:{seconds}</summary>
    <dl class="settings-list">
      <div><dt>Correo electr&oacute;nico</dt><dd class="operator-email">{operator.email}</dd></div>
      <div><dt>Identificador de cuenta</dt><dd><code>{operator.id}</code></dd></div>
      <div>
        <dt>Estado del acceso</dt>
        <dd><span class="badge success"><Icon name="check" size={14} />Segundo factor verificado</span></dd>
      </div>
      <div>
        <dt>Tiempo restante de sesi&oacute;n</dt>
        <dd class="session-time"><Icon name="clock" size={17} /><span>{minutes}:{seconds}</span></dd>
      </div>
    </dl>
  </details>

  <CaOperations {oncommand} />
</div>
