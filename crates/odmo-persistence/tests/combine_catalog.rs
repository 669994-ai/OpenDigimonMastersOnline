//! Persistence boundary tests for the combine catalogs and random-box pool.
//!
//! These assert that the JSON backend returns the seeded `odmo_types` catalog
//! values, and that the Postgres backend implements the same persistence-port
//! traits. The trait signatures return only `odmo_types` types, so no protocol
//! type can cross the persistence boundary.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use odmo_application::game::{DigiCombineRepository, RandomBoxRepository, UnionCombineRepository};
use odmo_persistence::{DEMO_CATALOG_ITEM_A, DEMO_CATALOG_ITEM_B, JsonRepository};
use odmo_persistence::pg::PgRepository;
use odmo_types::{DigiCombineCatalog, RandomBoxReward, UnionCombineCatalog};

/// Build a `JsonRepository` over a unique temp path so each test seeds a fresh
/// demo world without colliding with sibling tests.
fn fresh_json_repository() -> JsonRepository {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push("odmo-combine-tests");
    path.push(format!("world-{nanos}-{unique}.json"));
    JsonRepository::open_or_create(path).expect("seed JSON repository")
}

#[test]
fn json_digi_combine_catalog_returns_seeded_catalog() {
    let repo = fresh_json_repository();

    // Binding to the explicit `odmo_types` type proves the return value never
    // carries a protocol type across the persistence boundary.
    let catalog: DigiCombineCatalog = repo
        .digi_combine_catalog()
        .expect("digi combine catalog available");

    assert_eq!(catalog.rank_rows.len(), 2, "two seeded rank rows");
    assert_eq!(catalog.rank_rows[0].ceiling_type, 1);
    assert_eq!(catalog.rank_rows[0].weight, 80);
    assert_eq!(catalog.rank_rows[0].rewards.len(), 1);
    assert_eq!(catalog.rank_rows[0].rewards[0].item_id, 5101);
    assert_eq!(catalog.rank_rows[0].rewards[0].grade, 1);
    assert_eq!(catalog.rank_rows[1].weight, 20);
    assert_eq!(catalog.rank_rows[1].rewards[0].item_id, 5102);

    assert_eq!(catalog.item_list.len(), 2);
    assert_eq!(catalog.item_list[0].item_id, DEMO_CATALOG_ITEM_A);
    assert_eq!(catalog.item_list[0].group_id, 1);

    assert_eq!(catalog.item_groups.len(), 1);
    assert_eq!(catalog.item_groups[0].group_id, 1);
    assert_eq!(
        catalog.item_groups[0].members,
        vec![DEMO_CATALOG_ITEM_A, DEMO_CATALOG_ITEM_B]
    );

    assert_eq!(catalog.ceil_groups.len(), 1);
    assert_eq!(catalog.ceil_groups[0].ceiling_type, 1);
    assert_eq!(catalog.ceil_groups[0].entries.len(), 1);
    assert_eq!(catalog.ceil_groups[0].entries[0].value_b, 100);
}

#[test]
fn json_union_combine_catalog_returns_seeded_catalog() {
    let repo = fresh_json_repository();

    // `UnionCombineCatalog` aliases `DigiCombineCatalog`; bind it explicitly to
    // keep the boundary type visible at the call site.
    let catalog: UnionCombineCatalog = repo
        .union_combine_catalog()
        .expect("union combine catalog available");

    assert_eq!(catalog.rank_rows.len(), 2, "two seeded rank rows");
    assert_eq!(catalog.item_list.len(), 2);
    assert_eq!(catalog.item_groups.len(), 1);
    assert_eq!(catalog.ceil_groups.len(), 1);
    assert_eq!(catalog.ceil_groups[0].entries[0].value_b, 100);
}

#[test]
fn json_random_box_rewards_returns_seeded_pool() {
    let repo = fresh_json_repository();

    let rewards: Vec<RandomBoxReward> = repo
        .random_box_rewards()
        .expect("random box pool available");

    assert_eq!(rewards.len(), 3, "three seeded weighted rewards");
    assert_eq!(rewards[0].item_id, 5201);
    assert_eq!(rewards[0].amount, 1);
    assert_eq!(rewards[0].weight, 70);
    assert_eq!(rewards[1].item_id, 5202);
    assert_eq!(rewards[1].weight, 25);
    assert_eq!(rewards[2].item_id, 5203);
    assert_eq!(rewards[2].amount, 2);
    assert_eq!(rewards[2].weight, 5);

    // Weights form a non-degenerate distribution.
    let total: u32 = rewards.iter().map(|r| r.weight).sum();
    assert_eq!(total, 100);
}

/// Compile-time proof that the Postgres backend implements the same three
/// persistence-port traits as the JSON backend. Because every trait method
/// returns an `odmo_types` catalog/reward type, no protocol type can leak
/// across the persistence boundary. No database connection is required.
const _: fn() = || {
    fn assert_impl<T: DigiCombineRepository + UnionCombineRepository + RandomBoxRepository>() {}
    assert_impl::<PgRepository>();
    assert_impl::<JsonRepository>();
};
