use std::collections::HashMap;

use odmo_application::game::{DigiCombineRepository, UnionCombineRepository};
use odmo_types::{
    CombineCeilingEntry, DigiCombineCatalog, DigiCombineCeil, DigiCombineGroup, DigiCombineItem,
    DigiCombineRank, DigiCombineReward, UnionCombineCatalog,
};

use super::PgRepository;

/// Variant discriminator shared by Digi and Union combine catalogs.
const VARIANT_DIGI: i16 = 0;
const VARIANT_UNION: i16 = 1;

impl PgRepository {
    /// Load one combine catalog by its variant discriminator, reassembling
    /// child rows into their parent rows via id-keyed maps.
    fn combine_catalog(&self, variant: i16) -> anyhow::Result<DigiCombineCatalog> {
        // Rank rows and their reward pools.
        let rank_rows = self.block_on(async {
            sqlx::query_as::<_, (i64, i16, i64)>(
                "SELECT id, ceiling_type, weight FROM combine_ranks WHERE variant = $1 ORDER BY id",
            )
            .bind(variant)
            .fetch_all(&self.pool)
            .await
        })?;

        let rank_rewards = self.block_on(async {
            sqlx::query_as::<_, (i64, i32, i32, i16)>(
                "SELECT r.rank_row_id, r.item_id, r.amount, r.grade \
                 FROM combine_rank_rewards r \
                 JOIN combine_ranks k ON k.id = r.rank_row_id \
                 WHERE k.variant = $1 ORDER BY r.id",
            )
            .bind(variant)
            .fetch_all(&self.pool)
            .await
        })?;

        let mut rewards_by_rank: HashMap<i64, Vec<DigiCombineReward>> = HashMap::new();
        for (rank_row_id, item_id, amount, grade) in rank_rewards {
            rewards_by_rank
                .entry(rank_row_id)
                .or_default()
                .push(DigiCombineReward {
                    item_id,
                    amount: amount as u16,
                    grade: grade as u8,
                });
        }

        let rank_rows = rank_rows
            .into_iter()
            .map(|(row_id, ceiling_type, weight)| DigiCombineRank {
                ceiling_type: ceiling_type as u8,
                weight: weight as u32,
                rewards: rewards_by_rank.remove(&row_id).unwrap_or_default(),
            })
            .collect();

        // Allowed combine materials.
        let item_list = self.block_on(async {
            sqlx::query_as::<_, (i32, i32)>(
                "SELECT item_id, group_id FROM combine_items WHERE variant = $1 ORDER BY id",
            )
            .bind(variant)
            .fetch_all(&self.pool)
            .await
        })?;

        let item_list = item_list
            .into_iter()
            .map(|(item_id, group_id)| DigiCombineItem { item_id, group_id })
            .collect();

        // Material groups and their members.
        let group_rows = self.block_on(async {
            sqlx::query_as::<_, (i64, i32)>(
                "SELECT id, group_id FROM combine_groups WHERE variant = $1 ORDER BY id",
            )
            .bind(variant)
            .fetch_all(&self.pool)
            .await
        })?;

        let group_members = self.block_on(async {
            sqlx::query_as::<_, (i64, i32)>(
                "SELECT m.group_row_id, m.member_id \
                 FROM combine_group_members m \
                 JOIN combine_groups g ON g.id = m.group_row_id \
                 WHERE g.variant = $1 ORDER BY m.id",
            )
            .bind(variant)
            .fetch_all(&self.pool)
            .await
        })?;

        let mut members_by_group: HashMap<i64, Vec<i32>> = HashMap::new();
        for (group_row_id, member_id) in group_members {
            members_by_group
                .entry(group_row_id)
                .or_default()
                .push(member_id);
        }

        let item_groups = group_rows
            .into_iter()
            .map(|(row_id, group_id)| DigiCombineGroup {
                group_id,
                members: members_by_group.remove(&row_id).unwrap_or_default(),
            })
            .collect();

        // Ceiling groups and their tier entries.
        let ceil_rows = self.block_on(async {
            sqlx::query_as::<_, (i64, i16)>(
                "SELECT id, ceiling_type FROM combine_ceils WHERE variant = $1 ORDER BY id",
            )
            .bind(variant)
            .fetch_all(&self.pool)
            .await
        })?;

        let ceil_entries = self.block_on(async {
            sqlx::query_as::<_, (i64, i16, i16, i32)>(
                "SELECT e.ceil_row_id, e.tier, e.value_a, e.value_b \
                 FROM combine_ceil_entries e \
                 JOIN combine_ceils c ON c.id = e.ceil_row_id \
                 WHERE c.variant = $1 ORDER BY e.id",
            )
            .bind(variant)
            .fetch_all(&self.pool)
            .await
        })?;

        let mut entries_by_ceil: HashMap<i64, Vec<CombineCeilingEntry>> = HashMap::new();
        for (ceil_row_id, tier, value_a, value_b) in ceil_entries {
            entries_by_ceil
                .entry(ceil_row_id)
                .or_default()
                .push(CombineCeilingEntry {
                    tier: tier as u8,
                    value_a: value_a as u8,
                    value_b: value_b as u16,
                });
        }

        let ceil_groups = ceil_rows
            .into_iter()
            .map(|(row_id, ceiling_type)| DigiCombineCeil {
                ceiling_type: ceiling_type as u8,
                entries: entries_by_ceil.remove(&row_id).unwrap_or_default(),
            })
            .collect();

        Ok(DigiCombineCatalog {
            rank_rows,
            item_list,
            item_groups,
            ceil_groups,
        })
    }
}

impl DigiCombineRepository for PgRepository {
    fn digi_combine_catalog(&self) -> anyhow::Result<DigiCombineCatalog> {
        self.combine_catalog(VARIANT_DIGI)
    }
}

impl UnionCombineRepository for PgRepository {
    fn union_combine_catalog(&self) -> anyhow::Result<UnionCombineCatalog> {
        self.combine_catalog(VARIANT_UNION)
    }
}
