use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use std::convert::Infallible;
use std::sync::{Arc, LazyLock};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

static LOG_CHANNEL: LazyLock<Arc<broadcast::Sender<String>>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(2048);
    Arc::new(tx)
});

pub fn broadcast_log(msg: String) {
    let _ = LOG_CHANNEL.send(msg);
}

pub struct BroadcastLogLayer;

impl<S: Subscriber> Layer<S> for BroadcastLogLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);
        let level = event.metadata().level();
        let target = event.metadata().target();
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("{} [{:>5}] [{}] {}", timestamp, level, target, visitor.message);
        broadcast_log(line);
    }
}

#[derive(Default)]
struct LogVisitor {
    message: String,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let s = format!("{:?}", value);
            self.message = if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                s[1..s.len() - 1].to_string()
            } else {
                s
            };
        } else {
            if !self.message.is_empty() {
                self.message.push(' ');
            }
            self.message.push_str(&format!("{}={:?}", field.name(), value));
        }
    }
}

pub async fn stream_console_logs() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = LOG_CHANNEL.subscribe();
    let broadcast_stream = BroadcastStream::new(rx);

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let init_line = format!(
        "{} [ INFO] [9router::gateway] Live console stream connected. Native Rust gateway ready.",
        now
    );

    let stream = futures_util::stream::once(async move {
        Ok(Event::default().data(init_line))
    })
    .chain(broadcast_stream.filter_map(|res| async move {
        match res {
            Ok(msg) => Some(Ok(Event::default().data(msg))),
            Err(_) => None,
        }
    }));

    Sse::new(stream).keep_alive(KeepAlive::default())
}
