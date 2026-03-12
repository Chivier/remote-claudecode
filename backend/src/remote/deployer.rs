use tokio::process::Command;

use crate::db::servers::Server;

/// Deploy or update the broker binary on a remote server
pub async fn deploy_broker(
    server: &Server,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let target = if server.ssh_user.is_empty() {
        server.hostname.clone()
    } else {
        format!("{}@{}", server.ssh_user, server.hostname)
    };

    let mut ssh_args = vec![
        "-p".to_string(),
        server.ssh_port.to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
    ];

    if let Some(ref key_path) = server.ssh_key_path {
        ssh_args.push("-i".to_string());
        ssh_args.push(key_path.clone());
    }

    ssh_args.push(target.clone());

    // Check if broker already exists
    let check_output = Command::new("ssh")
        .args(&ssh_args)
        .arg("which cloudcli-broker 2>/dev/null || echo 'NOT_FOUND'")
        .output()
        .await?;

    let check_result = String::from_utf8_lossy(&check_output.stdout);
    let broker_exists = !check_result.contains("NOT_FOUND");

    if broker_exists {
        // Update existing broker
        tracing::info!("Broker already installed on {}, checking version", target);
        let version_output = Command::new("ssh")
            .args(&ssh_args)
            .arg("cloudcli-broker --version 2>/dev/null || echo 'unknown'")
            .output()
            .await?;
        let version = String::from_utf8_lossy(&version_output.stdout)
            .trim()
            .to_string();
        return Ok(format!("Broker already installed (version: {})", version));
    }

    // Deploy broker via install script
    // In production, this would download the binary for the remote architecture
    let install_cmd = r#"
        ARCH=$(uname -m)
        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        mkdir -p ~/.local/bin
        echo "Would download cloudcli-broker for ${OS}-${ARCH}"
        echo "Broker deployment placeholder - implement actual download URL"
    "#;

    let deploy_output = Command::new("ssh")
        .args(&ssh_args)
        .arg(install_cmd)
        .output()
        .await?;

    if !deploy_output.status.success() {
        let stderr = String::from_utf8_lossy(&deploy_output.stderr);
        return Err(format!("Deployment failed: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&deploy_output.stdout)
        .trim()
        .to_string();
    Ok(stdout)
}

/// Test SSH connectivity to a remote server
pub async fn test_ssh_connection(
    server: &Server,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let target = if server.ssh_user.is_empty() {
        server.hostname.clone()
    } else {
        format!("{}@{}", server.ssh_user, server.hostname)
    };

    let mut args = vec![
        "-p".to_string(),
        server.ssh_port.to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
    ];

    if let Some(ref key_path) = server.ssh_key_path {
        args.push("-i".to_string());
        args.push(key_path.clone());
    }

    args.push(target);
    args.push("echo 'SSH_OK' && uname -a".to_string());

    let output = Command::new("ssh").args(&args).output().await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("SSH connection failed: {}", stderr).into())
    }
}
