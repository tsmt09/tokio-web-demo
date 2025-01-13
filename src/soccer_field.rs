use std::{
    collections::HashMap,
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::{Html, IntoResponse},
};
use futures::{SinkExt, StreamExt};
use minijinja::{context, path_loader};
use rand::Rng;
use serde::Serialize;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::AppState;

const FIELD_BOUNDARY_Y: i16 = 800;
const FIELD_BOUNDARY_X: i16 = 400;
const MAX_PLAYER_SPEED: f32 = 5.0;
const PLAYER_ACCELERATION: f32 = 0.1;
const TICKRATE: u16 = 30;

#[derive(Debug)]
struct SoccerField {
    players: HashMap<u16, Player>,
    ball: Ball,
    id_inc: u16,
}

#[derive(Debug)]
struct Ball {
    position: Coordinate,
    speed: Speed,
}

#[derive(Debug)]
struct Player {
    target: Coordinate,
    position: Coordinate,
    speed: Speed,
}

//                  y    x
#[derive(Copy, Clone, Debug, Serialize)]
struct Coordinate(i16, i16);
//             y    x
#[derive(Copy, Clone, Debug)]
struct Speed(f32, f32);

fn update_pos_with_speed(pos: i16, speed: f32) -> i16 {
    (pos as f32 + speed).round() as i16
}

impl Coordinate {
    fn apply_speed_with_boundaries(&mut self, speed: Speed) {
        self.0 = std::cmp::max(
            8,
            std::cmp::min(update_pos_with_speed(self.0, speed.0), FIELD_BOUNDARY_Y - 8),
        );
        self.1 = std::cmp::max(
            8,
            std::cmp::min(update_pos_with_speed(self.1, speed.1), FIELD_BOUNDARY_X - 8),
        );
    }
    fn distance_angle(&self, other: &Self) -> (f32, f32) {
        let dy: f32 = (self.0 - other.0) as f32;
        let dx: f32 = (self.1 - other.1) as f32;
        (f32::sqrt(dx * dx + dy * dy), f32::atan2(dy, dx))
    }
}

impl Speed {
    fn reduce(&mut self) {
        self.0 = self.0 * 0.9;
        self.1 = self.1 * 0.9;
        if self.0 == 0.0 {
            self.1 = 0.0;
        }
        if self.1 == 0.0 {
            self.0 = 0.0;
        }
    }
}

impl Player {
    fn update_target(&mut self, y: i16, x: i16) {
        self.target.0 = y;
        self.target.1 = x;
    }
    fn update(&mut self, ball: &mut Ball) {
        let (distance, angle) = self.target.distance_angle(&self.position);
        if distance > 5.0 {
            let distance_speed = if distance * PLAYER_ACCELERATION > MAX_PLAYER_SPEED {
                distance * PLAYER_ACCELERATION
            } else {
                MAX_PLAYER_SPEED
            };
            self.speed.0 = distance_speed * angle.sin();
            self.speed.1 = distance_speed * angle.cos();
            self.position.apply_speed_with_boundaries(self.speed);
            // check collision
            let (distance, angle) = self.position.distance_angle(&ball.position);
            if distance < 18.0 {
                ball.pushed_by(angle, &self);
            }
        }
    }
}

impl Ball {
    fn update(&mut self) {
        self.update_speed();
        self.update_position();
        self.bounce();
    }
    fn update_position(&mut self) {
        self.position.apply_speed_with_boundaries(self.speed);
    }
    fn update_speed(&mut self) {
        self.speed.reduce();
    }
    fn bounce(&mut self) {
        // checking limits
        match self.position {
            Coordinate(y, _) if { y >= FIELD_BOUNDARY_Y - 8 } => {
                self.speed.0 = self.speed.0 * -1.0;
            }
            Coordinate(y, _) if { y <= 8 } => {
                self.speed.0 = self.speed.0 * -1.0;
            }
            Coordinate(_, x) if { x >= FIELD_BOUNDARY_X - 8 } => {
                self.speed.1 = self.speed.1 * -1.0;
            }
            Coordinate(_, x) if { x <= 8 } => {
                self.speed.1 = self.speed.1 * -1.0;
            }
            Coordinate(_, _) => {}
        }
    }
    fn pushed_by(&mut self, angle: f32, player: &Player) {
        self.speed.0 = player.speed.0 * angle.sin();
        self.speed.1 = player.speed.1 * angle.cos();
        self.update_position();
    }
}

impl SoccerField {
    fn update(&mut self) {
        for (_, player) in self.players.iter_mut() {
            player.update(&mut self.ball);
        }
        self.ball.update();
    }

