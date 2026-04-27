fn main() {
    dotenvy::dotenv().ok();

    println!(
        "cargo:rustc-env=SERVER_URL={}",
        std::env::var("SERVER_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:7878".to_string())
    );
}