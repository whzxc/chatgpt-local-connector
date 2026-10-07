use connector_core::{runtime, Result};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["stdio"] => connector_core::transport::stdio().await,
        ["serve"] => runtime::serve().await,
        ["cli", rest @ ..] => {
            std::process::exit(
                connector_core::cli::run(rest.iter().map(|s| s.to_string()).collect()).await,
            );
        }
        ["--version"] => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err("Usage: local-connector [stdio | serve | cli <command> | --version]".into()),
    }
}
