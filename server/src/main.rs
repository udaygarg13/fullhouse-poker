use common::game_models::GameConfig;
use futures_util::{SinkExt, StreamExt};
use server::db::{Error, PlayerDatabase};
use server::game::{
    add_viewer_sender, clear_viewers, remove_viewer_sender, run_game, send_cached_viewer_state,
    take_final_player_senders, viewer_senders,
};
use std::{collections::HashSet, sync::Arc};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender as ActionSender;
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

use futures_util::stream::{SplitSink, SplitStream};

type Ws = WebSocketStream<TcpStream>;

type ClientSender = tokio::sync::mpsc::UnboundedSender<String>;

#[derive(Clone)]
struct SenderEntry {
    username: Option<String>,
    addr: std::net::SocketAddr,
    tx: ClientSender,
    game_action_tx: Option<ActionSender<(String, String)>>,
}

#[derive(Default)]
struct Queue {
    host: Option<String>,
    players: Vec<String>,
}

impl Queue {
    fn broadcast_msg(&self) -> String {
        let host = self.host.as_deref().unwrap_or("NONE");
        let players = if self.players.is_empty() {
            "EMPTY".to_string()
        } else {
            self.players.join(",")
        };
        format!("QUEUE {} {}", host, players)
    }
}

enum Command {
    Login(String, String),
    Create(String, String),
    Rules,
    Stats,
    Results,
    Deposit(i32),
    Withdraw(i32),
    Logout,
    Host,
    Join,
    Leave,
    Start,
    SaveRules(String),
    Unknown,
}

impl Command {
    fn parse(input: &str) -> Self {
        let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();

        match parts.as_slice() {
            ["LOGIN", u, p] => Self::Login((*u).into(), (*p).into()),
            ["CREATE", u, p] => Self::Create((*u).into(), (*p).into()),
            ["RULES"] => Self::Rules,
            ["STATS"] => Self::Stats,
            ["RESULTS"] => Self::Results,
            ["DEPOSIT", v] => v.parse().ok().map(Self::Deposit).unwrap_or(Self::Unknown),
            ["WITHDRAW", v] => v.parse().ok().map(Self::Withdraw).unwrap_or(Self::Unknown),
            ["LOGOUT"] => Self::Logout,
            ["HOST"] => Self::Host,
            ["JOIN"] => Self::Join,
            ["LEAVE"] => Self::Leave,
            ["START"] => Self::Start,
            ["SAVE_RULES", json] => Self::SaveRules((*json).into()),
            _ => Self::Unknown,
        }
    }
}

struct ClientSession {
    username: Option<String>,
    addr: std::net::SocketAddr,
    sink: SplitSink<Ws, Message>,
    stream: SplitStream<Ws>,
    db: Arc<PlayerDatabase>,
    logged_in: Arc<Mutex<HashSet<String>>>,
    config: Arc<Mutex<GameConfig>>,
    queue: Arc<Mutex<Queue>>,
    broadcast_senders: Arc<Mutex<Vec<SenderEntry>>>,
    client_receiver: tokio::sync::mpsc::UnboundedReceiver<String>,
    game_in_progress: Arc<Mutex<bool>>,
}

impl ClientSession {
    async fn send(&mut self, msg: impl Into<String>) {
        if let Err(e) = self.sink.send(Message::Text(msg.into().into())).await {
            eprintln!("Send Failed: {e}");
        }
    }

    async fn receive_user(&mut self) -> Option<String> {
        while let Some(msg) = self.stream.next().await {
            match msg {
                Ok(Message::Text(t)) => return Some(t.to_string()),
                Ok(Message::Close(_)) | Err(_) => return None,
                _ => {}
            }
        }
        None
    }

    async fn receive_menu(&mut self) -> Option<String> {
        loop {
            tokio::select! {
                ws_msg = self.stream.next() => {
                    match ws_msg {
                        Some(Ok(Message::Text(t))) => return Some(t.to_string()),
                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return None,
                        _ => {}
                    }
                }
                bcast = self.client_receiver.recv() => {
                    if let Some(msg) = bcast {
                        self.send(msg).await;
                    }
                }
            }
        }
    }

    async fn broadcast(&self, msg: String) {
        let senders = self.broadcast_senders.lock().await;
        for entry in senders.iter() {
            if let Err(e) = entry.tx.send(msg.clone()) {
                eprintln!(
                    "Failed to send broadcast to {}: {}",
                    entry.username.as_deref().unwrap_or("unknown"),
                    e
                );
            }
        }
    }

