use dioxus::prelude::*;

mod screens;
mod ws;

use futures::channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};

use screens::game_screen::GameScreen;
use screens::login_screen::LoginScreen;
use screens::main_screen::MenuScreen;
use ws::WsHandle;

const FAVICON: Asset = asset!("/assets/favicon.svg");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[derive(Clone, PartialEq)]
enum Screen {
    LoginScreen,
    MenuScreen,
    GameScreen,
}

const SERVER_URL: &str = env!("SERVER_URL");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Start directly at login
    let screen = use_signal(|| Screen::LoginScreen);

    let username = use_signal(|| String::new());
    let ws_handle: Signal<Option<WsHandle>> = use_signal(|| None);
    let server_msgs: Signal<Vec<String>> = use_signal(|| Vec::new());

    // Connect automatically once
    use_effect(move || {
        let mut ws_handle = ws_handle.clone();
        let mut server_msgs = server_msgs.clone();

        spawn(async move {
            let ws = match WebSocket::open(SERVER_URL) {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket connect failed: {e:?}");
                    return;
                }
            };

            let (mut sink, mut stream) = ws.split();
            let (tx, mut rx) = mpsc::unbounded::<String>();

            ws_handle.set(Some(WsHandle { tx }));

            // outgoing messages
            spawn(async move {
                while let Some(msg) = rx.next().await {
                    let _ = sink.send(Message::Text(msg)).await;
                }
            });

            // incoming messages
            while let Some(Ok(Message::Text(msg))) = stream.next().await {
                server_msgs.write().push(msg);
            }
        });
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        match screen() {
            Screen::LoginScreen => rsx! {
                LoginScreen {
                    ws_handle: ws_handle,
                    server_msgs: server_msgs,
                    screen: screen,
                    username: username,
                }
            },
            Screen::MenuScreen => rsx! {
                MenuScreen {
                    ws_handle: ws_handle,
                    server_msgs: server_msgs,
                    screen: screen,
                    username: username,
                }
            },
            Screen::GameScreen => rsx! {
                GameScreen {
                    ws_handle: ws_handle,
                    server_msgs: server_msgs,
					screen: screen,
                    username: username,
                }
            },
        }
    }
}