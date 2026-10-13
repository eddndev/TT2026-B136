import { factObject as object, factInvalid as invalid } from './procedural-fact-primitives.mjs';

export function deadlineTrackingPolicies(raw, present, state = 'accepted', reasons = []) {
  const names = ['profile', 'source', 'calendar'];
  object(raw, names);
  if (names.some((name) => !['fixed', 'follow', 'undetermined'].includes(raw[name]))) invalid();
  if (state === 'legacy_undeclared') {
    if (names.some((name) => raw[name] !== 'undetermined')) invalid();
    return raw;
  }
  names.forEach((name, i) => {
    const policy = raw[name];
    const requirements = reasons.filter((value) => value.dependency === name);
    if (!present[i]) {
      if (policy !== 'undetermined' || requirements.length) invalid();
      return;
    }
    if (state === 'accepted' && policy === 'undetermined') invalid();
    if (
      state === 'pending' &&
      policy === 'undetermined' &&
      !requirements.some((value) => value.reason === 'policy_undetermined')
    )
      invalid();
    for (const { reason } of requirements) {
      if (['source_changed', 'profile_changed'].includes(reason) && policy !== 'follow') invalid();
      if (reason === 'policy_undetermined' && policy !== 'undetermined') invalid();
    }
  });
  return raw;
}
