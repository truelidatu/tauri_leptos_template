use std::{
    cell::RefCell,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use futures_core::{Stream, future::LocalBoxFuture};
use futures_util::StreamExt;
use futures_util::{FutureExt, Sink};
use futures_util::{SinkExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
pub mod client;

#[doc(hidden)]
pub use bincode;
#[doc(hidden)]
pub use futures_channel;
#[doc(hidden)]
pub use futures_core;
#[doc(hidden)]
pub use futures_util;
// #[doc(hidden)]
// pub use pin_utils;
#[doc(hidden)]
pub use serde;

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub enum Message<Request, Response> {
    Request(usize, Request),
    Abort(usize),
    Response(usize, Response),
}

pub struct Builder<C, S, T> {
    client: PhantomData<C>,
    service: S,
    transport: T,
}

impl<T> Builder<(), (), T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            client: PhantomData::<()>,
            service: (),
        }
    }
}

impl<S, T> Builder<(), S, T> {
    pub fn with_client<C: client::Client>(self) -> Builder<C, S, T> {
        let Builder {
            transport, service, ..
        } = self;
        Builder {
            transport,
            client: PhantomData::<C>,
            service,
        }
    }
}

impl<C, T, E> Builder<C, (), T>
where
    C: client::Client + From<client::Configuration<C::Request, C::Response>> + 'static,
    <C as client::Client>::Response: DeserializeOwned,
    <C as client::Client>::Request: Serialize,
    T: Stream<Item = Vec<u8>> + Sink<Vec<u8>, Error = E> + Unpin + 'static,
{
    /// Build function for client-only RPC interfaces.
    pub fn build(self) -> (C, RpcEngine) {
        let client_callback_map: Rc<RefCell<client::CallbackMap<C::Response>>> = Default::default();
        // let (abort_requests_tx, abort_requests_rx) = futures_channel::mpsc::unbounded();
        let client_callback_map_cloned = client_callback_map.clone();
        let (mut transport_tx, mut transport_rx) = self.transport.split();
        let transport_incoming = async move {
            while let Some(array) = transport_rx.next().await {
                let message = array;
                match bincode::deserialize::<Message<(), C::Response>>(&message).unwrap() {
                    Message::Response(seq_id, response) => {
                        if let Some(callback_tx) =
                            client_callback_map_cloned.borrow_mut().remove(&seq_id)
                        {
                            // TODO: handle error
                            let _ = callback_tx.send(response);
                        }
                    }
                    _ => panic!("client received a server message"),
                }
            }
        }
        .boxed_local();

        let (request_tx, mut request_rx) = futures_channel::mpsc::unbounded::<(usize, C::Request)>();
        let (abort_tx, mut abort_rx) = futures_channel::mpsc::unbounded::<usize>();
        let transport_outgoing = async move {
            futures_util::select! {
                maybe_req = request_rx.next() => {
                    if let Some((seq_id, msg)) = maybe_req {
                        // TODO: optimize, serialize on client rpc fn to avoid copy?
                        let msg = bincode::serialize(&Message::<<C as client::Client>::Request, ()>::Request(seq_id, msg)).unwrap();
                        let _ = transport_tx.send(msg);
                    }
                }
                maybe_abort = abort_rx.next() => {
                    if let Some(seq_id) = maybe_abort {
                        let msg = bincode::serialize(&Message::<<C as client::Client>::Request, ()>::Abort(seq_id)).unwrap();
                        let _ = transport_tx.send(msg);
                    }
                }
            }
        }
        .boxed_local();
        
        let client = C::from((client_callback_map, request_tx, abort_tx));
        let server = RpcEngine {
            task: task(transport_incoming, transport_outgoing).boxed_local(),
        };
        (client, server)
    }
}

pub struct RpcEngine {
    task: LocalBoxFuture<'static, ()>,
}

impl Future for RpcEngine {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.task.poll_unpin(cx)
    }
}

async fn task(
    transport_incoming: LocalBoxFuture<'static, ()>,
    transport_outgoing: LocalBoxFuture<'static, ()>,
    // mut abort_requests_rx: futures_channel::mpsc::UnboundedReceiver<usize>,
) {
    let mut task_list: FuturesUnordered<_> = Default::default();
    task_list.push(transport_incoming);
    task_list.push(transport_outgoing);
    // task_list.push(
    //     async {
    //         while let Some(abort_request) = abort_requests_rx.next().await {
    //             // if let Some(abort_tx) = server_tasks.remove(&seq_id) {
    //             //     let _ = abort_tx.send(());
    //             // }
    //         }
    //     }
    //     .boxed_local(),
    // );
    loop {
        let Some(_) = task_list.next().await else {
            break;
        };
        // futures_util::select! {
        //     _ = transport_incoming => {},
        //     abort_request = abort_requests_rx.next() => {
        //         if let Some(seq_id) = abort_request {
        //             // if let Some(abort_tx) = server_tasks.remove(&seq_id) {
        //             //     let _ = abort_tx.send(());
        //             // }
        //         }
        //     },
        // }
    }
}
