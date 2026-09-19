<script>
  import { alertPreferenceGroups } from '../lib/alerts-presentation.mjs';
  export let current;
  const channelLabel = (value) =>
    [value.internal ? 'Internas' : '', value.email ? 'Correo' : ''].filter(Boolean).join(' y ') ||
    'Sin canales';
</script>

<section class="alert-preference-comparison" aria-label="Preferencias actuales guardadas">
  <h3>Preferencias actuales guardadas</h3>
  <p>Revisi&#243;n {current.revision}</p>
  <dl>
    {#each alertPreferenceGroups as group}
      <div>
        <dt>{group.label}</dt>
        <dd>
          {#if group.family}
            {current.values[group.key].lead_hours.length
              ? `${current.values[group.key].lead_hours.join(', ')} horas`
              : 'Sin anticipaciones'}
            / {channelLabel(current.values[group.key].channels)}
          {:else}{channelLabel(current.values[group.key])}{/if}
        </dd>
      </div>
    {/each}
  </dl>
  <p class="hint">Guardar mis preferencias reemplazar&#225; estos valores con el formulario.</p>
</section>
