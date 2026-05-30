use std::collections::HashMap;

use odmo_application::game::ExtraEvolutionRepository;
use odmo_types::{ExtraEvolutionMaterial, ExtraEvolutionNpc, ExtraEvolutionRecipe};

use super::PgRepository;

impl ExtraEvolutionRepository for PgRepository {
    fn extra_evolution_npcs(&self) -> anyhow::Result<Vec<ExtraEvolutionNpc>> {
        let pool = self.pool().clone();
        self.block_on(async move {
            let npc_rows = sqlx::query_as::<_, (i64, i32)>(
                "SELECT id, npc_id FROM extra_evolution_npcs ORDER BY npc_id",
            )
            .fetch_all(&pool)
            .await?;
            if npc_rows.is_empty() {
                return Ok(Vec::new());
            }

            let recipe_rows = sqlx::query_as::<_, (i64, i64, i16, i32, i16, i32, i64, i16)>(
                "SELECT id, npc_row_id, exchange_type, object_id, material_type, need_material_value, price, way_type \
                 FROM extra_evolution_recipes ORDER BY npc_row_id, id",
            )
            .fetch_all(&pool)
            .await?;

            let material_rows = sqlx::query_as::<_, (i64, i16, i32, i32)>(
                "SELECT recipe_row_id, material_scope, material_id, amount \
                 FROM extra_evolution_materials ORDER BY recipe_row_id, id",
            )
            .fetch_all(&pool)
            .await?;

            let mut materials_by_recipe: HashMap<i64, (Vec<ExtraEvolutionMaterial>, Vec<ExtraEvolutionMaterial>)> =
                HashMap::new();
            for (recipe_row_id, material_scope, material_id, amount) in material_rows {
                let entry = materials_by_recipe
                    .entry(recipe_row_id)
                    .or_insert_with(|| (Vec::new(), Vec::new()));
                let material = ExtraEvolutionMaterial {
                    material_id,
                    amount,
                };
                if material_scope == 1 {
                    entry.0.push(material);
                } else {
                    entry.1.push(material);
                }
            }

            let mut recipes_by_npc: HashMap<i64, Vec<ExtraEvolutionRecipe>> = HashMap::new();
            for (
                recipe_row_id,
                npc_row_id,
                exchange_type,
                object_id,
                material_type,
                need_material_value,
                price,
                way_type,
            ) in recipe_rows
            {
                let (main_materials, sub_materials) = materials_by_recipe
                    .remove(&recipe_row_id)
                    .unwrap_or_else(|| (Vec::new(), Vec::new()));
                recipes_by_npc
                    .entry(npc_row_id)
                    .or_default()
                    .push(ExtraEvolutionRecipe {
                        exchange_type: exchange_type as u16,
                        object_id,
                        material_type: material_type as u16,
                        need_material_value,
                        price,
                        way_type: way_type as u16,
                        main_materials,
                        sub_materials,
                    });
            }

            Ok(npc_rows
                .into_iter()
                .map(|(npc_row_id, npc_id)| ExtraEvolutionNpc {
                    npc_id,
                    recipes: recipes_by_npc.remove(&npc_row_id).unwrap_or_default(),
                })
                .collect())
        })
    }
}
