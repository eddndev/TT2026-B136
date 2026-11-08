<script>
  import Button from './Button.svelte';
  export let incident = false,
    alert = false,
    activity = false,
    onnavigate,
    onactivity;
</script>

{#if incident}<Button
    variant="text"
    class="alerts-return"
    onclick={() => onnavigate('integrity-incidents')}>Volver a incidentes</Button
  >{/if}
{#if alert}<Button variant="text" class="alerts-return" onclick={() => onnavigate('alerts')}
    >Volver a Alertas</Button
  >{/if}
{#if activity}<Button variant="text" class="alerts-return" onclick={onactivity}
    >Volver a actividad</Button
  >{/if}
