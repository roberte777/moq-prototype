use anyhow::Result;
use moq_lite::{Client, Origin, Track};
use prost::Message;
use tracing::{debug, info};
use url::Url;
use web_transport_quinn::ClientBuilder;

pub mod telemetry {
    include!(concat!(env!("OUT_DIR"), "/telemetry.rs"));
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::var("RELAY_URL").unwrap_or_else(|_| "https://localhost:4443".to_string());
    let broadcast_path = std::env::var("BROADCAST_PATH").unwrap_or_else(|_| "sensors".to_string());
    let track_name = std::env::var("TRACK_NAME").unwrap_or_else(|_| "temperature".to_string());

    info!(
        relay = %url,
        broadcast = %broadcast_path,
        track = %track_name,
        "Connecting to relay"
    );

    let origin = Origin::produce();

    // Create WebTransport client
    let wt_client = ClientBuilder::new()
        .dangerous()
        .with_no_certificate_verification()?;
    let session = wt_client.connect(url.parse::<Url>()?).await?;

    let client = Client::new().with_consume(origin.producer);
    let _session = client.connect(session).await?;

    let broadcast = origin
        .consumer
        .consume_broadcast(&broadcast_path)
        .expect("broadcast not found");

    let mut track = broadcast.subscribe_track(&Track::new(&track_name));

    info!(
        broadcast = %broadcast_path,
        track = %track_name,
        "Waiting for data"
    );

    while let Ok(Some(mut group)) = track.next_group().await {
        let sequence = group.info.sequence;

        while let Ok(Some(frame)) = group.read_frame().await {
            let data = telemetry::SensorData::decode(frame.as_ref())?;
            debug!(
                sequence = sequence,
                sensor_id = %data.sensor_id,
                temperature = data.temperature,
                timestamp = data.timestamp,
                "Received group"
            );
        }
    }

    info!("Track closed");
    Ok(())
}
