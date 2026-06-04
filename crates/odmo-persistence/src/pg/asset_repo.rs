use odmo_application::game::{EvolutionAssetRepository, ItemAssetRepository};
use odmo_types::{EvolutionAsset, ItemAsset};

use super::PgRepository;

impl PgRepository {
    fn load_catalog_rows(&self, query: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        self.block_on(async {
            sqlx::query_scalar::<_, serde_json::Value>(query)
                .fetch_all(&self.pool)
                .await
        })
        .map_err(Into::into)
    }
}

impl EvolutionAssetRepository for PgRepository {
    fn evolution_assets(&self) -> anyhow::Result<Vec<EvolutionAsset>> {
        let rows =
            self.load_catalog_rows("SELECT payload FROM evolution_assets ORDER BY base_type")?;
        rows.into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

impl ItemAssetRepository for PgRepository {
    fn item_assets(&self) -> anyhow::Result<Vec<ItemAsset>> {
        let rows = self.load_catalog_rows("SELECT payload FROM item_assets ORDER BY item_id")?;
        rows.into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}
