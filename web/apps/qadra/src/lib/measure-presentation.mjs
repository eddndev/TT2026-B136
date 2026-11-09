import { factTimeLabel } from './procedural-fact-time.mjs';

export const measureKinds = {
  periodic_appearance: 'Presentaci\u00f3n peri\u00f3dica',
  financial_guarantee: 'Garant\u00eda econ\u00f3mica',
  asset_seizure: 'Embargo de bienes',
  account_freeze: 'Inmovilizaci\u00f3n de cuentas y valores',
  travel_restriction: 'Restricci\u00f3n de salida del \u00e1mbito fijado',
  custody_or_institution: 'Cuidado, vigilancia o internamiento indicado',
  place_restriction: 'Restricci\u00f3n de reuniones o lugares',
  contact_restriction: 'Restricci\u00f3n de convivencia, acercamiento o comunicaci\u00f3n',
  home_separation: 'Separaci\u00f3n del domicilio',
  public_office_suspension: 'Suspensi\u00f3n en cargo p\u00fablico',
  professional_suspension: 'Suspensi\u00f3n de actividad profesional o laboral',
  electronic_monitoring: 'Localizador electr\u00f3nico',
  home_confinement: 'Resguardo domiciliario',
  pretrial_detention: 'Prisi\u00f3n preventiva',
};

export const measureActions = {
  impose: 'Imposici\u00f3n',
  confirm: 'Confirmaci\u00f3n',
  modify: 'Modificaci\u00f3n',
  revoke: 'Revocaci\u00f3n',
  cease: 'Cese',
  substitute_out: 'Sustituida',
  substitute_in: 'Sustituta',
};

export function measureTimeLabel(value) {
  return value.precision === 'unknown'
    ? `Fecha desconocida: ${value.reason}`
    : factTimeLabel(value);
}
