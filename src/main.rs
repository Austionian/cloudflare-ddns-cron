mod domain;
mod ip;

use anyhow::anyhow;
use domain::Domain;
use ip::Ip;
use reqwest::{self, Client};
use std::sync::LazyLock;

#[macro_export]
macro_rules! get_env {
    ($key:expr) => {{
        std::env::var($key).map_err(|err| anyhow!("getting {}: {err}", $key))
    }};
}

#[macro_export]
macro_rules! get_api_token {
    () => {
        $crate::get_env!("CLOUDFLARE_API_TOKEN")
    };
}

pub static CLIENT: LazyLock<Client> = LazyLock::new(reqwest::Client::new);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let ip = Ip::get().await?;

    tracing::info!(ip = ip.addr, "ip obtained");

    let mut gathering_surf = Domain::new(get_env!("GATHERING_SURF_ZONE_ID")?, "gathering.surf");
    let mut peach_software = Domain::new(
        get_env!("PEACH_SOFTWARE_ZONE_ID")?,
        "thepeachsoftware.company",
    );
    let mut bl0g = Domain::new(get_env!("BL0G_ZONE_ID")?, "r00ks.io");

    let results: [anyhow::Result<(), anyhow::Error>; 3] = tokio::join! {
        gathering_surf.ddns(&ip),
        peach_software.ddns(&ip),
        bl0g.ddns(&ip),
    }
    .into();

    for result in results {
        match result {
            Ok(()) => (),
            Err(err) => tracing::error!("{err}"),
        }
    }

    Ok(())
}
