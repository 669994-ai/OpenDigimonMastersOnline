use odmo_application::game::{PortalDefinition, PortalRepository};

use super::PgRepository;
use crate::get_portal_definitions;

impl PortalRepository for PgRepository {
    fn portal_by_id(&self, portal_id: i32) -> anyhow::Result<Option<PortalDefinition>> {
        // Reuse the workspace-owned portal catalog until a dedicated PostgreSQL
        // table is introduced for these definitions.
        let portals = get_portal_definitions();
        Ok(portals.into_iter().find(|p| p.id == portal_id))
    }
}
