use futures::{Stream, StreamExt};
use relay_api_types::TopBidUpdate;
use reqwest::Url;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug)]
pub enum WsError {
    InvalidUrl,
    WebSocket(tokio_tungstenite::tungstenite::Error),
    InvalidJson(serde_json::Error, String),
}

impl From<tokio_tungstenite::tungstenite::Error> for WsError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        WsError::WebSocket(e)
    }
}

pub struct RelayWsClient {
    base_url: Url,
}

impl RelayWsClient {
    pub fn new(base_url: Url) -> Self {
        Self { base_url }
    }

    pub async fn subscribe_top_bids(
        &self,
    ) -> Result<impl Stream<Item = Result<TopBidUpdate, WsError>>, WsError> {
        let mut url = self.base_url.clone();
        url.set_path("/relay/v1/builder/top_bids");

        let ws_scheme = match url.scheme() {
            "http" => "ws",
            "https" => "wss",
            _ => return Err(WsError::InvalidUrl),
        };
        url.set_scheme(ws_scheme).map_err(|_| WsError::InvalidUrl)?;

        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(WsError::WebSocket)?;
        let (_, read) = ws_stream.split();

        let stream = read.filter_map(|message| async {
            match message {
                Ok(Message::Text(text)) => match serde_json::from_str::<TopBidUpdate>(&text) {
                    Ok(update) => Some(Ok(update)),
                    Err(e) => Some(Err(WsError::InvalidJson(e, text))),
                },
                Ok(Message::Binary(bin)) => match serde_json::from_slice::<TopBidUpdate>(&bin) {
                    Ok(update) => Some(Ok(update)),
                    Err(e) => {
                        let text = String::from_utf8_lossy(&bin).to_string();
                        Some(Err(WsError::InvalidJson(e, text)))
                    }
                },
                _ => None, // Ignore other message types
            }
        });
        Ok(stream)
    }
}
