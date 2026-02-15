use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use shared::{ChatMessageWithUser, WsServerMessage};

/// Sender for WebSocket messages
pub type WsSender = mpsc::UnboundedSender<WsServerMessage>;

/// Client session information
#[derive(Debug)]
pub struct ClientSession {
    pub sender: WsSender,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub household_id: Option<Uuid>,
}

/// WebSocket connection manager
/// Manages all active WebSocket connections and chat rooms
pub struct WsManager {
    /// Map of session_id -> ClientSession
    sessions: RwLock<HashMap<Uuid, ClientSession>>,
    /// Map of household_id -> set of session_ids
    rooms: RwLock<HashMap<Uuid, HashSet<Uuid>>>,
}

impl WsManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sessions: RwLock::new(HashMap::new()),
            rooms: RwLock::new(HashMap::new()),
        })
    }

    /// Register a new WebSocket session
    pub async fn register(&self, session_id: Uuid, sender: WsSender) {
        let session = ClientSession {
            sender,
            user_id: None,
            username: None,
            household_id: None,
        };
        self.sessions.write().await.insert(session_id, session);
        log::debug!("WebSocket session registered: {}", session_id);
    }

    /// Authenticate a session with user information
    pub async fn authenticate(&self, session_id: &Uuid, user_id: Uuid, username: String) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.user_id = Some(user_id);
            session.username = Some(username.clone());

            // Send authenticated response
            let _ = session.sender.send(WsServerMessage::Authenticated {
                user_id,
                username,
            });
            log::debug!("WebSocket session authenticated: {} for user {}", session_id, user_id);
            true
        } else {
            false
        }
    }

    /// Get the user_id for a session
    pub async fn get_session_user(&self, session_id: &Uuid) -> Option<(Uuid, String)> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).and_then(|s| {
            match (&s.user_id, &s.username) {
                (Some(uid), Some(uname)) => Some((*uid, uname.clone())),
                _ => None,
            }
        })
    }

    /// Get the household_id for a session
    pub async fn get_session_household(&self, session_id: &Uuid) -> Option<Uuid> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).and_then(|s| s.household_id)
    }

    /// Join a chat room (household)
    pub async fn join_room(&self, session_id: &Uuid, household_id: Uuid) -> bool {
        // First leave any current room
        self.leave_room(session_id).await;

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if session.user_id.is_none() {
                // Must be authenticated first
                let _ = session.sender.send(WsServerMessage::Error {
                    code: "not_authenticated".to_string(),
                    message: "You must authenticate before joining a room".to_string(),
                });
                return false;
            }

            session.household_id = Some(household_id);
            drop(sessions);

            // Add to room
            let mut rooms = self.rooms.write().await;
            rooms
                .entry(household_id)
                .or_insert_with(HashSet::new)
                .insert(*session_id);

            // Send joined response
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(session_id) {
                let _ = session.sender.send(WsServerMessage::JoinedRoom { household_id });
            }

            log::debug!("Session {} joined room {}", session_id, household_id);
            true
        } else {
            false
        }
    }

    /// Leave the current chat room
    pub async fn leave_room(&self, session_id: &Uuid) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(household_id) = session.household_id.take() {
                drop(sessions);

                // Remove from room
                let mut rooms = self.rooms.write().await;
                if let Some(room) = rooms.get_mut(&household_id) {
                    room.remove(session_id);
                    if room.is_empty() {
                        rooms.remove(&household_id);
                    }
                }

                // Send left response
                let sessions = self.sessions.read().await;
                if let Some(session) = sessions.get(session_id) {
                    let _ = session.sender.send(WsServerMessage::LeftRoom);
                }

                log::debug!("Session {} left room {}", session_id, household_id);
            }
        }
    }

    /// Disconnect a session completely
    pub async fn disconnect(&self, session_id: &Uuid) {
        // Leave any room first
        self.leave_room(session_id).await;

        // Remove session
        self.sessions.write().await.remove(session_id);
        log::debug!("WebSocket session disconnected: {}", session_id);
    }

    /// Send a message to a specific session
    pub async fn send_to_session(&self, session_id: &Uuid, message: WsServerMessage) {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let _ = session.sender.send(message);
        }
    }

    /// Broadcast a message to all sessions in a room (household)
    pub async fn broadcast_to_room(&self, household_id: &Uuid, message: WsServerMessage) {
        let rooms = self.rooms.read().await;
        if let Some(session_ids) = rooms.get(household_id) {
            let sessions = self.sessions.read().await;
            for session_id in session_ids {
                if let Some(session) = sessions.get(session_id) {
                    let _ = session.sender.send(message.clone());
                }
            }
        }
    }

    /// Broadcast a new message to a room
    pub async fn broadcast_new_message(&self, household_id: &Uuid, message: ChatMessageWithUser) {
        self.broadcast_to_room(
            household_id,
            WsServerMessage::NewMessage { message },
        )
        .await;
    }

    /// Broadcast an edited message to a room
    pub async fn broadcast_message_edited(&self, household_id: &Uuid, message: ChatMessageWithUser) {
        self.broadcast_to_room(
            household_id,
            WsServerMessage::MessageEdited { message },
        )
        .await;
    }

    /// Broadcast a deleted message to a room
    pub async fn broadcast_message_deleted(&self, household_id: &Uuid, message_id: Uuid) {
        self.broadcast_to_room(
            household_id,
            WsServerMessage::MessageDeleted {
                message_id,
                household_id: *household_id,
            },
        )
        .await;
    }

    /// Get the number of sessions in a room
    #[allow(dead_code)]
    pub async fn room_size(&self, household_id: &Uuid) -> usize {
        let rooms = self.rooms.read().await;
        rooms.get(household_id).map(|s| s.len()).unwrap_or(0)
    }
}

impl Default for WsManager {
    fn default() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            rooms: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ws_manager_register_and_disconnect() {
        let manager = WsManager::new();
        let session_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::unbounded_channel();

        manager.register(session_id, tx).await;

        // Session should exist
        assert!(manager.sessions.read().await.contains_key(&session_id));

        manager.disconnect(&session_id).await;

        // Session should be removed
        assert!(!manager.sessions.read().await.contains_key(&session_id));
    }

    #[tokio::test]
    async fn test_ws_manager_authenticate() {
        let manager = WsManager::new();
        let session_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::unbounded_channel();

        manager.register(session_id, tx).await;

        let result = manager.authenticate(&session_id, user_id, "testuser".to_string()).await;
        assert!(result);

        let user = manager.get_session_user(&session_id).await;
        assert!(user.is_some());
        let (uid, uname) = user.unwrap();
        assert_eq!(uid, user_id);
        assert_eq!(uname, "testuser");
    }

    #[tokio::test]
    async fn test_ws_manager_join_and_leave_room() {
        let manager = WsManager::new();
        let session_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::unbounded_channel();

        manager.register(session_id, tx).await;
        manager.authenticate(&session_id, user_id, "testuser".to_string()).await;

        // Join room
        let result = manager.join_room(&session_id, household_id).await;
        assert!(result);
        assert_eq!(manager.room_size(&household_id).await, 1);

        // Leave room
        manager.leave_room(&session_id).await;
        assert_eq!(manager.room_size(&household_id).await, 0);
    }
}
