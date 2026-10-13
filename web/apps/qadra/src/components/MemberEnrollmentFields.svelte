<script>
  export let email,
    password,
    role,
    disabled = false,
    onchanged;
</script>

<label
  >Correo del nuevo usuario<input
    type="email"
    required
    autocomplete="off"
    bind:value={email}
    {disabled}
    oninput={onchanged}
  /></label
>
<label
  >Contrase&#241;a inicial<input
    type="password"
    minlength="12"
    required
    autocomplete="new-password"
    bind:value={password}
    {disabled}
  /></label
>
<small
  >M&#237;nimo 12 caracteres. La contrase&#241;a no se conserva al terminar la sesi&#243;n.</small
>
<label
  >Rol<select bind:value={role} {disabled} onchange={onchanged}>
    <option value="paralegal">Asistente legal</option>
    <option value="litigator">Litigante</option>
    <option value="owner">Administrador</option>
    <option value="client">Cliente</option>
  </select></label
>
<p class="notice">
  {role === 'owner'
    ? 'Acceso a documentos, sellado, auditor\u00eda y alta de usuarios.'
    : role === 'litigator'
      ? 'Puede cargar, sellar, verificar y exportar documentos.'
      : role === 'paralegal'
        ? 'Puede cargar, verificar y exportar documentos. No puede sellar.'
        : 'El rol cliente no tiene acceso documental en esta interfaz.'}
</p>