    async fn leave_queue(&self, username: &str) {
        let msg = {
            let mut q = self.queue.lock().await;
            if q.host.as_deref() == Some(username) {
                *q = Queue::default();
            } else {
                q.players.retain(|p| p != username);
            }
            q.broadcast_msg()
        };
        self.broadcast(msg).await;
    }

    async fn login_page(&mut self) -> Result<(), ()> {
        loop {
            let line = self.receive_user().await.ok_or(())?;

            match Command::parse(&line) {
                Command::Login(user, pin) => match self.db.login(&user, &pin) {
                    Ok(_) => {
                        let already_logged_in = {
                            let mut active = self.logged_in.lock().await;
                            if active.contains(&user) {
                                true
                            } else {
                                active.insert(user.clone());
                                false
                            }
                        };

                        if already_logged_in {
                            self.send("ERR Account already logged in").await;
                            continue;
                        }

                        self.username = Some(user.clone());

                        {
                            let mut senders = self.broadcast_senders.lock().await;
                            for entry in senders.iter_mut() {
                                if entry.addr == self.addr && entry.username.is_none() {
                                    entry.username = Some(user.clone());
                                    break;
                                }
                            }
                        }
                        self.send("OK").await;
                        return Ok(());
                    }
                    Err(e) => {
                        self.send(format!("ERR {}", e)).await;
                    }
                },

                Command::Create(user, pin) => match self.db.create_account(user, pin, 1000) {
                    Ok(_) => self.send("OK Account created").await,
                    Err(e) => self.send(format!("ERR {}", e)).await,
                },

                _ => self.send("ERR Must LOGIN or CREATE").await,
            }
        }
    }

    async fn menu_page(&mut self) {
        if *self.game_in_progress.lock().await {
            self.send("GAME_IN_PROGRESS".to_string()).await;
        } else {
            let current_queue = self.queue.lock().await.broadcast_msg();
            self.send(current_queue).await;
        }

        while let Some(line) = self.receive_menu().await {
            if line == "WATCH" {
                if *self.game_in_progress.lock().await {
                    let username = self.username.clone().unwrap_or_default();
                    let tx = {
                        let senders = self.broadcast_senders.lock().await;
                        senders
                            .iter()
                            .find(|e| e.username.as_deref() == Some(&username))
                            .map(|e| e.tx.clone())
                    };
                    if let Some(tx) = tx {
                        add_viewer_sender(tx.clone()).await;
                        send_cached_viewer_state(&tx).await;
                        self.send("OK Watching").await;
                    } else {
                        self.send("ERR Could not start watching").await;
                    }
                } else {
                    self.send("ERR No game in progress").await;
                }
                continue;
            }

            if line == "STOP_WATCH" {
                let username = self.username.clone().unwrap_or_default();
                let tx = {
                    let senders = self.broadcast_senders.lock().await;
                    senders
                        .iter()
                        .find(|e| e.username.as_deref() == Some(&username))
                        .map(|e| e.tx.clone())
                };
                if let Some(tx) = tx {
                    remove_viewer_sender(&tx).await;
                }
                self.send("OK Stopped watching").await;
                if *self.game_in_progress.lock().await {
                    self.send("GAME_IN_PROGRESS").await;
                }
                continue;
            }

            if line.starts_with("GAME ") {
                let game_cmd = line.trim_start_matches("GAME ").to_string();
                let username = self.username.clone().unwrap_or_default();
                let tx = {
                    let senders = self.broadcast_senders.lock().await;
                    senders
                        .iter()
                        .find(|e| e.username.as_deref() == Some(&username))
                        .and_then(|e| e.game_action_tx.clone())
                };
                if let Some(tx) = tx {
                    let _ = tx.send((username, game_cmd));
                } else {
                    self.send("ERR No active game").await;
                }
                continue;
            }

            if matches!(line.as_str(), "HOST" | "JOIN" | "START")
                && *self.game_in_progress.lock().await
            {
                self.send("ERR A game is in progress. Send WATCH to spectate.")
                    .await;
                continue;
            }

            let cmd = Command::parse(&line);
            if !self.dispatch(cmd).await {
                break;
            }
        }
    }

