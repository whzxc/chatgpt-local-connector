tokio::task_local! { pub static ORIGIN: crate::Value; }
pub fn current_origin() -> crate::Value {
    ORIGIN.try_with(Clone::clone).unwrap_or(crate::Value::Null)
}
