use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;

pub fn create_sse_response<S>(
    byte_stream: S,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    let mapped = byte_stream.filter_map(|item| match item {
        Ok(chunk) => {
            let s = String::from_utf8_lossy(&chunk).to_string();
            Some(Ok(Event::default().data(s)))
        }
        Err(_) => None,
    });

    Sse::new(mapped).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}
