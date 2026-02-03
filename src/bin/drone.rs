use anyhow::{Result, anyhow};
use moq_lite::{
    BroadcastConsumer, BroadcastProducer, OriginConsumer, Track, TrackConsumer, TrackProducer,
};
use moq_prototype::drone_proto::{self, DronePosition};
use moq_prototype::{
    COMMAND_TRACK, POSITION_TRACK, connect_bidirectional, control_broadcast_path,
    drone_broadcast_path,
};
use prost::Message;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::var("RELAY_URL").unwrap_or_else(|_| "https://localhost:4443".to_string());
    let drone_id = std::env::var("DRONE_ID").unwrap_or_else(|_| Uuid::new_v4().to_string());

    let drone_path = drone_broadcast_path(&drone_id);
    let control_path = control_broadcast_path(&drone_id);

    info!(
        drone_id = %drone_id,
        relay = %url,
        position_path = %format!("{drone_path}/{POSITION_TRACK}"),
        command_path = %format!("{control_path}/{COMMAND_TRACK}"),
        "Drone connecting to relay"
    );

    let (_session, producer, mut consumer) = connect_bidirectional(&url).await?;

    let mut broadcast = producer
        .create_broadcast(&drone_path)
        .expect("failed to create drone broadcast");

    let (cmd_broadcast, mut position_track, mut cmd_track) =
        connect_drone(&mut broadcast, &mut consumer, &control_path)
            .await
            .expect("Not able to connect drone");

    info!(drone_id = %drone_id, "Drone is online");

    let mut ticker = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let pos = DronePosition {
                    drone_id: drone_id.clone(),
                    latitude: 37.7749,
                    longitude: -122.4194,
                    altitude_m: 100.0,
                    heading_deg: 0.0,
                    speed_mps: 0.0,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };

                let mut buf = Vec::with_capacity(pos.encoded_len());
                pos.encode(&mut buf)?;
                position_track.write_frame(buf);

                debug!(
                    lat = pos.latitude,
                    lon = pos.longitude,
                    alt = pos.altitude_m,
                    "Sent position"
                );
            }

            result = cmd_track.next_group() => {
                match result {
                    Ok(Some(mut group)) => {
                        while let Ok(Some(frame)) = group.read_frame().await {
                            let cmd = drone_proto::DroneCommand::decode(frame.as_ref())?;
                            debug!(
                                command = ?drone_proto::CommandType::try_from(cmd.command)
                                    .unwrap_or(drone_proto::CommandType::Hover),
                                target_lat = cmd.target_lat,
                                target_lon = cmd.target_lon,
                                target_alt = cmd.target_alt_m,
                                "Received command"
                            );
                        }
                    }
                    Ok(None) => {
                        info!("Command track closed");
                        break;
                    }
                    Err(e) => {
                        warn!(error = %e, "Command track error, retrying");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        cmd_track = cmd_broadcast.subscribe_track(&Track::new(COMMAND_TRACK));
                    }
                }
            }
        }
    }

    Ok(())
}

async fn connect_drone(
    broadcast: &mut BroadcastProducer,
    consumer: &mut OriginConsumer,
    control_path: &str,
) -> Result<(BroadcastConsumer, TrackProducer, TrackConsumer)> {
    let position_track = broadcast.create_track(Track::new(POSITION_TRACK));
    let cmd_broadcast: BroadcastConsumer = loop {
        match consumer.announced().await {
            Some((ref p, Some(bc))) if p.as_str() == control_path => {
                break Ok(bc);
            }
            Some((ref p, None)) if p.as_str() == control_path => {
                break Err(anyhow!("Control path found but value is None"));
            }
            Some(_) => continue,
            None => break Err(anyhow!("Control path not found")),
        }
    }?;
    let cmd_track = cmd_broadcast.subscribe_track(&Track::new(COMMAND_TRACK));
    Ok((cmd_broadcast, position_track, cmd_track))
}
