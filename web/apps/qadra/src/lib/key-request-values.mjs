const encoder = new TextEncoder();

function subject(value, label) {
  if (
    typeof value !== 'string' ||
    !value.trim() ||
    /[\u0000-\u001f\u007f-\u009f]/.test(value) ||
    encoder.encode(value).byteLength > 128
  )
    throw new Error(`${label} debe contener entre 1 y 128 bytes de texto, sin controles.`);
  return value;
}

export function keyRequestValues(draft) {
  if (!draft || typeof draft !== 'object' || Array.isArray(draft))
    throw new Error('Completa los datos de la solicitud.');
  const commonName = subject(draft.commonName, 'El nombre');
  const organization = subject(draft.organization, 'La organizaci\u00f3n');
  const country = draft.country;
  if (typeof country !== 'string' || !/^[A-Z]{2}$/.test(country))
    throw new Error('El pa\u00eds debe tener dos letras may\u00fasculas, por ejemplo MX.');
  const password = draft.password;
  if (
    typeof password !== 'string' ||
    [...password].length < 12 ||
    encoder.encode(password).byteLength > 1024
  )
    throw new Error('La contrase\u00f1a debe tener al menos 12 caracteres y hasta 1024 bytes.');
  if (draft.passwordConfirmation !== password)
    throw new Error('La confirmaci\u00f3n de la contrase\u00f1a no coincide.');
  return { commonName, organization, country, password };
}
