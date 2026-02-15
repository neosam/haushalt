use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_ws::Message;
use futures::StreamExt;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::Config;
use crate::services::{chat as chat_service, households as household_service, websocket::WsManager};
use shared::{WsClientMessage, WsServerMessage};

/// Configure the WebSocket route
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/ws", web::get().to(ws_handler));
}

/// WebSocket connection handler
async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    ws_manager: web::Data<Arc<WsManager>>,
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
) -> Result<HttpResponse> {
    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let session_id = Uuid::new_v4();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsServerMessage>();

    // Register session
    ws_manager.register(session_id, tx).await;

    let ws_manager_clone = ws_manager.clone();
    let pool_clone = pool.clone();
    let config_clone = config.clone();

    // Spawn task to handle incoming messages
    actix_rt::spawn(async move {
        let mut session = session;

        // Spawn task to send outgoing messages
        let mut session_clone = session.clone();
        let send_task = actix_rt::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if session_clone.text(json).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Handle incoming messages
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                        handle_client_message(
                            &session_id,
                            client_msg,
                            &ws_manager_clone,
                            &pool_clone,
                            &config_clone,
                        )
                        .await;
                    } else {
                        ws_manager_clone
                            .send_to_session(
                                &session_id,
                                WsServerMessage::Error {
                                    code: "invalid_message".to_string(),
                                    message: "Failed to parse message".to_string(),
                                },
                            )
                            .await;
                    }
                }
                Message::Ping(bytes) => {
                    let _ = session.pong(&bytes).await;
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }

        // Cleanup
        ws_manager_clone.disconnect(&session_id).await;
        send_task.abort();
        let _ = session.close(None).await;
    });

    Ok(response)
}

