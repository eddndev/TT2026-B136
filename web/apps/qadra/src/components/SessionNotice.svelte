<script>
  export let phase,
    pending = false,
    oncheck,
    onlogout;
</script>

<section class="notice" role="status">
  {#if phase === 'hidden'}
    Vuelve a esta pantalla para confirmar tu sesi&oacute;n antes de abrir el despacho.
  {:else}
    Comprobando tu sesi&oacute;n.
    {#if pending}El despacho se abrir&aacute; cuando se confirme tu acceso.
    {:else}Tus cambios siguen en esta pantalla.{/if}
  {/if}
  <button class="secondary" disabled={phase !== 'checking'} onclick={oncheck}
    >Comprobar de nuevo</button
  >
  <button class="text-button" onclick={onlogout}>Cerrar sesi&oacute;n</button>
</section>
