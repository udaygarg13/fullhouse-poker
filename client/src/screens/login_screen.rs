use crate::ws::WsHandle;
use crate::Screen;
use dioxus::prelude::*;

#[component]
pub fn LoginScreen(
    ws_handle: Signal<Option<WsHandle>>,
    server_msgs: Signal<Vec<String>>,
    screen: Signal<Screen>,
    username: Signal<String>,
) -> Element {
    let mut user_input = use_signal(|| String::new());
    let mut pin_input = use_signal(|| String::new());
    let mut error_msg = use_signal(|| String::new());
    let mut tab = use_signal(|| "login");

    use_effect(move || {
        let msgs = server_msgs();
        if msgs.is_empty() {
            return;
        }

        for resp in &msgs {
            if resp.starts_with("OK") {
                let body = resp.get(3..).unwrap_or("").trim().to_string();

                if body.to_lowercase().contains("account created") {
                    error_msg.set("Account created! You can now log in.".into());
                    continue;
                }

                if *tab.peek() == "login" {
                    username.set(user_input.peek().trim().to_string());
                    screen.set(Screen::MenuScreen);
                    return;
                }
            } else if resp.starts_with("ERR") {
                let body = resp.get(4..).unwrap_or("").trim().to_string();
                error_msg.set(body);
            }
        }

        server_msgs.write().clear();
    });

    let on_submit = move |_| {
        let ws = ws_handle().clone();
        let u = user_input();
        let p = pin_input();
        error_msg.set(String::new());

        if let Some(ws) = ws {
            if tab() == "login" {
                ws.send(format!("LOGIN {} {}", u.trim(), p.trim()));
            } else {
                ws.send(format!("CREATE {} {}", u.trim(), p.trim()));
            }
        }
    };

    let is_success = error_msg().contains("Account created");

    rsx! {
        div {
            class: "flex flex-col items-center justify-center h-screen bg-gray-950 font-sans relative overflow-hidden",

            button {
                class: "absolute top-4 left-4 px-3 py-1 rounded-md bg-gray-800 border border-gray-700 text-gray-300 hover:bg-gray-700 text-xs",
                onclick: move |_| {
                    screen.set(Screen::ConnectScreen);
                },
                "Back"
            }

            div { class: "text-center mb-10 relative",
                h1 { class: "text-4xl font-bold mb-1 text-gray-200 uppercase", "Poker" }
                p { class: "text-gray-500 text-xs tracking-widest uppercase", "Team Fullhouse" }
            }

            div {
                class: "bg-gray-900 p-8 rounded-lg w-[380px] border border-gray-800 relative",

                div { class: "flex mb-8 bg-gray-800 rounded-lg p-1 border border-gray-700",
                    button {
                        class: if tab() == "login" {
                            "flex-1 py-2 px-4 bg-gray-900 text-gray-100 font-semibold text-sm rounded-lg shadow"
                        } else {
                            "flex-1 py-2 px-4 bg-transparent text-gray-500 font-medium text-sm rounded-lg"
                        },
                        onclick: move |_| { tab.set("login"); error_msg.set(String::new()); },
                        "Sign In"
                    }
                    button {
                        class: if tab() == "create" {
                            "flex-1 py-2 px-4 bg-gray-900 text-gray-100 font-semibold text-sm rounded-lg shadow"
                        } else {
                            "flex-1 py-2 px-4 bg-transparent text-gray-500 font-medium text-sm rounded-lg"
                        },
                        onclick: move |_| { tab.set("create"); error_msg.set(String::new()); },
                        "Create Account"
                    }
                }

                div { class: "mb-5",
                    label { class: "block mb-2 text-gray-400 text-xs font-medium", "Username" }
                    input {
                        class: "w-full p-3 rounded-lg border border-gray-800 bg-gray-900 text-gray-100 text-sm",
                        placeholder: "Username",
                        value: "{user_input}",
                        oninput: move |e| user_input.set(e.value()),
                    }
                }

                div { class: "mb-7",
                    label { class: "block mb-2 text-gray-400 text-xs font-medium", "PIN" }
                    input {
                        r#type: "password",
                        class: "w-full p-3 rounded-lg border border-gray-800 bg-gray-900 text-gray-100 text-sm",
                        placeholder: "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}",
                        value: "{pin_input}",
                        oninput: move |e| pin_input.set(e.value()),
                    }
                }

                if !error_msg().is_empty() {
                    div {
                        class: if is_success {
                            "p-3 mb-5 rounded-lg text-sm leading-5 bg-green-100 border border-green-200 text-green-600"
                        } else {
                            "p-3 mb-5 rounded-lg text-sm leading-5 bg-red-100 border border-red-200 text-red-600"
                        },
                        "{error_msg}"
                    }
                }

                button {
                    class: "w-full py-3 bg-indigo-800 text-gray-100 rounded-lg font-bold text-sm",
                    onclick: on_submit,
                    if tab() == "login" { "Sign In" } else { "Create Account" }
                }
            }
        }
    }
}