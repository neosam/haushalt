use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Timeout;
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use shared::{WsClientMessage, WsServerMessage};

const TOKEN_KEY: &str = "auth_token";

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticated,
    InRoom,
    Reconnecting,
    Error,
}

/// WebSocket client for chat functionality
#[derive(Clone)]
pub struct WsClient {
    ws: Rc<RefCell<Option<WebSocket>>>,
    state: RwSignal<WsConnectionState>,
    last_message: RwSignal<Option<WsServerMessage>>,
    household_id: RwSignal<Option<Uuid>>,
    reconnect_attempts: Rc<RefCell<u32>>,
    max_reconnect_attempts: u32,
    reconnect_timeout: Rc<RefCell<Option<Timeout>>>,
}

impl WsClient {
    pub fn new() -> Self {
        Self {
            ws: Rc::new(RefCell::new(None)),
            state: create_rw_signal(WsConnectionState::Disconnected),
            last_message: create_rw_signal(None),
            household_id: create_rw_signal(None),
            reconnect_attempts: Rc::new(RefCell::new(0)),
            max_reconnect_attempts: 5,
            reconnect_timeout: Rc::new(RefCell::new(None)),
        }
    }

    /// Get the current connection state
    pub fn state(&self) -> ReadSignal<WsConnectionState> {
        self.state.read_only()
    }

    /// Get the last received server message
    pub fn last_message(&self) -> ReadSignal<Option<WsServerMessage>> {
        self.last_message.read_only()
    }

    /// Connect to the WebSocket server
    pub fn connect(&self) {
        let current_state = self.state.get_untracked();
        if current_state == WsConnectionState::Connecting
            || current_state == WsConnectionState::Connected
            || current_state == WsConnectionState::Authenticated
            || current_state == WsConnectionState::InRoom
            || current_state == WsConnectionState::Reconnecting
        {
            return;
        }

        self.state.set(WsConnectionState::Connecting);

        // Get WebSocket URL from current location
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
        let host = location.host().unwrap_or_else(|_| "localhost:8080".to_string());

        let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
        let ws_url = format!("{}//{}/api/ws", ws_protocol, host);

        let ws = match WebSocket::new(&ws_url) {
            Ok(ws) => ws,
            Err(_) => {
                self.state.set(WsConnectionState::Error);
                return;
            }
        };

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Clone for closures
        let state = self.state;
        let last_message = self.last_message;
        let ws_ref = self.ws.clone();
        let household_id = self.household_id;
        let reconnect_attempts = self.reconnect_attempts.clone();

        // onopen handler
        let state_clone = state;
        let ws_clone = ws.clone();
        let onopen = Closure::wrap(Box::new(move |_| {
            state_clone.set(WsConnectionState::Connected);
            *reconnect_attempts.borrow_mut() = 0;

            // Auto-authenticate if we have a token
            if let Ok(token) = LocalStorage::get::<String>(TOKEN_KEY) {
                let msg = WsClientMessage::Authenticate { token };
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = ws_clone.send_with_str(&json);
                }
            }
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        // onmessage handler
        let state_clone = state;
        let household_id_clone = household_id;
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Some(text) = e.data().as_string() {
                if let Ok(msg) = serde_json::from_str::<WsServerMessage>(&text) {
                    // Update state based on message type
                    match &msg {
                        WsServerMessage::Authenticated { .. } => {
                            state_clone.set(WsConnectionState::Authenticated);
                        }
                        WsServerMessage::JoinedRoom { household_id } => {
                            household_id_clone.set(Some(*household_id));
                            state_clone.set(WsConnectionState::InRoom);
                        }
                        WsServerMessage::LeftRoom => {
                            household_id_clone.set(None);
                            state_clone.set(WsConnectionState::Authenticated);
                        }
                        WsServerMessage::Error { .. } => {
                            // Don't change state on errors, let the handler decide
                        }
                        _ => {}
                    }
                    last_message.set(Some(msg));
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // onerror handler
        let state_clone = state;
        let onerror = Closure::wrap(Box::new(move |_: ErrorEvent| {
            state_clone.set(WsConnectionState::Error);
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        // onclose handler
        let client = self.clone();
        let onclose = Closure::wrap(Box::new(move |_: CloseEvent| {
            client.state.set(WsConnectionState::Disconnected);
            client.household_id.set(None);
            *client.ws.borrow_mut() = None;

            // Try to reconnect
            client.schedule_reconnect();
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        *ws_ref.borrow_mut() = Some(ws);
    }

    /// Schedule a reconnection attempt
    fn schedule_reconnect(&self) {
        let attempts = *self.reconnect_attempts.borrow();
        if attempts >= self.max_reconnect_attempts {
            self.state.set(WsConnectionState::Error);
            return;
        }

        self.state.set(WsConnectionState::Reconnecting);
        *self.reconnect_attempts.borrow_mut() = attempts + 1;

        // Exponential backoff: 1s, 2s, 4s, 8s, 16s
        let delay_ms = 1000 * (1 << attempts);
        let client = self.clone();

        let timeout = Timeout::new(delay_ms, move || {
            client.connect();
        });

        *self.reconnect_timeout.borrow_mut() = Some(timeout);
    }

    /// Send a message to the server
    pub fn send(&self, message: WsClientMessage) {
        if let Some(ws) = self.ws.borrow().as_ref() {
            if let Ok(json) = serde_json::to_string(&message) {
                let _ = ws.send_with_str(&json);
            }
        }
    }

    /// Join a chat room (household)
    pub fn join_room(&self, household_id: Uuid) {
        self.send(WsClientMessage::JoinRoom { household_id });
    }

    /// Leave the current chat room
    pub fn leave_room(&self) {
        self.send(WsClientMessage::LeaveRoom);
    }

    /// Send a chat message
    pub fn send_message(&self, content: String) {
        self.send(WsClientMessage::SendMessage { content });
    }

    /// Edit a chat message
    pub fn edit_message(&self, message_id: Uuid, content: String) {
        self.send(WsClientMessage::EditMessage { message_id, content });
    }

    /// Delete a chat message
    pub fn delete_message(&self, message_id: Uuid) {
        self.send(WsClientMessage::DeleteMessage { message_id });
    }

    /// Disconnect from the server
    pub fn disconnect(&self) {
        // Cancel reconnection timeout
        *self.reconnect_timeout.borrow_mut() = None;
        *self.reconnect_attempts.borrow_mut() = self.max_reconnect_attempts; // Prevent reconnect

        if let Some(ws) = self.ws.borrow().as_ref() {
            let _ = ws.close();
        }
        *self.ws.borrow_mut() = None;
        self.state.set(WsConnectionState::Disconnected);
        self.household_id.set(None);
    }

    /// Check if connected and in a room
    pub fn is_ready(&self) -> bool {
        self.state.get() == WsConnectionState::InRoom
    }
}

impl Default for WsClient {
    fn default() -> Self {
        Self::new()
    }
}
