use futures::channel::mpsc;

#[derive(Clone)]
pub struct WsHandle {
    pub tx: mpsc::UnboundedSender<String>,
}

impl WsHandle {
    pub fn send(&self, msg: impl Into<String>) {
        if let Err(e) = self.tx.unbounded_send(msg.into()) {
            eprintln!("Send Failed: {e}");
        }
    }
}