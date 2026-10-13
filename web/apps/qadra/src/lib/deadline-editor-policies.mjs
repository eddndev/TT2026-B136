import { factInvalid as invalid } from './procedural-fact-primitives.mjs';

const dependencies = ['profile', 'source', 'calendar'];
const labels = { profile: 'al perfil', source: 'a la fuente', calendar: 'al calendario' };
const explicit = (value) => ['fixed', 'follow'].includes(value);

function dependencyKeys(definition) {
  const selection = definition.input.selection;
  const prefix = selection.case_id;
  const source = selection.source.kind === 'known' ? selection.source.value : null;
  let sourceKey = '';
  if (source?.id && source.revision) {
    sourceKey = `${prefix}:${source.family}:${source.id}`;
    if (source.family === 'notification')
      sourceKey = source.resolution?.id ? `${sourceKey}:${source.resolution.id}` : '';
  } else if (source?.result_id && source.hearing_id && source.revision) {
    sourceKey = `${prefix}:hearing_result:${source.hearing_id}:${source.result_id}`;
  }
  const profile = definition.profile;
  const calendar = definition.input.calendar;
  return {
    profile: profile?.id && profile.revision ? `${prefix}:profile:${profile.id}` : '',
    source: sourceKey,
    calendar: calendar?.id && calendar.revision ? `${prefix}:calendar:${calendar.id}` : '',
  };
}

export function initialDeadlinePolicies(definition, policies = null) {
  const keys = dependencyKeys(definition);
  return Object.fromEntries(
    dependencies.map((dependency) => [
      dependency,
      {
        key: keys[dependency],
        value: keys[dependency] && explicit(policies?.[dependency]) ? policies[dependency] : '',
      },
    ]),
  );
}

export function reconcileDeadlinePolicies(state, definition) {
  const keys = dependencyKeys(definition);
  return Object.fromEntries(
    dependencies.map((dependency) => [
      dependency,
      {
        key: keys[dependency],
        value:
          keys[dependency] && keys[dependency] === state[dependency].key
            ? state[dependency].value
            : '',
      },
    ]),
  );
}

export function deadlinePoliciesCommand(state, definition) {
  const reconciled = reconcileDeadlinePolicies(state, definition);
  const absent = {
    profile: false,
    source: definition.input.selection.source.kind === 'unknown',
    calendar: definition.input.calendar === null,
  };
  return Object.fromEntries(
    dependencies.map((dependency) => {
      if (absent[dependency]) return [dependency, 'undetermined'];
      const choice = reconciled[dependency];
      if (!choice.key || !explicit(choice.value))
        invalid(`Selecciona como dar seguimiento ${labels[dependency]}.`);
      return [dependency, choice.value];
    }),
  );
}

export function adoptDeadlineDefinition(previous, draft, next) {
  const clone = (value) => structuredClone(value);
  const choose = (before, edited, updated) =>
    clone(JSON.stringify(before) === JSON.stringify(edited) ? updated : edited);
  const profileGroup = (definition) => ({
    profile: definition.profile,
    qualification: definition.input.qualification,
    ordered_quantity: definition.input.ordered_quantity,
  });
  const profile = choose(profileGroup(previous), profileGroup(draft), profileGroup(next));
  return {
    title: choose(previous.title, draft.title, next.title),
    responsible_id: choose(previous.responsible_id, draft.responsible_id, next.responsible_id),
    profile: profile.profile,
    input: {
      selection: choose(previous.input.selection, draft.input.selection, next.input.selection),
      calendar: choose(previous.input.calendar, draft.input.calendar, next.input.calendar),
      qualification: profile.qualification,
      ordered_quantity: profile.ordered_quantity,
    },
  };
}
