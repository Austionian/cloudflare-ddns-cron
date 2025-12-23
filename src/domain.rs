use crate::{CLIENT, Ip, get_api_token};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use tracing::instrument;

#[derive(Deserialize, Debug)]
struct CloudflareResponse {
    result: Option<Vec<CloudflareRecord>>,
}

#[derive(Deserialize, Debug)]
struct CloudflarePatchResponse {
    errors: Vec<CloudflareMessage>,
    success: bool,
}

#[derive(Deserialize, Debug)]
struct CloudflareRecord {
    content: Option<String>,
    id: String,
}

#[derive(Deserialize, Debug)]
struct CloudflareMessage {
    message: String,
}

#[derive(serde::Serialize)]
struct PatchBody {
    r#type: &'static str,
    name: &'static str,
    content: String,
    ttl: u8,
}

pub struct Domain {
    zone_id: String,
    record_id: Option<String>,
    domain: &'static str,
}

impl Domain {
    pub fn new(zone_id: String, domain: &'static str) -> Self {
        Self {
            zone_id,
            domain,
            record_id: None,
        }
    }

    fn get_get_url(&self) -> String {
        format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type=A",
            self.zone_id
        )
    }

    fn get_patch_url(&self) -> Option<String> {
        self.record_id.as_ref().map(|record_id| {
            format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                self.zone_id, record_id
            )
        })
    }

    /// Checks if the A record in Cloudflare matches the IP address of Ip
    #[instrument(name = "Domain::is_same", skip(self, ip))]
    async fn is_same(&mut self, ip: &Ip) -> anyhow::Result<bool> {
        match CLIENT
            .get(self.get_get_url())
            .bearer_auth(get_api_token!()?)
            .send()
            .await
            .context("Failed to GET request to Cloudflare")?
            .error_for_status()
        {
            Ok(response) => {
                let response = response.json::<CloudflareResponse>().await?;

                response
                    .result
                    .ok_or(anyhow!("Empty result from Cloudflare"))?
                    .first() // There should only ever be one A record, so just get the first.
                    .map(|record| {
                        // Update the record id on the domain in case the IP address needs to be
                        // updated.
                        self.record_id = Some(record.id.clone());

                        // Look at the content of the record and compare with ip.
                        record.content.as_ref().map(|content| {
                            tracing::info!("A record's IP is {}", content,);
                            Ok(*content == ip.addr)
                        })
                    })
                    .ok_or(anyhow!("No record found"))?
                    .ok_or(anyhow!("Empty record"))?
            }
            Err(error) => anyhow::bail!("Unable to retrive record: {}", error),
        }
    }

    /// Checks if the existing A record's ip address matches the ip that's given to the function.
    /// If it doesn't match, updates the A record to what was given.
    #[instrument(name = "Domain::ddns", skip(self, ip), fields(domain = %self.domain))]
    pub async fn ddns(&mut self, ip: &Ip) -> anyhow::Result<()> {
        if !self.is_same(ip).await? {
            tracing::info!("Updating {}'s record to {}", self.domain, ip.addr);

            // Update the dns record
            match CLIENT
                .patch(&self.get_patch_url().ok_or(anyhow!("no record id found"))?)
                .bearer_auth(get_api_token!()?)
                .json(&PatchBody {
                    r#type: "A",
                    name: "@", // A record should be set to root
                    ttl: 1,    // Setting to 1 means 'automatic'
                    content: ip.addr.clone(),
                })
                .send()
                .await
                .context("Failed to send PATCH request")?
                .error_for_status()
            {
                Ok(response) => {
                    let response = response.json::<CloudflarePatchResponse>().await?;
                    if !response.success {
                        let error = response
                            .errors
                            .iter()
                            .map(|err| err.message.clone())
                            .collect::<String>();

                        tracing::error!("Failed to update {}: {}", self.domain, error);
                        anyhow::bail!("Failed to update {}", self.domain)
                    } else {
                        tracing::info!("Updated {}'s record to {}", self.domain, ip.addr);
                        Ok(())
                    }
                }
                Err(error) => anyhow::bail!("Failed to update {}: {}", self.domain, error),
            }
        } else {
            tracing::info!("Records matched.");
            Ok(())
        }
    }
}
