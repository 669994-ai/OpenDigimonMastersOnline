use std::collections::HashMap;

use odmo_application::game::DigiSummonRepository;
use odmo_types::{DigiSummonProduct, DigiSummonReward, DigiSummonTicket};

use super::PgRepository;

impl DigiSummonRepository for PgRepository {
    fn digi_summon_products(&self) -> anyhow::Result<Vec<DigiSummonProduct>> {
        let rows = self.block_on(async {
            sqlx::query_as::<_, (i64, i32, i32, i32, i32, i32, String, String, String)>(
                "SELECT id, product_id, string_id, draw_count, rank, remaining_daily_limit, icon, name, description FROM digi_summon_products ORDER BY product_id",
            )
            .fetch_all(&self.pool)
            .await
        })?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let tickets = self.block_on(async {
            sqlx::query_as::<_, (i64, i32, i32)>(
                "SELECT product_row_id, item_id, cost FROM digi_summon_tickets ORDER BY id",
            )
            .fetch_all(&self.pool)
            .await
        })?;

        let rewards = self.block_on(async {
            sqlx::query_as::<_, (i64, i32, i32, i32, i32, i32, i32, i32)>(
                "SELECT product_row_id, item_list_id, item_id, grade, amount, weight, reward_group, group_code FROM digi_summon_rewards ORDER BY id",
            )
            .fetch_all(&self.pool)
            .await
        })?;

        let mut tickets_by_product: HashMap<i64, Vec<DigiSummonTicket>> = HashMap::new();
        for (product_row_id, item_id, cost) in tickets {
            tickets_by_product
                .entry(product_row_id)
                .or_default()
                .push(DigiSummonTicket { item_id, cost });
        }

        let mut rewards_by_product: HashMap<i64, Vec<DigiSummonReward>> = HashMap::new();
        for (product_row_id, item_list_id, item_id, grade, amount, weight, group, group_code) in
            rewards
        {
            rewards_by_product
                .entry(product_row_id)
                .or_default()
                .push(DigiSummonReward {
                    item_list_id,
                    item_id,
                    grade,
                    amount,
                    weight,
                    group,
                    group_code,
                });
        }

        Ok(rows
            .into_iter()
            .map(
                |(
                    row_id,
                    product_id,
                    string_id,
                    draw_count,
                    rank,
                    remaining_daily_limit,
                    icon,
                    name,
                    description,
                )| DigiSummonProduct {
                    product_id,
                    string_id,
                    draw_count,
                    rank,
                    remaining_daily_limit,
                    icon,
                    name,
                    description,
                    tickets: tickets_by_product.remove(&row_id).unwrap_or_default(),
                    rewards: rewards_by_product.remove(&row_id).unwrap_or_default(),
                },
            )
            .collect())
    }
}
