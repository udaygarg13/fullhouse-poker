use dioxus::prelude::*;

#[component]
pub fn ConnectScreen(
    on_connect: EventHandler<String>,
) -> Element {
    let mut server_input = use_signal(|| "127.0.0.1:7878".to_string());
    let mut error_msg = use_signal(|| String::new());

    let on_submit = move |_| {
        let raw = server_input().trim().to_string();
        if raw.is_empty() {
            error_msg.set("Please enter a server address.".into());
            return;
        }
        error_msg.set(String::new());
        on_connect.call(raw);
    };

    let on_keydown = move |e: KeyboardEvent| {
        if e.key() == Key::Enter {
            let raw = server_input().trim().to_string();
            if raw.is_empty() {
                error_msg.set("Please enter a server address.".into());
                return;
            }
            error_msg.set(String::new());
            on_connect.call(raw);
        }
    };

    rsx! {
        div {
            class: "flex flex-col items-center justify-center h-screen bg-gray-950 font-sans relative overflow-hidden",

            div { class: "text-center mb-10 relative",
                h1 { class: "text-4xl font-bold mb-1 text-gray-200 uppercase", "Poker" }
                p { class: "text-gray-500 text-xs tracking-widest uppercase", "Team Fullhouse" }
            }

            div {
                class: "bg-gray-900 p-8 rounded-lg w-[380px] border border-gray-800 relative",

                div { class: "mb-8",
                    h2 { class: "text-gray-200 font-semibold text-base mb-1", "Connect to Server" }
                    p { class: "text-gray-500 text-xs", "Enter the server address to join." }
                }

                div { class: "mb-7",
                    label { class: "block mb-2 text-gray-400 text-xs font-medium", "Server Address" }
                    input {
                        class: "w-full p-3 rounded-lg border border-gray-800 bg-gray-900 text-gray-100 text-sm",
                        placeholder: "127.0.0.1:7878",
                        value: "{server_input}",
                        oninput: move |e| server_input.set(e.value()),
                        onkeydown: on_keydown,
                    }
                }

                if !error_msg().is_empty() {
                    div {
                        class: "p-3 mb-5 rounded-lg text-sm leading-5 bg-red-100 border border-red-200 text-red-600",
                        "{error_msg}"
                    }
                }

                button {
                    class: "w-full py-3 bg-indigo-800 text-gray-100 rounded-lg font-bold text-sm",
                    onclick: on_submit,
                    "Connect"
                }
            }
        }
    }
}