
#[grsrpc::service]
pub trait TaskDisplay {
    fn update_task_status(task_id: u32, status: String) -> String;
    async fn async_fn_test(task_id: u32) -> String;
}

