use crate::CLIENT;
use tracing::instrument;

pub struct Ip {
    pub addr: String,
}

impl Ip {
    #[instrument(name = "Ip::get")]
    pub async fn get() -> anyhow::Result<Self> {
        tracing::info!("getting ip");
        let ipify = CLIENT.get("https://api.ipify.org").send();
        let hazip = CLIENT.get("https://ipv4.icanhazip.com").send();
        let ipinfo = CLIENT.get("https://ipinfo.io/ip").send();

        let ip = tokio::select! {
            ip = ipify => ip?.text(),
            ip = hazip => ip?.text(),
            ip = ipinfo => ip?.text(),
        }
        .await?;

        Ok(Self {
            addr: ip.trim().to_string(),
        })
    }
}