/// Handle incoming WebSocket messages from clients
async fn handle_client_message(
    session_id: &Uuid,
    message: WsClientMessage,
    ws_manager: &Arc<WsManager>,
    pool: &SqlitePool,
    config: &Config,
) {
    match message {
        WsClientMessage::Authenticate { token } => {
            // Validate JWT token
            match crate::middleware::auth::validate_token(&token, &config.jwt_secret) {
                Ok(claims) => {
                    // Get user info
                    let user_id = claims.sub;
                    match crate::services::auth::get_user_by_id(pool, &user_id).await {
                        Ok(Some(user)) => {
                            ws_manager
                                .authenticate(session_id, user_id, user.username)
                                .await;
                        }
                        _ => {
                            ws_manager
                                .send_to_session(
                                    session_id,
                                    WsServerMessage::Error {
                                        code: "auth_failed".to_string(),
                                        message: "User not found".to_string(),
                                    },
                                )
                                .await;
                        }
                    }
                }
                Err(_) => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "auth_failed".to_string(),
                                message: "Invalid token".to_string(),
                            },
                        )
                        .await;
                }
            }
        }

        WsClientMessage::JoinRoom { household_id } => {
            // Check if authenticated
            let user_info = ws_manager.get_session_user(session_id).await;
            if user_info.is_none() {
                ws_manager
                    .send_to_session(
                        session_id,
                        WsServerMessage::Error {
                            code: "not_authenticated".to_string(),
                            message: "You must authenticate first".to_string(),
                        },
                    )
                    .await;
                return;
            }

            let (user_id, _) = user_info.unwrap();

            // Check membership
            if !household_service::is_member(pool, &household_id, &user_id)
                .await
                .unwrap_or(false)
            {
                ws_manager
                    .send_to_session(
                        session_id,
                        WsServerMessage::Error {
                            code: "forbidden".to_string(),
                            message: "You are not a member of this household".to_string(),
                        },
                    )
                    .await;
                return;
            }

            ws_manager.join_room(session_id, household_id).await;
        }

        WsClientMessage::LeaveRoom => {
            ws_manager.leave_room(session_id).await;
        }

        WsClientMessage::SendMessage { content } => {
            // Check if in a room
            let household_id = match ws_manager.get_session_household(session_id).await {
                Some(id) => id,
                None => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "not_in_room".to_string(),
                                message: "You must join a room first".to_string(),
                            },
                        )
                        .await;
                    return;
                }
            };

            let user_id = match ws_manager.get_session_user(session_id).await {
                Some((uid, _)) => uid,
                None => return,
            };

            // Create message
            match chat_service::create_message(pool, &household_id, &user_id, &content).await {
                Ok(message) => {
                    // Get message with user info
                    if let Ok(Some(msg_with_user)) =
                        chat_service::get_message_with_user(pool, &message.id).await
                    {
                        ws_manager
                            .broadcast_new_message(&household_id, msg_with_user)
                            .await;
                    }
                }
                Err(chat_service::ChatError::EmptyContent) => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "empty_content".to_string(),
                                message: "Message cannot be empty".to_string(),
                            },
                        )
                        .await;
                }
                Err(e) => {
                    log::error!("Error creating message via WebSocket: {:?}", e);
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "send_failed".to_string(),
                                message: "Failed to send message".to_string(),
                            },
                        )
                        .await;
                }
            }
        }

        WsClientMessage::EditMessage {
            message_id,
            content,
        } => {
            let household_id = match ws_manager.get_session_household(session_id).await {
                Some(id) => id,
                None => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "not_in_room".to_string(),
                                message: "You must join a room first".to_string(),
                            },
                        )
                        .await;
                    return;
                }
            };

            let user_id = match ws_manager.get_session_user(session_id).await {
                Some((uid, _)) => uid,
                None => return,
            };

            match chat_service::update_message(pool, &message_id, &user_id, &content).await {
                Ok(_) => {
                    if let Ok(Some(msg_with_user)) =
                        chat_service::get_message_with_user(pool, &message_id).await
                    {
                        ws_manager
                            .broadcast_message_edited(&household_id, msg_with_user)
                            .await;
                    }
                }
                Err(chat_service::ChatError::NotAuthorized) => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "forbidden".to_string(),
                                message: "You can only edit your own messages".to_string(),
                            },
                        )
                        .await;
                }
                Err(chat_service::ChatError::NotFound) => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "not_found".to_string(),
                                message: "Message not found".to_string(),
                            },
                        )
                        .await;
                }
                Err(e) => {
                    log::error!("Error editing message via WebSocket: {:?}", e);
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "edit_failed".to_string(),
                                message: "Failed to edit message".to_string(),
                            },
                        )
                        .await;
                }
            }
        }

        WsClientMessage::DeleteMessage { message_id } => {
            let household_id = match ws_manager.get_session_household(session_id).await {
                Some(id) => id,
                None => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "not_in_room".to_string(),
                                message: "You must join a room first".to_string(),
                            },
                        )
                        .await;
                    return;
                }
            };

            let user_id = match ws_manager.get_session_user(session_id).await {
                Some((uid, _)) => uid,
                None => return,
            };

            match chat_service::delete_message(pool, &message_id, &user_id).await {
                Ok(()) => {
                    ws_manager
                        .broadcast_message_deleted(&household_id, message_id)
                        .await;
                }
                Err(chat_service::ChatError::NotAuthorized) => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "forbidden".to_string(),
                                message: "You can only delete your own messages".to_string(),
                            },
                        )
                        .await;
                }
                Err(chat_service::ChatError::NotFound) => {
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "not_found".to_string(),
                                message: "Message not found".to_string(),
                            },
                        )
                        .await;
                }
                Err(e) => {
                    log::error!("Error deleting message via WebSocket: {:?}", e);
                    ws_manager
                        .send_to_session(
                            session_id,
                            WsServerMessage::Error {
                                code: "delete_failed".to_string(),
                                message: "Failed to delete message".to_string(),
                            },
                        )
                        .await;
                }
            }
        }

        WsClientMessage::Ping => {
            ws_manager
                .send_to_session(session_id, WsServerMessage::Pong)
                .await;
        }
    }
}
