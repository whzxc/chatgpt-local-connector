//! OS login registration belongs to the Desktop executable.
use connector_core::{output, private_dir, root, Result};
pub async fn enabled() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    #[cfg(target_os = "macos")]
    {
        let Some(home) = dirs::home_dir() else {
            return false;
        };
        let path = home.join("Library/LaunchAgents/com.whzxc.chatgpt-local-connector.plist");
        let path = path.to_string_lossy();
        let program = output(
            "plutil",
            &["-extract", "ProgramArguments.0", "raw", "-o", "-", &path],
        )
        .await;
        let state = output(
            "plutil",
            &[
                "-extract",
                "EnvironmentVariables.CLC_STATE_DIR",
                "raw",
                "-o",
                "-",
                &path,
            ],
        )
        .await;
        return program.is_ok_and(|s| s.trim() == exe.to_string_lossy())
            && state.is_ok_and(|s| s.trim() == root().to_string_lossy());
    }
    #[cfg(windows)]
    {
        return output(
            "schtasks.exe",
            &["/Query", "/TN", "ChatGPT Local Connector", "/XML"],
        )
        .await
        .is_ok_and(|s| {
            s.contains(&xml_escape(&exe.to_string_lossy()))
                && s.contains("--autostart")
                && s.contains(&xml_escape(&root().to_string_lossy()))
        });
    }
    #[allow(unreachable_code)]
    false
}
#[cfg(windows)]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub async fn set(enabled: bool) -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    {
        let path = dirs::home_dir()
            .unwrap()
            .join("Library/LaunchAgents/com.whzxc.chatgpt-local-connector.plist");
        if enabled {
            let xml = |v: &str| {
                v.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
            };
            private_dir(path.parent().unwrap())?;
            std::fs::write(path,format!("<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>Label</key><string>com.whzxc.chatgpt-local-connector</string><key>ProgramArguments</key><array><string>{}</string><string>--autostart</string></array><key>EnvironmentVariables</key><dict><key>CLC_STATE_DIR</key><string>{}</string></dict><key>RunAtLoad</key><true/></dict></plist>",xml(&exe.to_string_lossy()),xml(&root().to_string_lossy()))).map_err(|e|e.to_string())?;
        } else if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    #[cfg(windows)]
    {
        if enabled {
            output(
                "schtasks.exe",
                &[
                    "/Create",
                    "/TN",
                    "ChatGPT Local Connector",
                    "/SC",
                    "ONLOGON",
                    "/TR",
                    &format!(
                        "\"{}\" --autostart --state-dir \"{}\"",
                        exe.display(),
                        root().display()
                    ),
                    "/F",
                ],
            )
            .await?;
        } else if output(
            "schtasks.exe",
            &["/Query", "/TN", "ChatGPT Local Connector"],
        )
        .await
        .is_ok()
        {
            output(
                "schtasks.exe",
                &["/Delete", "/TN", "ChatGPT Local Connector", "/F"],
            )
            .await?;
        }
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("不支持开机启动".into())
}
