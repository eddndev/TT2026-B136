use super::{authorization::authorize, port, sources, PostgresPrecautionaryHearingStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    identity::Principal,
    precautionary_hearings::{PrecautionaryContext, PrecautionaryContextReadStore},
    ApplicationError,
};
use domain::cases::CaseId;

impl PrecautionaryContextReadStore for PostgresPrecautionaryHearingStore {
    fn get(
        &self,
        actor: &Principal,
        case: CaseId,
    ) -> Result<PrecautionaryContext, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        let context = sources::current_observed_context(&mut tx, case, self.hasher.as_ref())?;
        let material = context.material();
        let floor = material
            .administration
            .changed_at
            .max(material.stage.recorded_at())
            .max(material.stage_administration.changed_at);
        let resource = format!(
            "case:{case}:administration:{}:stage:{}:context:{}",
            material.administration.revision.get(),
            material.stage.stage_revision().get(),
            context.digest(self.hasher.as_ref()).to_hex(),
        );
        append_transaction(
            &mut tx,
            &actor.email,
            "precautionary_context.read",
            &resource,
            self.now_after(floor)?,
        )?;
        tx.commit().map_err(port)?;
        Ok(context)
    }
}
