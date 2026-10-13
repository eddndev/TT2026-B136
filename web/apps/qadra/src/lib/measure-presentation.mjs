import { factTimeLabel } from './procedural-fact-time.mjs';

export const measureKinds = {
  periodic_appearance: 'Presentacion periodica',
  financial_guarantee: 'Garantia economica',
  asset_seizure: 'Embargo de bienes',
  account_freeze: 'Inmovilizacion de cuentas y valores',
  travel_restriction: 'Restriccion de salida del ambito fijado',
  custody_or_institution: 'Cuidado, vigilancia o internamiento indicado',
  place_restriction: 'Restriccion de reuniones o lugares',
  contact_restriction: 'Restriccion de convivencia, acercamiento o comunicacion',
  home_separation: 'Separacion del domicilio',
  public_office_suspension: 'Suspension en cargo publico',
  professional_suspension: 'Suspension de actividad profesional o laboral',
  electronic_monitoring: 'Localizador electronico',
  home_confinement: 'Resguardo domiciliario',
  pretrial_detention: 'Prision preventiva',
};

export const measureActions = {
  impose: 'Imposicion',
  confirm: 'Confirmacion',
  modify: 'Modificacion',
  revoke: 'Revocacion',
  cease: 'Cese',
  substitute_out: 'Sustituida',
  substitute_in: 'Sustituta',
};

export function measureTimeLabel(value) {
  return value.precision === 'unknown'
    ? `Fecha desconocida: ${value.reason}`
    : factTimeLabel(value);
}
