
#[grsrpc::service]
pub trait TaskDisplay {
    fn update_task_status(task_id: u32, status: String) -> String;
}

// Recursive expansion of service macro
// =====================================
// use grsrpc::serde::{Serialize, Deserialize};

// #[derive(Serialize, Deserialize)]
// pub enum TaskDisplay {
//     UpdateTaskStatus { task_id: u32, status: String },
// }
// #[derive(grsrpc::serde::Serialize, grsrpc::serde::Deserialize)]
// pub enum TaskDisplayerResponse {
//     UpdateTaskStatus(()),
// }
// pub trait TaskDisplayer {
//     fn update_task_status(&self, task_id: u32, status: String) -> ();
// }
// impl<T> TaskDisplayer for std::sync::Arc<T>
// where
//     T: TaskDisplayer,
// {
//     fn update_task_status(&self, task_id: u32, status: String) -> () {
//         T::update_task_status(self, task_id, status)
//     }
// }
// impl<T> TaskDisplayer for std::boxed::Box<T>
// where
//     T: TaskDisplayer,
// {
//     fn update_task_status(&self, task_id: u32, status: String) -> () {
//         T::update_task_status(self, task_id, status)
//     }
// }
// impl<T> TaskDisplayer for std::rc::Rc<T>
// where
//     T: TaskDisplayer,
// {
//     fn update_task_status(&self, task_id: u32, status: String) -> () {
//         T::update_task_status(self, task_id, status)
//     }
// }
// #[derive(core::clone::Clone)]
// pub struct TaskDisplayerClient {
//     request_tx: grsrpc::futures_channel::mpsc::UnboundedSender<(usize, TaskDisplayerRequest)>,
//     abort_tx: grsrpc::futures_channel::mpsc::UnboundedSender<usize>,
//     callback_map:
//         std::rc::Rc<std::cell::RefCell<grsrpc::client::CallbackMap<TaskDisplayerResponse>>>,
//     seq_id: std::rc::Rc<std::cell::RefCell<usize>>,
// }
// impl std::fmt::Debug for TaskDisplayerClient {
//     fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         formatter.debug_struct("TaskDisplayerClient").finish()
//     }
// }
// impl grsrpc::client::Client for TaskDisplayerClient {
//     type Request = TaskDisplayerRequest;
//     type Response = TaskDisplayerResponse;
// }
// impl From<grsrpc::client::Configuration<TaskDisplayerRequest, TaskDisplayerResponse>>
//     for TaskDisplayerClient
// {
//     fn from(
//         (callback_map, request_tx, abort_tx): grsrpc::client::Configuration<
//             TaskDisplayerRequest,
//             TaskDisplayerResponse,
//         >,
//     ) -> Self {
//         Self {
//             callback_map,
//             request_tx,
//             abort_tx,
//             seq_id: std::default::Default::default(),
//         }
//     }
// }
// impl TaskDisplayerClient {
//     pub fn update_task_status(
//         &self,
//         task_id: u32,
//         status: String,
//     ) -> grsrpc::client::RequestFuture<()> {
//         let __seq_id = self.seq_id.replace_with(|seq_id| seq_id.wrapping_add(1));
//         let __request = TaskDisplayerRequest::UpdateTaskStatus { task_id, status };
//         let (__response_tx, __response_rx) = grsrpc::futures_channel::oneshot::channel();
//         self.callback_map
//             .borrow_mut()
//             .insert(__seq_id, __response_tx);
//         self.request_tx
//             .unbounded_send((__seq_id, __request))
//             .unwrap();
//         let __response_future = grsrpc::futures_util::FutureExt::map(__response_rx, |response| {
//             let response = response.unwrap();
//             let TaskDisplayerResponse::UpdateTaskStatus(__inner) = response else {
//                 {
//                     core::panicking::panic_fmt(core::const_format_args!(
//                         "received incorrect response variant"
//                     ));
//                 }
//             };
//             __inner
//         });
//         let __abort_tx = self.abort_tx.clone();
//         grsrpc::client::RequestFuture::new(
//             __response_future,
//             std::boxed::Box::new(move || (__abort_tx).unbounded_send(__seq_id).unwrap()),
//         )
//     }
// }
// pub struct TaskDisplayerService<T> {
//     server_impl: T,
// }
// impl<T: TaskDisplayer> grsrpc::service::Service for TaskDisplayerService<T> {
//     type Request = TaskDisplayerRequest;
//     type Response = TaskDisplayerResponse;
//     async fn execute(
//         &self,
//         __seq_id: usize,
//         mut __abort_rx: grsrpc::futures_channel::oneshot::Receiver<()>,
//         __request: Self::Request,
//     ) -> (usize, Option<(Self::Response)>) {
//         let __result = match __request {
//             Self::Request::UpdateTaskStatus { task_id, status } => {
//                 let __response = self.server_impl.update_task_status(task_id, status);
//                 Some({ Self::Response::UpdateTaskStatus(__response) })
//             }
//         };
//         (__seq_id, __result)
//     }
// }
// impl<T: TaskDisplayer> std::convert::From<T> for TaskDisplayerService<T> {
//     fn from(server_impl: T) -> Self {
//         Self { server_impl }
//     }
// }

