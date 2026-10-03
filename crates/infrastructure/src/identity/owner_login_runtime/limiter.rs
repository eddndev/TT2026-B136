use application::ApplicationError;
use domain::{crypto::DocumentHasher, identity::UserId};
use uuid::Uuid;

use super::{invalid, unavailable, OwnerLoginRateLimit, RedisOwnerLoginRuntime};
use crate::RingSha256Hasher;

impl RedisOwnerLoginRuntime {
    pub(super) fn start_budget(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<(), ApplicationError> {
        if owner.as_uuid().is_nil() || binding.is_nil() {
            return Err(invalid());
        }
        let mut bytes = b"qadra:owner-certificate-login-start-limit:v1\0".to_vec();
        bytes.extend_from_slice(owner.as_uuid().as_bytes());
        bytes.extend_from_slice(binding.as_bytes());
        self.admit(
            "start:global",
            subject_key("start:owner-binding", &bytes),
            self.policy.start_global,
            self.policy.start_owner_binding,
        )
    }

    pub(super) fn proof_budget(&self, token: &str) -> Result<(), ApplicationError> {
        let mut bytes = b"qadra:owner-certificate-login-proof-limit:v1\0".to_vec();
        bytes.extend_from_slice(token.as_bytes());
        self.admit(
            "proof:global",
            subject_key("proof:token", &bytes),
            self.policy.proof_global,
            self.policy.proof_token,
        )
    }

    fn admit(
        &self,
        global: &str,
        subject: String,
        global_limit: OwnerLoginRateLimit,
        subject_limit: OwnerLoginRateLimit,
    ) -> Result<(), ApplicationError> {
        // The fixed-window algorithm is shared; operation keys and policy are not.
        let admitted: i64 =
            redis::Script::new(include_str!("../password_reset_runtime/budget.lua"))
                .key(format!("identity:certificate-login-rate:v1:{global}"))
                .key(subject)
                .arg(global_limit.maximum)
                .arg(global_limit.window_ms)
                .arg(subject_limit.maximum)
                .arg(subject_limit.window_ms)
                .invoke(&mut self.connection()?)
                .map_err(|_| unavailable())?;
        match admitted {
            1 => Ok(()),
            0 => Err(ApplicationError::AccountLocked),
            _ => Err(unavailable()),
        }
    }
}

fn subject_key(kind: &str, bytes: &[u8]) -> String {
    format!(
        "identity:certificate-login-rate:v1:{kind}:{}",
        RingSha256Hasher.hash_bytes(bytes).to_hex()
    )
}
