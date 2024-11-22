use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    Form,
};
use chrono::{NaiveDateTime, Utc};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};
use tokio::sync::RwLock;

#[derive(Deserialize)]
pub struct Chatform {
    name: String,
}

pub async fn chat(State(chat): State<Arc<Chat>>, Form(f): Form<Chatform>) -> impl IntoResponse {
    // join chat
    if f.name.to_lowercase() == "system" {
        return "Username 'system' is not allowed".into();
    }
    let user_hash = seahash::hash(f.name.as_bytes());
    if let Err(e) = chat.join(f.name.clone()).await {
        return e.into();
    };
    let reply = format!(
        r##"
        <div id="openChat" hx-ext="ws" ws-connect="/chat/ws/{user_hash}">
            <div class="mb-3">
                <form ws-send>
                    <div class="input-group">
                        <input name="chat_message" type="text" class="form-control" placeholder="Type your message...">
                        <button class="btn btn-primary" type="submit">Send</button>
                    </div>
                </form>
            </div>
            <div id="chatBox" class="chat-container">
            </div>
        </div>
    "##
    );
    reply
}

pub async fn websocket_handler(
    State(chat): State<Arc<Chat>>,
    Path(id): Path<u64>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, chat, id))
}

async fn handle_socket(ws: WebSocket, chat: Arc<Chat>, id: u64) {
    let Some(user) = chat.find(id).await else {
        log::warn!("user with id '{id}' cannot be found");
        let _ = ws.close().await;
        return;
    };
    log::info!("user '{user}' ({id}) opened the socket");

    // Chat handles
    let (mut ws_send, mut ws_recv) = ws.split();
    let (chat_tx, mut chat_rx) = (chat.tx.clone(), chat.tx.subscribe());

    // Consume old message buffer
    for msg in chat.log.read().await.iter() {
        if ws_send
            .send(Message::Text(msg.websocket_reply()))
            .await
            .is_err()
        {
            break;
        }
    }

    // open tasks
    let ws_chat_clone = chat.clone();
    let ws_recv_user = user.clone();
    let mut ws_to_chat_task = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_recv.next().await {
            match message {
                Message::Text(json) => {
                    let Ok(val) = serde_json::from_str::<Value>(&json) else {
                        break;
                    };
                    let text = val["chat_message"]
                        .as_str()
                        .unwrap_or("cannot parse chat message")
                        .to_string();
                    log::debug!("user sent: {ws_recv_user}: {text}");
                    let msg = ChatMessage {
                        timestamp: Utc::now().naive_utc(),
                        username: ws_recv_user.clone(),
                        message: text,
                    };
                    ws_chat_clone.log(&msg).await;
                    if let Err(e) = chat_tx.send(msg) {
                        log::error!("Cannot send chat message: {e}");
                    };
                }
                Message::Close(_) => {
                    log::warn!("Socket for user '{}' closed by client", ws_recv_user);
                    return;
                }
                _ => {
                    log::warn!("not implemented");
                }
            }
        }
    });

    let chat_user = user.clone();
    let mut chat_to_ws_task = tokio::spawn(async move {
        while let Ok(msg) = chat_rx.recv().await {
            log::debug!("user '{}' receives: {}", chat_user, msg.log());
            if ws_send
                .send(Message::Text(msg.websocket_reply()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let join_msg = ChatMessage {
        timestamp: Utc::now().naive_utc(),
        username: "System".into(),
        message: format!("'{}' joined the chat", &user),
    };
    chat.log(&join_msg).await;
    if let Err(e) = chat.tx.send(join_msg) {
        log::error!("Cannot send chat message: {e}");
    };

    tokio::select! {
        _ = &mut chat_to_ws_task => ws_to_chat_task.abort(),
        _ = &mut ws_to_chat_task => chat_to_ws_task.abort(),
    };

    let leave_msg = ChatMessage {
        timestamp: Utc::now().naive_utc(),
        username: "System".into(),
        message: format!("'{}' left the chat", &user),
    };
    chat.log(&leave_msg).await;
    if let Err(e) = chat.tx.send(leave_msg) {
        log::error!("Cannot send chat message: {e}");
    };

    log::info!("user '{user}' ({id}) left the chat");
    chat.leave(&user).await;
}

#[derive(Clone)]
struct ChatMessage {
    timestamp: NaiveDateTime,
    username: String,
    message: String,
}

impl ChatMessage {
    fn log(&self) -> String {
        format!(
            "{} {}: {}",
            self.timestamp.and_utc().to_rfc3339(),
            self.username,
            self.message
        )
    }
    fn websocket_reply(&self) -> String {
        format!(
            r#"<div id="chatBox" hx-swap-oob="afterbegin">
                <div class="chat-message">
                    <strong>{}:</strong>
                    <span class="timestamp">{}</span>
                    <p>{}</p>
                <div>
            </div>"#,
            self.username,
            self.timestamp.and_utc().format("%Y-%m-%d %H:%M:%S"),
            self.message
        )
    }
}

pub struct Chat {
    tx: tokio::sync::broadcast::Sender<ChatMessage>,
    users: RwLock<HashSet<String>>,
    log: RwLock<VecDeque<ChatMessage>>,
}

impl Chat {
    pub fn new(log_size: usize) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(log_size);
        Chat {
            tx,
            users: RwLock::new(HashSet::new()),
            log: RwLock::new(VecDeque::new()),
        }
    }
    pub async fn join(&self, username: String) -> Result<(), &'static str> {
        if !self.users.write().await.insert(username.clone()) {
            log::warn!("User {} already in chat", username);
            Err("user already in chat")
        } else {
            log::info!("User {} joined the chat", username);
            Ok(())
        }
    }
    pub async fn leave(&self, username: &String) -> bool {
        self.users.write().await.remove(username)
    }
    pub async fn find(&self, id: u64) -> Option<String> {
        self.users
            .read()
            .await
            .iter()
            .find(|name| seahash::hash(name.as_bytes()) == id)
            .cloned()
    }
    async fn log(&self, msg: &ChatMessage) {
        self.log.write().await.push_back(msg.clone());
        if self.log.read().await.len() > 1000 {
            let _ = self.log.write().await.pop_front();
        }
    }
}
