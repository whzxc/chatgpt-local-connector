use connector_core::{plugin, runtime, Result};

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
        [] | ["mcp"] => plugin::stdio().await,
        ["stdio"] => connector_core::transport::stdio().await,
        ["serve"] => runtime::serve().await,
        ["export", directory] => {
            let result = plugin::export(std::path::Path::new(directory))?;
            println!("{result}");
            Ok(())
        }
        ["cli", rest @ ..] => {
            std::process::exit(
                connector_core::cli::run(rest.iter().map(|s| s.to_string()).collect()).await,
            );
        }
        ["--version"] => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err(
            "Usage: local-connector [mcp | stdio | serve | export <directory> | cli <command> | --version]"
                .into(),
        ),
    }
}
