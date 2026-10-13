<script>
  import { profileFields } from '../lib/case-administration.mjs';
  export let record;
</script>

<dl class="case-values">
  <div>
    <dt>T&#237;tulo</dt>
    <dd>{record.title}</dd>
  </div>
  <div>
    <dt>Referencia interna</dt>
    <dd>{record.reference}</dd>
  </div>
  <div>
    <dt>Administraci&#243;n</dt>
    <dd>{record.administrative_status === 'closed' ? 'Cerrado administrativamente' : 'Activo'}</dd>
  </div>
  {#if record.profile}
    {#each profileFields as field}<div class:case-value-wide={field.multiline}>
        <dt>{field.label}</dt>
        <dd class:case-multiline={field.multiline}>
          {record.profile[field.key] || 'Sin registrar'}
        </dd>
      </div>{/each}
    <div class="case-value-wide">
      <dt>Delitos registrados</dt>
      <dd>
        <ol>
          {#each record.profile.offenses as offense}<li>{offense}</li>{/each}
        </ol>
      </dd>
    </div>
  {:else}<div>
      <dt>Ficha penal</dt>
      <dd>Pendiente de completar</dd>
    </div>{/if}
</dl>
