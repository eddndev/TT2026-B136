<script>
  import { alertPreferenceGroups } from '../lib/alerts-presentation.mjs';
  export let values,
    hours,
    disabled = false;
</script>

<div class="alert-preference-groups">
  {#each alertPreferenceGroups as group}
    <fieldset {disabled} class="alert-preference-group">
      <legend>{group.label}</legend>
      {#if group.family}
        <label
          >Anticipaciones de {group.noun} (horas)
          <input bind:value={hours[group.key]} inputmode="numeric" autocomplete="off" />
        </label>
        <p class="hint">
          Hasta ocho horas distintas entre 1 y 720, separadas por comas. Vac&#237;o desactiva la
          anticipaci&#243;n.
        </p>
        <div class="alert-channels">
          <label
            ><input type="checkbox" bind:checked={values[group.key].channels.internal} /> Internas
            para {group.noun}</label
          >
          <label
            ><input type="checkbox" bind:checked={values[group.key].channels.email} /> Correo para {group.noun}</label
          >
        </div>
      {:else}
        <div class="alert-channels">
          <label
            ><input type="checkbox" bind:checked={values[group.key].internal} /> Internas para {group.noun}</label
          >
          <label
            ><input type="checkbox" bind:checked={values[group.key].email} /> Correo para {group.noun}</label
          >
        </div>
      {/if}
    </fieldset>
  {/each}
</div>
