use crate::api_client::ApiClient;
use crate::credentials::Credentials;

pub async fn login(server_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(server_url, None);

    // Start device auth flow
    let device = client.device_start().await?;
    let full_url = format!(
        "{}{}",
        server_url.trim_end_matches('/'),
        device.verification_url
    );

    println!("Opening browser for authentication...");
    println!("If the browser doesn't open, visit: {full_url}");

    // Open browser
    if let Err(e) = open::that(&full_url) {
        eprintln!("Could not open browser: {e}");
    }

    // Poll for approval
    print!("Waiting for authentication...");
    let poll_interval = std::time::Duration::from_secs(2);
    let max_attempts = 150; // 5 minutes at 2s intervals

    for _ in 0..max_attempts {
        tokio::time::sleep(poll_interval).await;

        match client.device_status(&device.token).await {
            Ok(status) => {
                if status.status == "approved" {
                    if let (Some(token), Some(email), Some(org_name)) =
                        (status.token, status.email, status.org_name)
                    {
                        println!(" done!");
                        println!();
                        println!("Logged in as {} (org: {})", email, org_name);

                        let creds = Credentials {
                            server_url: server_url.to_string(),
                            token,
                            email,
                            org_name,
                        };
                        creds.save()?;
                        println!("Credentials saved to {}", Credentials::path().display());
                        return Ok(());
                    }
                }
                // Still pending, continue polling
            }
            Err(e) => {
                eprintln!("\nError polling status: {e}");
                return Err(e);
            }
        }
    }

    Err("Authentication timed out after 5 minutes".into())
}
