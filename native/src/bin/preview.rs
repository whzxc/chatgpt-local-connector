// Development-only read bridge. This binary is never bundled in the desktop app.
#[tokio::main]
async fn main() -> Result<(), String> {
    let service = connector_core::service::Service::new()?;
    let task = connector_core::transport::listen_mode(service.clone(), true).await?;
    println!(
        "{}",
        serde_json::json!({"port":service.port.load(std::sync::atomic::Ordering::SeqCst),"token":service.token})
    );
    tokio::signal::ctrl_c().await.map_err(|e| e.to_string())?;
    task.abort();
    Ok(())
}
