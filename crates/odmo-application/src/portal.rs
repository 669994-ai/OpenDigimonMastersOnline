use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use odmo_types::{AccountId, CharacterId, CharacterSummary, GameSessionTicket, TransferTicket};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct PortalBridge {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialNotification {
    pub kind: SocialNotificationKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SocialNotificationKind {
    FriendConnect { name: String },
    MapTamerSpawn { character: CharacterSummary },
    MapTamerUnload { character: CharacterSummary },
}

impl PortalBridge {
    pub fn new(root: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&root).with_context(|| {
            format!(
                "failed to create portal bridge directory {}",
                root.display()
            )
        })?;
        Ok(Self { root })
    }

    pub fn store_transfer_ticket(&self, ticket: &TransferTicket) -> anyhow::Result<()> {
        let path = self.ticket_path(ticket.account_id);
        let payload = serde_json::to_vec_pretty(ticket)?;
        fs::write(&path, payload)
            .with_context(|| format!("failed to write transfer ticket {}", path.display()))?;
        Ok(())
    }

    pub fn load_transfer_ticket(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<Option<TransferTicket>> {
        let path = self.ticket_path(account_id);
        if !path.exists() {
            return Ok(None);
        }

        let payload = fs::read(&path)
            .with_context(|| format!("failed to read transfer ticket {}", path.display()))?;
        let ticket = serde_json::from_slice(&payload)
            .with_context(|| format!("failed to decode transfer ticket {}", path.display()))?;
        Ok(Some(ticket))
    }

    pub fn consume_transfer_ticket(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<Option<TransferTicket>> {
        let path = self.ticket_path(account_id);
        let ticket = self.load_transfer_ticket(account_id)?;
        if ticket.is_some() && path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove consumed ticket {}", path.display()))?;
        }
        Ok(ticket)
    }

    pub fn store_game_session_ticket(&self, ticket: &GameSessionTicket) -> anyhow::Result<()> {
        let path = self.game_ticket_path(ticket.account_id);
        let payload = serde_json::to_vec_pretty(ticket)?;
        fs::write(&path, payload)
            .with_context(|| format!("failed to write game session ticket {}", path.display()))?;
        Ok(())
    }

    pub fn consume_game_session_ticket(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<Option<GameSessionTicket>> {
        let path = self.game_ticket_path(account_id);
        if !path.exists() {
            return Ok(None);
        }

        let payload = fs::read(&path)
            .with_context(|| format!("failed to read game session ticket {}", path.display()))?;
        let ticket = serde_json::from_slice(&payload)
            .with_context(|| format!("failed to decode game session ticket {}", path.display()))?;
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove consumed game ticket {}", path.display()))?;
        Ok(Some(ticket))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn enqueue_social_notification(
        &self,
        character_id: CharacterId,
        notification: SocialNotification,
    ) -> anyhow::Result<()> {
        let path = self.social_inbox_path(character_id);
        let mut notifications = self.load_social_notifications(character_id)?;
        notifications.push(notification);
        let payload = serde_json::to_vec_pretty(&notifications)?;
        fs::write(&path, payload)
            .with_context(|| format!("failed to write social inbox {}", path.display()))?;
        Ok(())
    }

    pub fn consume_social_notifications(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<SocialNotification>> {
        let path = self.social_inbox_path(character_id);
        let notifications = self.load_social_notifications(character_id)?;
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove social inbox {}", path.display()))?;
        }
        Ok(notifications)
    }

    pub fn load_map_presence(
        &self,
        map_id: i16,
        channel: u8,
    ) -> anyhow::Result<Vec<CharacterSummary>> {
        let path = self.map_presence_path(map_id, channel);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let payload = fs::read(&path)
            .with_context(|| format!("failed to read map presence {}", path.display()))?;
        let entries = serde_json::from_slice(&payload)
            .with_context(|| format!("failed to decode map presence {}", path.display()))?;
        Ok(entries)
    }

    pub fn upsert_map_presence(&self, character: &CharacterSummary) -> anyhow::Result<()> {
        let path = self.map_presence_path(character.map_id, character.channel);
        let mut entries = self.load_map_presence(character.map_id, character.channel)?;
        entries.retain(|entry| entry.id != character.id);
        entries.push(character.clone());
        let payload = serde_json::to_vec_pretty(&entries)?;
        fs::write(&path, payload)
            .with_context(|| format!("failed to write map presence {}", path.display()))?;
        Ok(())
    }

    pub fn remove_map_presence(
        &self,
        map_id: i16,
        channel: u8,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<CharacterSummary>> {
        let path = self.map_presence_path(map_id, channel);
        let mut entries = self.load_map_presence(map_id, channel)?;
        let before_len = entries.len();
        entries.retain(|entry| entry.id != character_id);

        if entries.is_empty() {
            if path.exists() {
                fs::remove_file(&path).with_context(|| {
                    format!("failed to remove empty map presence {}", path.display())
                })?;
            }
        } else if entries.len() != before_len {
            let payload = serde_json::to_vec_pretty(&entries)?;
            fs::write(&path, payload)
                .with_context(|| format!("failed to update map presence {}", path.display()))?;
        }

        Ok(entries)
    }

    fn ticket_path(&self, account_id: AccountId) -> PathBuf {
        self.root.join(format!("account-{account_id}.json"))
    }

    fn game_ticket_path(&self, account_id: AccountId) -> PathBuf {
        self.root.join(format!("game-account-{account_id}.json"))
    }

    fn social_inbox_path(&self, character_id: CharacterId) -> PathBuf {
        self.root
            .join(format!("social-character-{character_id}.json"))
    }

    fn map_presence_path(&self, map_id: i16, channel: u8) -> PathBuf {
        self.root
            .join(format!("map-{map_id}-channel-{channel}.json"))
    }

    fn load_social_notifications(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<SocialNotification>> {
        let path = self.social_inbox_path(character_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let payload = fs::read(&path)
            .with_context(|| format!("failed to read social inbox {}", path.display()))?;
        let notifications = serde_json::from_slice(&payload)
            .with_context(|| format!("failed to decode social inbox {}", path.display()))?;
        Ok(notifications)
    }
}
