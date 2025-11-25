
#[grsrpc::service]
// #[async_trait::async_trait]
pub trait TaskDisplay {
    fn update_task_status(task_id: u32, status: String) -> String;
    async fn async_fn_test(task_id: u32) -> String;
}


#[async_trait::async_trait]
pub trait Calculator {
    fn add(&self, a: i32, b: i32) -> i32;
    async fn async_with_result(&self, name: String) -> String;
    async fn call_update_task_status(&self) -> ();
}

#[grsrpc::service(mode="multi_thread")]
#[async_trait::async_trait]
pub trait HelloWorldMultiThread {
    fn hello_world(name: String) -> String;
    async fn async_hello_world(name: String) -> String;
}