    async fn dispatch(&mut self, cmd: Command) -> bool {
        match cmd {
            Command::Rules => {
                let rules = {
                    let cfg = self.config.lock().await;
                    serde_json::to_string(&*cfg).unwrap_or_else(|e| format!("ERR {}", e))
                };
                self.send(rules).await;
            }

            Command::SaveRules(json) => match serde_json::from_str::<GameConfig>(&json) {
                Ok(new_config) => {
                    *self.config.lock().await = new_config;
                    self.send("OK Config updated").await;
                }
                Err(_) => self.send("ERR Invalid config format").await,
            },

            Command::Stats => match self.db.get_all_players() {
                Ok(players) if players.is_empty() => self.send("OK No players").await,

                Ok(players) => {
                    let text = players
                        .iter()
                        .map(|p| {
                            format!(
                                "{}|{}|{}|{}|{}",
                                p.name, p.balance, p.rounds_played, p.pots_won, p.folds
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    self.send(format!("OK {}", text)).await;
                }

                Err(e) => {
                    self.send(format!("ERR {}", e)).await;
                }
            },

            Command::Results => match self.db.get_round_results() {
                Ok(rows) if rows.is_empty() => self.send("OK No results").await,

                Ok(rows) => {
                    let text = rows
                        .iter()
                        .map(|(ended_at, winner, winning_hand, amount_won)| {
                            format!("{}|{}|{}|{}", ended_at, winner, winning_hand, amount_won)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    self.send(format!("OK {}", text)).await;
                }

                Err(e) => self.send(format!("ERR {}", e)).await,
            },

            Command::Deposit(v) => {
                let user = self.username.as_ref().unwrap();
                match self.db.deposit(user, v) {
                    Ok(bal) => self.send(format!("OK Balance ${}", bal)).await,
                    Err(e) => self.send(format!("ERR {}", e)).await,
                }
            }

            Command::Withdraw(v) => {
                let user = self.username.as_ref().unwrap();
                match self.db.withdraw(user, v) {
                    Ok(bal) => self.send(format!("OK Balance ${}", bal)).await,
                    Err(Error::InsufficientFunds) => self.send("ERR Insufficient funds").await,
                    Err(e) => self.send(format!("ERR {}", e)).await,
                }
            }

            Command::Logout => {
                let username = self.username.clone().unwrap_or_default();
                self.leave_queue(&username).await;
                self.send("OK").await;
                return false;
            }

            Command::Host => {
                let username = self.username.clone().unwrap_or_default();
                let msg = {
                    let mut q = self.queue.lock().await;
                    if q.host.is_some() {
                        None
                    } else {
                        q.host = Some(username.clone());
                        q.players.push(username.clone());
                        Some(q.broadcast_msg())
                    }
                };

                match msg {
                    Some(bcast) => {
                        self.send("OK Hosting").await;
                        self.broadcast(bcast).await;
                    }
                    None => {
                        self.send("ERR A game is already being hosted").await;
                    }
                }
            }

            Command::Join => {
                let username = self.username.clone().unwrap_or_default();
                let msg = {
                    let mut q = self.queue.lock().await;
                    if q.host.is_none() {
                        Err("ERR No game is being hosted")
                    } else if q.players.contains(&username) {
                        Err("ERR Already in queue")
                    } else {
                        q.players.push(username.clone());
                        Ok(q.broadcast_msg())
                    }
                };

                match msg {
                    Ok(bcast) => {
                        self.send("OK Joined queue").await;
                        self.broadcast(bcast).await;
                    }
                    Err(e) => {
                        self.send(e).await;
                    }
                }
            }

            Command::Leave => {
                let username = self.username.clone().unwrap_or_default();
                self.leave_queue(&username).await;
                self.send("OK Left queue").await;
            }

            Command::Start => {
                let username = self.username.clone().unwrap_or_default();
                if *self.game_in_progress.lock().await {
                    self.send("ERR A game is in progress. Send WATCH to spectate.")
                        .await;
                    return true;
                }
                let result = {
                    let q = self.queue.lock().await;
                    if q.host.as_deref() != Some(&username) {
                        Err("ERR Only the host can start the game")
                    } else if q.players.len() < 2 {
                        Err("ERR Need at least 2 players to start")
                    } else {
                        Ok(q.players.clone())
                    }
                };

                match result {
                    Ok(queued_usernames) => {
                        let (action_tx, action_rx) =
                            tokio::sync::mpsc::unbounded_channel::<(String, String)>();
                        *self.game_in_progress.lock().await = true;

                        let game_players: Vec<(String, std::net::SocketAddr, ClientSender)> = {
                            let mut senders = self.broadcast_senders.lock().await;
                            queued_usernames
                                .iter()
                                .filter_map(|uname| {
                                    senders
                                        .iter_mut()
                                        .find(|e| e.username.as_deref() == Some(uname))
                                        .map(|e| {
                                            e.game_action_tx = Some(action_tx.clone());
                                            (e.username.clone().unwrap(), e.addr, e.tx.clone())
                                        })
                                })
                                .collect()
                        };

                        for (_, _, tx) in &game_players {
                            let _ = tx.send("GAME_START".to_string());
                        }
                        {
                            let senders = self.broadcast_senders.lock().await;
                            for entry in senders.iter() {
                                if let Some(uname) = entry.username.as_deref() {
                                    if !queued_usernames.iter().any(|u| u == uname) {
                                        let _ = entry.tx.send("GAME_IN_PROGRESS".to_string());
                                    }
                                }
                            }
                        }

                        {
                            *self.queue.lock().await = Queue::default();
                        }

                        let db = self.db.clone();
                        let config = self.config.lock().await.clone();
                        let gip = self.game_in_progress.clone();
                        let broadcast_senders = self.broadcast_senders.clone();

                        tokio::spawn(async move {
                            run_game(game_players, action_rx, config, db).await;
                            *gip.lock().await = false;
                            let remaining_players = take_final_player_senders().await;
                            let viewers = viewer_senders().lock().await;

                            let mut targets: Vec<ClientSender> = remaining_players;
                            for tx in viewers.iter() {
                                if !targets.iter().any(|existing| existing.same_channel(tx)) {
                                    targets.push(tx.clone());
                                }
                            }
                            drop(viewers);

                            {
                                let senders = broadcast_senders.lock().await;
                                for entry in senders.iter() {
                                    if entry.username.is_some()
                                        && !targets
                                            .iter()
                                            .any(|existing| existing.same_channel(&entry.tx))
                                    {
                                        targets.push(entry.tx.clone());
                                    }
                                }
                            }

                            for tx in &targets {
                                let _ = tx.send("GAME_ENDED".to_string());
                            }
                            clear_viewers().await;
                        });
                    }
                    Err(e) => {
                        self.send(e).await;
                    }
                }
            }

            Command::Unknown | Command::Login(_, _) | Command::Create(_, _) => {
                self.send("ERR Unknown command").await;
            }
        }

        true
    }

    async fn user_cleanup(&mut self) {
        if let Some(user) = self.username.clone() {
            self.leave_queue(&user).await;
            self.logged_in.lock().await.remove(&user);
            let user_tx = {
                let senders = self.broadcast_senders.lock().await;
                senders
                    .iter()
                    .find(|entry| entry.username.as_deref() == Some(&user))
                    .map(|entry| entry.tx.clone())
            };
            {
                let mut senders = self.broadcast_senders.lock().await;
                for entry in senders.iter_mut() {
                    if entry.username.as_deref() == Some(&user) {
                        if let Some(tx) = &entry.game_action_tx {
                            let _ = tx.send((user.clone(), "LEAVE".to_string()));
                        }
                        entry.game_action_tx = None;
                    }
                }
                senders.retain(|entry| !entry.tx.is_closed());
            }
            if let Some(tx) = user_tx {
                remove_viewer_sender(&tx).await;
            }
            println!("{user} disconnected");
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    addr: std::net::SocketAddr,
    db: Arc<PlayerDatabase>,
    logged_in: Arc<Mutex<HashSet<String>>>,
    config: Arc<Mutex<GameConfig>>,
    queue: Arc<Mutex<Queue>>,
    broadcast_senders: Arc<Mutex<Vec<SenderEntry>>>,
    game_in_progress: Arc<Mutex<bool>>,
) {
    let ws = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let (sink, stream) = ws.split();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    {
        let mut senders = broadcast_senders.lock().await;
        senders.retain(|entry| !entry.tx.is_closed());
        senders.push(SenderEntry {
            username: None,
            addr,
            tx,
            game_action_tx: None,
        });
    }

    let mut session = ClientSession {
        username: None,
        addr,
        sink,
        stream,
        db,
        logged_in,
        config,
        queue,
        broadcast_senders,
        client_receiver: rx,
        game_in_progress,
    };

    if session.login_page().await.is_ok() {
        session.menu_page().await;
    }

    session.user_cleanup().await;
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "7878".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Server listening on {}", addr);

    let db = Arc::new(PlayerDatabase::new("database.db").unwrap());
    let logged_in = Arc::new(Mutex::new(HashSet::<String>::new()));
    let config = Arc::new(Mutex::new(GameConfig::default()));
    let queue = Arc::new(Mutex::new(Queue::default()));
    let broadcast_senders: Arc<Mutex<Vec<SenderEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let game_in_progress: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        println!("Client connected: {}", addr);

        tokio::spawn(handle_client(
            stream,
            addr,
            db.clone(),
            logged_in.clone(),
            config.clone(),
            queue.clone(),
            broadcast_senders.clone(),
            game_in_progress.clone(),
        ));
    }
}
