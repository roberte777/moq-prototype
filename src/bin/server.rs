use anyhow::Result;
use async_stream::stream;
use moq_lite::{BroadcastConsumer, OriginProducer, Track, TrackProducer};
use moq_prototype::drone::DroneSessionMap;
use moq_prototype::drone_proto::DronePosition;
use moq_prototype::grpc::{self, EchoServiceClient};
use moq_prototype::unit_context::UnitContext;
use moq_prototype::unit_map::UnitMap;
use moq_prototype::{PRIMARY_TRACK, connect_bidirectional, echo_broadcast_path};
use prost::Message;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

const GRPC_ADDR: &str = "[::1]:50051";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let url = std::env::var("RELAY_URL").unwrap_or_else(|_| "https://localhost:4443".to_string());

    let unit_map: Arc<UnitMap<UnitContext>> = Arc::new(UnitMap::new());
    let session_map: Arc<DroneSessionMap> = Arc::new(DroneSessionMap::new());

    let grpc_addr = GRPC_ADDR.parse()?;
    let server_unit_map = Arc::clone(&unit_map);
    let server_session_map = Arc::clone(&session_map);
    tokio::spawn(async move {
        if let Err(e) = grpc::start_server(grpc_addr, server_unit_map, server_session_map).await {
            error!("gRPC server error: {e}");
        }
    });

    // Wait for server to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    info!("Server connecting to relay at {url}");

    let (_session, producer, consumer) = connect_bidirectional(&url).await?;
    let producer = Arc::new(producer);

    let mut drone_announcements = consumer
        .with_root("drone/")
        .expect("drone prefix not authorized");

    info!("Waiting for drones to connect...");

    loop {
        match drone_announcements.announced().await {
            Some((path, Some(broadcast))) => {
                let drone_id = path.to_string();
                info!(drone_id = %drone_id, "Drone discovered");

                spawn_drone_bridge(drone_id.clone(), broadcast, Arc::clone(&producer));
            }

            // Drone disconnects
            Some((path, None)) => {
                let drone_id = path.to_string();
                info!(drone_id = %drone_id, "Drone departed");
                // stuff cleans up when streams start closing
            }

            None => {
                info!("Announcement stream closed");
                break;
            }
        }
    }

    Ok(())
}

fn spawn_drone_bridge(
    drone_id: String,
    broadcast: BroadcastConsumer,
    producer: Arc<OriginProducer>,
) {
    tokio::spawn(async move {
        // FIXME: how tf do I report errors back to the drone
        if let Err(e) = run_drone_bridge(drone_id.clone(), broadcast, producer).await {
            error!(drone_id = %drone_id, error = %e, "Bridge error");
        }
    });
}

async fn run_drone_bridge(
    drone_id: String,
    broadcast: BroadcastConsumer,
    producer: Arc<OriginProducer>,
) -> Result<()> {
    // create the broadcasts so the bidirectoinal comms are open.
    let mut client = EchoServiceClient::connect(format!("http://{GRPC_ADDR}")).await?;
    let mut track = broadcast.subscribe_track(&Track::new(PRIMARY_TRACK));

    let echo_broadcast_path = echo_broadcast_path(&drone_id);
    let mut echo_broadcast = producer
        .create_broadcast(&echo_broadcast_path)
        .ok_or_else(|| anyhow::anyhow!("Failed to create echo broadcast on server"))?;
    let mut echo_track: TrackProducer = echo_broadcast.create_track(Track::new(PRIMARY_TRACK));

    let drone_id_clone = drone_id.clone();

    let stream = stream! {
        loop {
            match track.next_group().await {
                Ok(Some(mut group)) => {
                    while let Ok(Some(frame)) = group.read_frame().await {
                        if let Ok(pos) = DronePosition::decode(frame.as_ref()) {
                            debug!(
                                drone_id = %drone_id_clone,
                                lat = pos.latitude,
                                lon = pos.longitude,
                                alt = pos.altitude_m,
                                "Received position"
                            );
                            yield pos;
                        }
                    }
                }
                Ok(None) => {
                    info!(drone_id = %drone_id_clone, "Telemetry stream closed");
                    break;
                }
                Err(e) => {
                    warn!(drone_id = %drone_id_clone, error = %e, "Telemetry stream error");
                    break;
                }
            }
        }
    };
    let response = client.echo(stream).await?;
    let mut echo_stream = response.into_inner();

    info!(drone_id = %drone_id, "Bridge established");

    while let Some(pos) = echo_stream.message().await? {
        info!(drone_id = %drone_id, position = ?pos, "Echoing position");
        let mut buf = Vec::with_capacity(pos.encoded_len());
        pos.encode(&mut buf)?;
        echo_track.write_frame(buf);
    }

    info!(drone_id = %drone_id, "Bridge closed");

    Ok(())
}
