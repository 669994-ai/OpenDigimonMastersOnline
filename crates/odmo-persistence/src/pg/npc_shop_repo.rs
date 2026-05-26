use odmo_application::game::{NpcShopDefinition, NpcShopRepository};

use super::PgRepository;
use crate::get_npc_shops;

impl NpcShopRepository for PgRepository {
    fn shop_by_npc(&self, npc_id: i32, map_id: i16) -> anyhow::Result<Option<NpcShopDefinition>> {
        // For now, use the same hardcoded NPC shop definitions as JsonRepository.
        // In a production setup, these would come from an `npc_shops` table loaded at startup.
        let shops = get_npc_shops();
        Ok(shops
            .into_iter()
            .find(|s| s.npc_id == npc_id && s.map_id == map_id))
    }
}
