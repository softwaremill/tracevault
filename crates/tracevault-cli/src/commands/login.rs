use crate::api_client::ApiClient;
use crate::credentials::Credentials;

pub async fn login(server_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(server_url, None);

    // Start device auth flow
    let device = client.device_start().await?;

    // Always build the browser URL from server_url.
    // In production, a reverse proxy routes /api/* to the backend and
    // everything else to the SvelteKit frontend — same domain.
    let full_url = format!(
        "{}/auth/device?token={}",
        server_url.trim_end_matches('/'),
        device.token
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
                    let token = status
                        .token
                        .ok_or("Server approved but did not return a session token")?;
                    let email = status
                        .email
                        .ok_or("Server approved but did not return an email")?;
                    let org_name = status.org_name.ok_or(
                        "You are not a member of any organization. \
                         Please log in via the web interface to create or join an organization first."
                    )?;

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
