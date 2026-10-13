import { alertUuid as uuid, alertInvalid as invalid } from './alerts-primitives.mjs';
import {
  alertPreferenceCommand,
  alertReadCommand,
  alertPreferencesEnvelope,
} from './alerts-preference-values.mjs';
import { alertsQuery } from './alerts-query.mjs';
import { alertPageValue, alertDetailValue } from './alerts-values.mjs';

export function alertsApi(request, actor) {
  uuid(actor);
  let active = true;
  const assertActive = () => {
    if (!active) invalid('Esta consulta personal de alertas ya no esta abierta.');
  };
  async function call(path, options) {
    assertActive();
    try {
      const value = await request(path, options);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      throw error;
    }
  }
  return {
    dispose() {
      active = false;
    },
    async list(input) {
      assertActive();
      const { query, parameters } = alertsQuery(input);
      return alertPageValue(await call(`/alerts?${parameters}`), actor, query);
    },
    async get(id) {
      uuid(id);
      return alertDetailValue(await call(`/alerts/${id}`), actor, id);
    },
    async markRead(id, input) {
      uuid(id);
      const command = alertReadCommand(input);
      return alertDetailValue(
        await call(`/alerts/${id}/read`, {
          method: 'POST',
          data: command,
        }),
        actor,
        id,
        command.operation_id,
      );
    },
    async preferences() {
      return alertPreferencesEnvelope(await call('/alert-preferences'), actor);
    },
    async savePreferences(input) {
      const command = alertPreferenceCommand(input);
      return alertPreferencesEnvelope(
        await call('/alert-preferences', {
          method: 'PUT',
          data: structuredClone(command),
        }),
        actor,
        command,
      );
    },
  };
}
