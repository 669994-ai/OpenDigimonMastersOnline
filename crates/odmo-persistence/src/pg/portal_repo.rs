use odmo_application::game::{PortalDefinition, PortalRepository};

use super::PgRepository;
use crate::get_portal_definitions;

impl PortalRepository for PgRepository {
    fn portal_by_id(&self, portal_id: i32) -> anyhow::Result<Option<PortalDefinition>> {
        // For now, use the same hardcoded portal definitions as JsonRepository.
        // In a production setup, these would come from a `portals` table loaded at startup.
        let portals = get_portal_definitions();
        Ok(portals.into_iter().find(|p| p.id == portal_id))
    }
}
