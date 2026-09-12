//! Administrative schema setup and offline legacy import composition.

use crate::cli::DatabaseAction;
use anyhow::Context;
use infrastructure::{initialize_database, LegacyImport};

pub fn run(action: DatabaseAction, json: bool) -> anyhow::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL must name the target database")?;
    match action {
        DatabaseAction::Migrate { runtime_role } => {
            initialize_database(&database_url, &runtime_role)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({"migrated":true,"runtime_role":runtime_role})
                );
            } else {
                println!("database schema and runtime privileges prepared");
            }
        }
        DatabaseAction::Import {
            data_dir,
            mapping,
            apply,
        } => {
            let kek = crate::vault_cmd::load_kek("KEK_BASE64")?;
            let import = LegacyImport::inspect(&data_dir, &mapping, &kek)?;
            import.check_target(&database_url)?;
            if apply {
                import.apply(&database_url)?;
            }
            if json {
                println!(
                    "{}",
                    serde_json::json!({"applied":apply,"report":import.report()})
                );
            } else {
                println!(
                    "validated {} documents and {} historical audit entries; applied={apply}",
                    import.report().documents,
                    import.report().audit_entries
                );
                println!("snapshot fingerprint: {}", import.report().fingerprint);
            }
        }
    }
    Ok(())
}