    fn new() -> Self {
        Self {
            players: HashMap::new(),
            ball: Ball {
                position: Coordinate(400, 200),
                speed: Speed(0.0, 0.0),
            },
            id_inc: 0,
        }
    }
    fn player_join(&mut self) -> u16 {
        let player_id = self.id_inc;
        self.id_inc += 1;
        self.players.insert(
            player_id,
            Player {
                target: Coordinate(0, 0),
                position: Coordinate(0, 0),
                speed: Speed(0.0, 0.0),
            },
        );
        player_id
    }
    fn player_leave(&mut self, id: u16) {
        self.players.remove(&id);
    }
    fn positions(&self) -> PositionsList {
        PositionsList {
            ball: self.ball.position,
            players: self
                .players
                .iter()
                .map(|(id, p)| (*id, p.position))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PositionsList {
    ball: Coordinate,
    players: HashMap<u16, Coordinate>,
}

#[derive(Clone, Debug)]
enum PlayerUpdates {
    Position(u16, i16, i16),
    Join(u16),
    Leave(u16),
}

#[derive(Clone, Debug)]
enum FieldUpdates {
    Positions(PositionsList),
    Joined(u16, u16),
}

pub struct SoccerFieldThread {
    _handle: JoinHandle<()>,
    _exit: oneshot::Sender<bool>,
    field_updates: broadcast::Sender<FieldUpdates>,
    player_updates: mpsc::Sender<PlayerUpdates>,
}

impl SoccerFieldThread {
    pub fn spawn() -> Self {
        let (fu_tx, _) = broadcast::channel(2048);
        let (player_updates, pu_rx) = mpsc::channel(2048);
        let (exit, exit_rx) = oneshot::channel();

        let field_updates = fu_tx.clone();

        let handle = std::thread::spawn(move || Self::thread(fu_tx.clone(), pu_rx, exit_rx));

        SoccerFieldThread {
            _handle: handle,
            _exit: exit,
            field_updates,
            player_updates,
        }
    }

    fn thread(
        field_updates: broadcast::Sender<FieldUpdates>,
        mut player_updates: mpsc::Receiver<PlayerUpdates>,
        mut exit: oneshot::Receiver<bool>,
    ) {
        // init soccerfield
        let mut field = SoccerField::new();
        let mut last_debug_print = Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000 / TICKRATE as u64));
            if let Ok(_) = exit.try_recv() {
                return;
            }
            if Instant::now().duration_since(last_debug_print) > Duration::from_secs(1) {
                last_debug_print = Instant::now();
                log::debug!("Field positions: {:?}", field.positions());
            }
            field.update();
            if field_updates.receiver_count() > 0 {
                let _ = field_updates.send(FieldUpdates::Positions(field.positions()));
            }
            if player_updates.sender_strong_count() > 0 {
                while let Ok(update) = player_updates.try_recv() {
                    match update {
                        PlayerUpdates::Position(id, y, x) => {
                            if let Some(player) = field.players.get_mut(&id) {
                                player.update_target(y, x);
                            }
                        }
                        PlayerUpdates::Join(token) => {
                            let id = field.player_join();
                            log::info!("Join request by {} as id {}", token, id);
                            let _ = field_updates.send(FieldUpdates::Joined(token, id));
                        }
                        PlayerUpdates::Leave(id) => {
                            field.player_leave(id);
                            log::info!("Player {} left the field", id);
                        }
                    }
                }
            }
        }
    }
}

pub async fn get_field() -> impl IntoResponse {
    let mut minij = minijinja::Environment::new();
    minij.set_loader(path_loader("templates"));
    let rendered = minij
        .get_template("field.html")
        .unwrap()
        .render(context! {})
        .unwrap();
    Html::from(rendered)
}

pub async fn websocket_handler(
    State(app): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let thread = Arc::clone(&app.soccer_thread);
    ws.on_upgrade(move |socket| handle_socket(socket, thread))
}

async fn handle_socket(ws: WebSocket, soccer_thread: Arc<SoccerFieldThread>) {
    let local_token: u16 = rand::thread_rng().gen();
    // join field
    let player_updates = soccer_thread.player_updates.clone();
    player_updates
        .send(PlayerUpdates::Join(local_token))
        .await
        .unwrap();
    // wait for id
    let mut receiver = soccer_thread.field_updates.subscribe();
    let mut waiting = true;
    let mut my_id = 0;
    log::info!(
        "client connected on soccerthread, joins with token {}",
        local_token
    );
    while waiting {
        if let Ok(FieldUpdates::Joined(token, id)) = receiver.recv().await {
            if token == local_token {
                log::info!("{} joined as id {}", local_token, id);
                waiting = false;
                my_id = id;
            }
        }
    }
    // split ws
    let (mut ws_send, mut ws_recv) = ws.split();
    // server to client task
    let mut server_to_client_task = tokio::spawn(async move {
        while let Ok(update) = receiver.recv().await {
            match &update {
                FieldUpdates::Positions(poslist) => {
                    ws_send
                        .send(Message::Text(serde_json::to_string(&poslist).unwrap()))
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    });

    // client to server task

    let mut client_to_server_task = tokio::spawn(async move {
        while let Some(update) = ws_recv.next().await {
            match update {
                Ok(Message::Text(json)) => {
                    let (yf, xf): (f32, f32) = serde_json::from_str(&json).unwrap();
                    let (y, x): (i16, i16) = (yf.round() as i16, xf.round() as i16);
                    let _ = player_updates
                        .send(PlayerUpdates::Position(my_id, y, x))
                        .await;
                }
                Ok(Message::Close(_)) => {
                    log::warn!("Socket for user '{}' closed by client", my_id);
                    let _ = player_updates.send(PlayerUpdates::Leave(my_id)).await;
                    return;
                }
                Ok(_) => {
                    log::warn!("not implemented");
                }
                Err(error) => {
                    log::error!("websocket error: {:?}", error);
                    let _ = player_updates.send(PlayerUpdates::Leave(my_id)).await;
                    return;
                }
            }
        }
    });

    tokio::select! {
        _ = &mut client_to_server_task => server_to_client_task.abort(),
        _ = &mut server_to_client_task => client_to_server_task.abort(),
    };
}
