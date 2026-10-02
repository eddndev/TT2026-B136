import { createSessionMonitor } from './session-monitor.mjs';

const activityTypes = new Set(['keydown', 'input', 'pointerdown']);
const expirationReasons = new Set(['deadline', 'rejected', 'invalid-state']);

export function createSessionLifecycle({
  api,
  drafts,
  now = () => performance.now(),
  setTimer = globalThis.setTimeout,
  clearTimer = globalThis.clearTimeout,
  onState = () => {},
}) {
  let phase = 'signed-out';
  let reason = 'initial';
  let principalId = null;
  let captureFailures = 0;
  let logoutUncertain = false;
  let generation = 0;
  let visibilityRevision = 0;
  let visible = true;
  let check = null;
  let activityFlight = null;
  let readFlight = null;
  let logoutFlight = null;

  const monitor = createSessionMonitor({
    api,
    now,
    setTimer,
    clearTimer,
    onExpired: (value) => expire(value),
  });

  function publish() {
    onState(Object.freeze({ phase, reason, principalId, captureFailures, logoutUncertain }));
  }

  function invalidateWork() {
    generation++;
    visibilityRevision++;
    check = null;
    activityFlight = null;
    readFlight = null;
    logoutFlight = null;
    monitor.stop();
  }

  function canAdmit() {
    return phase === 'active' && monitor.active();
  }

  function expire(value = 'rejected') {
    if (['signed-out', 'expired', 'disposed'].includes(phase)) return;
    phase = 'expired';
    reason = expirationReasons.has(value) ? value : 'rejected';
    invalidateWork();
    try {
      captureFailures = drafts.suspend().failed.length;
    } catch {
      captureFailures = 1;
    }
    principalId = null;
    logoutUncertain = false;
    api.invalidateSession();
    publish();
  }

  function acceptSession(value, receipt) {
    if (phase === 'disposed') return false;
    phase = 'checking';
    invalidateWork();
    if (drafts.active()) drafts.suspend();
    principalId = null;
    captureFailures = 0;
    logoutUncertain = false;
    try {
      monitor.start(value, receipt);
      if (phase === 'expired' || !monitor.active()) return false;
      drafts.activate(value.user.id);
      principalId = value.user.id;
    } catch {
      expire('invalid-state');
      return false;
    }
    phase = visible ? 'active' : 'hidden';
    reason = visible ? 'authenticated' : 'visibility-hidden';
    publish();
    return true;
  }

  function logout() {
    if (phase === 'disposed') return Promise.resolve(false);
    if (logoutFlight) return logoutFlight.promise;
    phase = 'signed-out';
    invalidateWork();
    const expected = generation;
    const flight = {};
    logoutFlight = flight;
    drafts.logout();
    principalId = null;
    captureFailures = 0;
    logoutUncertain = false;
    reason = 'logout-pending';
    let request;
    try {
      // The API captures the current bearer before invalidating it locally.
      request = api.logout();
    } catch (error) {
      request = Promise.reject(error);
    }
    api.invalidateSession();
    publish();
    function finish(confirmed) {
      if (generation === expected && logoutFlight === flight) {
        logoutUncertain = !confirmed;
        reason = confirmed ? 'logout-confirmed' : 'logout-unconfirmed';
        logoutFlight = null;
        publish();
      }
      return confirmed;
    }
    flight.promise = Promise.resolve(request).then(
      () => finish(true),
      () => finish(false),
    );
    return flight.promise;
  }

  function activity(event) {
    if (
      event?.isTrusted !== true ||
      !activityTypes.has(event.type) ||
      !canAdmit() ||
      activityFlight
    )
      return Promise.resolve(false);
    const expected = generation;
    const flight = {};
    activityFlight = flight;
    flight.promise = monitor
      .activity()
      .then((confirmed) => generation === expected && confirmed && canAdmit())
      .finally(() => {
        if (activityFlight === flight) activityFlight = null;
      });
    return flight.promise;
  }

  function visibilityChanged(value) {
    if (!['hidden', 'visible'].includes(value)) return Promise.resolve(false);
    visible = value === 'visible';
    if (['signed-out', 'expired', 'disposed'].includes(phase) || !monitor.active())
      return Promise.resolve(false);
    if (!visible) {
      visibilityRevision++;
      check = null;
      phase = 'hidden';
      reason = 'visibility-hidden';
      publish();
      return Promise.resolve(false);
    }
    if (check) return check.promise;
    const expected = generation;
    const revision = ++visibilityRevision;
    const predecessor = activityFlight?.promise ?? readFlight?.promise;
    const flight = {};
    check = flight;
    phase = 'checking';
    reason = 'visibility-check';
    publish();
    const current = () =>
      generation === expected && visibilityRevision === revision && check === flight && visible;
    flight.promise = (async () => {
      try {
        // A request started before this return cannot replace its fresh status read.
        if (predecessor) await predecessor;
        if (!current() || !monitor.active()) return false;
        const reading = { promise: monitor.refresh() };
        readFlight = reading;
        let confirmed;
        try {
          confirmed = await reading.promise;
        } finally {
          if (readFlight === reading) readFlight = null;
        }
        if (!current() || !monitor.active()) return false;
        phase = confirmed ? 'active' : 'checking';
        reason = confirmed ? 'confirmed' : 'session-unconfirmed';
        publish();
        return confirmed && canAdmit();
      } finally {
        if (check === flight) check = null;
      }
    })();
    return flight.promise;
  }

  const releaseGuard = api.setSessionGuard(canAdmit);

  function dispose() {
    if (phase === 'disposed') return;
    phase = 'disposed';
    invalidateWork();
    drafts.logout();
    principalId = null;
    captureFailures = 0;
    logoutUncertain = false;
    api.invalidateSession();
    releaseGuard();
    reason = 'disposed';
    publish();
  }

  publish();
  return { acceptSession, canAdmit, expire, logout, visibilityChanged, activity, dispose };
}
