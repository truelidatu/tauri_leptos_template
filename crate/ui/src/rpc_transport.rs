use std::{pin::Pin, task::{Context, Poll}};

use grsrpc::futures_core::{Stream};
use futures_sink::Sink;
use ws_stream_wasm::{WsErr, WsMessage, WsStream};
pub struct WasmWsTransport(WsStream);

impl WasmWsTransport {
    pub fn new(stream: WsStream) -> Self {
        Self(stream)
    }
}

impl Stream for WasmWsTransport {
    type Item = Vec<u8>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_next(cx) {
                // TODO: consider return error if ws message is text. Current implementation will make down stream error when deserialize which is fine
                Poll::Ready(Some(WsMessage::Text(text))) => Poll::Ready(Some(text.into_bytes())),
                Poll::Ready(Some(WsMessage::Binary(bytes))) => Poll::Ready(Some(bytes)),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

impl Sink<Vec<u8>> for WasmWsTransport {
    type Error = WsErr;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_ready(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            ws_stream.start_send(WsMessage::Binary(item))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_flush(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_close(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}