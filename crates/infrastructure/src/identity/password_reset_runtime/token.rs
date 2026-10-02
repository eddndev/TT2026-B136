use application::identity::password_reset::ResetTokenSource;
use application::ApplicationError;
use zeroize::Zeroizing;

/// Fresh operating-system entropy; token buffers remain owned and zeroizing.
pub struct RandomResetTokenSource;

impl ResetTokenSource for RandomResetTokenSource {
    fn generate(&self) -> Result<Zeroizing<[u8; 32]>, ApplicationError> {
        generate_with(getrandom::getrandom)
    }
}

fn generate_with(
    fill: impl FnOnce(&mut [u8]) -> Result<(), getrandom::Error>,
) -> Result<Zeroizing<[u8; 32]>, ApplicationError> {
    let mut bytes = Zeroizing::new([0; 32]);
    fill(bytes.as_mut())
        .map_err(|_| ApplicationError::Port("password reset entropy unavailable".into()))?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "token_tests.rs"]
mod tests;
