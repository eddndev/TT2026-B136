<script>
  import { calendarEntities } from '../lib/judicial-calendar-labels.mjs';
  export let scope,
    disabled = false;
</script>

<fieldset class="calendar-fieldset" {disabled}>
  <legend>&#193;mbito declarado</legend>
  <p class="hint">
    El &#225;mbito queda fijo al publicar. Los nombres y claves no acreditan competencia judicial.
  </p>
  <div class="calendar-form-grid">
    <label>T&#237;tulo del calendario<input bind:value={scope.title} /></label>
    <label
      >Fuero<select bind:value={scope.jurisdiction}
        ><option value="">Selecciona el fuero</option><option value="federal">Federal</option
        ><option value="local">Local</option></select
      ></label
    >
    <label>Autoridad<input bind:value={scope.authority} /></label>
    <label>&#211;rgano<input bind:value={scope.organ} /></label>
    <label>Territorio<input bind:value={scope.territory} /></label>
  </div>
  <label>Uso declarado<textarea rows="3" bind:value={scope.use_description}></textarea></label>
  <fieldset class="calendar-fieldset">
    <legend>Entidades del &#225;mbito</legend>
    <p class="hint">
      Selecci&#243;n expresa de 1 a 32 entidades. No se elige un alcance nacional
      autom&#225;ticamente.
    </p>
    <div class="calendar-entities">
      {#each calendarEntities as [code, name]}<label class="calendar-check"
          ><input type="checkbox" value={code} bind:group={scope.entity_codes} />{code}
          {name}</label
        >{/each}
    </div>
  </fieldset>
</fieldset>
