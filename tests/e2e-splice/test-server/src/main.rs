use zap_server::splice_worker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Worker connects to Splice supervisor via ZAP_SOCKET env var
    let socket_path =
        std::env::var("ZAP_SOCKET").expect("ZAP_SOCKET environment variable not set");

    println!("[test-server] Connecting to Splice at: {}", socket_path);

    // Run the splice worker (handles protocol, dispatch, etc.)
    splice_worker::run(&socket_path).await?;

    Ok(())
}
