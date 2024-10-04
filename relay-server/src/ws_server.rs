use crate::top_bid::TopBids;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::State;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;

/// Setup WS Server.
pub fn new<I, A>(api_impl: I) -> Router
where
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: TopBids + 'static,
{
    Router::new()
        .route("/relay/v1/builder/top_bids", get(get_top_bids::<I, A>))
        .with_state(api_impl)
}

/// GetTopBids - GET /relay/v1/builder/top_bids
#[tracing::instrument(skip_all)]
async fn get_top_bids<I, A>(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(api_impl): State<I>,
) -> impl IntoResponse
where
    I: AsRef<A> + Send + Sync + 'static,
    A: TopBids + 'static,
{
    ws.on_upgrade(move |socket| handle_socket(socket, addr, api_impl))
}

async fn handle_socket<I, A>(socket: WebSocket, who: SocketAddr, api_impl: I)
where
    I: AsRef<A> + Send + Sync + 'static,
    A: TopBids,
{
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        let stream = match api_impl.as_ref().get_top_bids().await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to get top bids stream: {:?}", e);
                let _ = sender.close().await;
                return;
            }
        };

        let mut stream = stream;
        while let Some(update) = stream.next().await {
            match serde_json::to_string(&update) {
                Ok(json) => {
                    if let Err(e) = sender.send(Message::Text(json)).await {
                        tracing::error!("Error sending message: {:?}", e);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Error serializing update: {:?}", e);
                    continue;
                }
            }
        }
        let _ = sender.close().await;
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            match message {
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    tracing::info!("Client {} disconnected", who);
}
