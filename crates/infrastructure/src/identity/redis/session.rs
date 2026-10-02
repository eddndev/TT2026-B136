use application::identity::{
    Principal, SessionGrant, SessionIdentity, SessionPolicy, SessionState,
};
use application::ApplicationError;
use domain::identity::{Role, UserId};

use super::{port_error, validate_generation, RedisSessionStore};

type ScriptState = (String, i64, i64, i64);

impl RedisSessionStore {
    pub(super) fn create_timed_session(
        &self,
        identity: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<SessionGrant, ApplicationError> {
        let identity_json = serialize_identity(identity)?;
        let access_token = self.random_token()?;
        let state = self
            .invoke_session_script("create", &access_token, &identity_json, policy)?
            .ok_or_else(|| ApplicationError::Port("Redis rejected session creation".into()))?;
        Ok(SessionGrant {
            access_token,
            state,
        })
    }

    pub(super) fn read_timed_session(
        &self,
        token: &str,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        self.invoke_session_script("read", token, "", policy)
    }

    pub(super) fn touch_timed_session(
        &self,
        token: &str,
        expected: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        let identity_json = serialize_identity(expected)?;
        self.invoke_session_script("activity", token, &identity_json, policy)
    }

    fn invoke_session_script(
        &self,
        operation: &str,
        token: &str,
        identity_json: &str,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        let value: Option<ScriptState> = redis::Script::new(include_str!("session.lua"))
            .key(self.digest_key("session", token.as_bytes()))
            .arg(operation)
            .arg(policy.absolute_ttl_seconds() * 1_000)
            .arg(policy.idle_ttl_seconds().unwrap_or(0) * 1_000)
            .arg(identity_json)
            .invoke(&mut self.connection()?)
            .map_err(port_error)?;
        Ok(value.and_then(decode_state))
    }
}

fn serialize_identity(identity: &SessionIdentity) -> Result<String, ApplicationError> {
    validate_generation(identity.auth_generation)?;
    serde_json::to_string(identity)
        .map_err(|error| ApplicationError::Port(format!("session serialization failed: {error}")))
}

fn decode_state(value: ScriptState) -> Option<SessionState> {
    let (identity_json, now, absolute, idle) = value;
    let identity = serde_json::from_str::<StoredSession>(&identity_json).ok()?;
    if identity.auth_generation > i64::MAX as u64 {
        return None;
    }
    Some(SessionState {
        identity: SessionIdentity {
            principal: Principal {
                id: identity.principal.id,
                email: identity.principal.email,
                role: identity.principal.role,
            },
            auth_generation: identity.auth_generation,
        },
        server_now_unix_ms: now,
        absolute_expires_at_unix_ms: absolute,
        idle_expires_at_unix_ms: (idle != 0).then_some(idle),
    })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSession {
    principal: StoredPrincipal,
    auth_generation: u64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPrincipal {
    id: UserId,
    email: String,
    role: Role,
}
