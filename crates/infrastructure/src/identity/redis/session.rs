use application::identity::certificate_login::{
    CertificateSessionProvenance, SessionAuthentication,
};
use application::identity::{SessionGrant, SessionIdentity, SessionPolicy, SessionState};
use application::ApplicationError;

use super::{
    certificate_wire::{self, Authentication},
    validate_generation, wire, RedisSessionStore,
};

type ScriptState = (String, String, i64, i64, i64, i64);

struct CertificateArguments {
    authentication: String,
    ceiling: i64,
    from: i64,
    until: i64,
}

impl CertificateArguments {
    fn new(origin: &CertificateSessionProvenance) -> Option<Self> {
        let (from, until) = certificate_wire::bounds(origin)?;
        let authentication = serde_json::to_string(&Authentication::from_origin(origin)).ok()?;
        if authentication.len() > 16_384 {
            return None;
        }
        Some(Self {
            authentication,
            ceiling: 0,
            from,
            until,
        })
    }
}

impl RedisSessionStore {
    pub(super) fn create_timed_session(
        &self,
        identity: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<SessionGrant, ApplicationError> {
        let identity_json = serialize_identity(identity)?;
        let access_token = self.random_token()?;
        let state = self
            .invoke_session_script("create", &access_token, &identity_json, policy, None)?
            .ok_or_else(unavailable)?;
        Ok(SessionGrant {
            access_token,
            state,
        })
    }

    pub(super) fn create_certificate_timed_session(
        &self,
        identity: &SessionIdentity,
        origin: &CertificateSessionProvenance,
        policy: SessionPolicy,
        ceiling: i64,
    ) -> Result<SessionGrant, ApplicationError> {
        let mut certificate = CertificateArguments::new(origin).ok_or_else(invalid)?;
        if !certificate_wire::matches(identity, origin)
            || !(1..=wire::MAX_EXACT).contains(&ceiling)
            || ceiling > certificate.until
        {
            return Err(invalid());
        }
        certificate.ceiling = ceiling;
        let identity_json = serialize_identity(identity)?;
        let access_token = self.random_token()?;
        let state = self
            .invoke_session_script(
                "create_certificate",
                &access_token,
                &identity_json,
                policy,
                Some(&certificate),
            )?
            .ok_or_else(invalid)?;
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
        self.invoke_session_script("read", token, "", policy, None)
    }

    pub(super) fn touch_timed_session(
        &self,
        token: &str,
        expected: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        let identity_json = serialize_identity(expected)?;
        self.invoke_session_script("activity", token, &identity_json, policy, None)
    }

    pub(super) fn touch_certificate_timed_session(
        &self,
        token: &str,
        expected: &SessionIdentity,
        origin: &CertificateSessionProvenance,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        if !certificate_wire::matches(expected, origin) {
            return Ok(None);
        }
        let Some(certificate) = CertificateArguments::new(origin) else {
            return Ok(None);
        };
        let identity_json = serialize_identity(expected)?;
        self.invoke_session_script(
            "activity_certificate",
            token,
            &identity_json,
            policy,
            Some(&certificate),
        )
    }

    fn invoke_session_script(
        &self,
        operation: &str,
        token: &str,
        identity_json: &str,
        policy: SessionPolicy,
        certificate: Option<&CertificateArguments>,
    ) -> Result<Option<SessionState>, ApplicationError> {
        let value: Option<ScriptState> = redis::Script::new(include_str!("session.lua"))
            .key(self.digest_key("session", token.as_bytes()))
            .arg(operation)
            .arg(policy.absolute_ttl_seconds() * 1_000)
            .arg(policy.idle_ttl_seconds().unwrap_or(0) * 1_000)
            .arg(identity_json)
            .arg(certificate.map_or("", |value| value.authentication.as_str()))
            .arg(certificate.map_or(0, |value| value.ceiling))
            .arg(certificate.map_or(0, |value| value.from))
            .arg(certificate.map_or(0, |value| value.until))
            .invoke(&mut self.connection().map_err(|_| unavailable())?)
            .map_err(|_| unavailable())?;
        Ok(value.and_then(decode_state))
    }
}

fn serialize_identity(identity: &SessionIdentity) -> Result<String, ApplicationError> {
    validate_generation(identity.auth_generation)?;
    serde_json::to_string(identity).map_err(|_| unavailable())
}

fn decode_state(value: ScriptState) -> Option<SessionState> {
    let (identity_json, authentication_json, now, absolute, idle, ceiling) = value;
    let identity = wire::identity(&identity_json)?;
    let authentication = if authentication_json.is_empty() && ceiling == 0 {
        SessionAuthentication::Password
    } else {
        let origin = wire::parse::<Authentication>(&authentication_json)?.into_origin()?;
        let (from, until) = certificate_wire::bounds(&origin)?;
        if !certificate_wire::matches(&identity, &origin)
            || now < from
            || now >= until
            || ceiling <= now
            || ceiling > until
            || absolute > ceiling
        {
            return None;
        }
        SessionAuthentication::Certificate(origin.into())
    };
    Some(SessionState {
        identity,
        authentication,
        server_now_unix_ms: now,
        absolute_expires_at_unix_ms: absolute,
        idle_expires_at_unix_ms: (idle != 0).then_some(idle),
    })
}

fn invalid() -> ApplicationError {
    ApplicationError::InvalidInput("invalid certificate session state".into())
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("session store unavailable".into())
}
