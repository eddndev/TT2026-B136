#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[path = "precautionary_context_read_support/mod.rs"]
mod read_support;

use read_support::*;

#[test]
fn current_context_read_returns_exact_complete_material_and_pctx1_digest() {
    let mut material = context_support::changed(context_support::trial());
    context_support::observed_newer(&mut material);
    let context = PrecautionaryContext::new(&Hasher, material).unwrap();
    let actor = reader(Role::Paralegal);
    let store = returning(&actor, context.material().case_id, context.clone());
    let returned = service(store, identity(&actor), clock())
        .get("session", context.material().case_id)
        .unwrap();
    assert_eq!(returned, context);
    assert_eq!(returned.canonical_bytes(), context.canonical_bytes());
    assert_eq!(returned.digest(&Hasher), context.digest(&Hasher));
    assert_ne!(actor.id, returned.material().administration.changed_by.id);
}
