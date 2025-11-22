//! WebSocket handler for real-time event broadcasting

use crate::{
    auth::Claims,
    state::{AppState, OrbitEvent},
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use futures::{SinkExt, StreamExt};
use tokio::select;

/// WebSocket upgrade handler
///
/// Validates JWT from cookies before upgrading to WebSocket
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    jar: CookieJar,
    job_id: Option<Path<String>>,
) -> Response {
    let job_id = job_id.map(|Path(id)| id);
    // Extract and validate JWT from cookies
    let token = match crate::auth::extract_jwt_from_cookies(&jar) {
        Some(t) => t,
        None => {
            tracing::warn!("WebSocket connection rejected: no auth token");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let claims = match crate::auth::validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            tracing::warn!("WebSocket connection rejected: invalid token");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    if claims.is_expired() {
        tracing::warn!("WebSocket connection rejected: expired token");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    tracing::info!(
        "WebSocket connection established for user: {} (role: {})",
        claims.username,
        claims.role
    );

    // Upgrade to WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, state, claims, job_id))
}

/// Handle WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    claims: Claims,
    job_id_filter: Option<String>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to event broadcast
    let mut event_rx = state.subscribe_events();

    // Send initial connection message
    let welcome = serde_json::json!({
        "type": "connected",
        "user": claims.username,
        "timestamp": chrono::Utc::now().timestamp(),
    });

    if sender
        .send(Message::Text(welcome.to_string()))
        .await
        .is_err()
    {
        return;
    }

    // Main event loop
    loop {
        select! {
            // Receive events from broadcast channel
            event = event_rx.recv() => {
                match event {
                    Ok(orbit_event) => {
                        // Check if user has permission to view this event
                        if !claims.get_role().has_permission(orbit_event.required_role()) {
                            continue;
                        }

                        // Filter by job ID if specified
                        if let Some(ref filter_id) = job_id_filter {
                            if orbit_event.job_id() != filter_id {
                                continue;
                            }
                        }

                        // Serialize and send event
                        match serde_json::to_string(&orbit_event) {
                            Ok(json) => {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    tracing::debug!("Client disconnected");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to serialize event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Event broadcast error: {}", e);
                        break;
                    }
                }
            }

            // Handle incoming messages from client (ping/pong, commands)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("Client closed WebSocket connection");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Handle client commands (future: subscribe to specific jobs, etc.)
                        tracing::debug!("Received message from client: {}", text);
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }

    tracing::info!("WebSocket connection closed for user: {}", claims.username);
}

/// Broadcast event to all WebSocket subscribers
///
/// This is a helper function that can be called from job runners to emit events
pub fn broadcast_event(state: &AppState, event: OrbitEvent) {
    state.emit_event(event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_serialization() {
        let event =
            OrbitEvent::job_updated("test-job-123".to_string(), "processing".to_string(), 0.5);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("JobUpdated"));
        assert!(json.contains("test-job-123"));
        assert!(json.contains("0.5"));
    }
}
