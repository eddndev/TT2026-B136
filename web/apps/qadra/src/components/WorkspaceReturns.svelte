<script>
  export let incident = false,
    alert = false,
    activity = false,
    onnavigate,
    onactivity;
</script>

{#if incident}<button
    class="text-button alerts-return"
    onclick={() => onnavigate('integrity-incidents')}>Volver a incidentes</button
  >{/if}
{#if alert}<button class="text-button alerts-return" onclick={() => onnavigate('alerts')}
    >Volver a Alertas</button
  >{/if}
{#if activity}<button class="text-button alerts-return" onclick={onactivity}
    >Volver a actividad</button
  >{/if}
