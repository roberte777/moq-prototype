use anyhow::Result;
use moq_lite::Track;
use moq_prototype::drone_proto::{self, CommandType, DroneCommand, DronePosition};
use moq_prototype::unit::UnitId;
use moq_prototype::unit_context::UnitContext;
use moq_prototype::unit_map::UnitMap;
use moq_prototype::{COMMAND_TRACK, POSITION_TRACK, connect_bidirectional, control_broadcast_path};
use prost::Message;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const COMMANDS: [CommandType; 4] = [
    CommandType::Goto,
    CommandType::Hover,
    CommandType::Land,
    CommandType::ReturnHome,
];

#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::var("RELAY_URL").unwrap_or_else(|_| "https://localhost:4443".to_string());

    println!("Controller connecting to relay at {url}");

    let (_session, producer, consumer) = connect_bidirectional(&url).await?;

    let producer = Arc::new(producer);
    let unit_map: Arc<UnitMap<UnitContext>> = Arc::new(UnitMap::new());

    let mut drone_announcements = consumer
        .with_root("drone/")
        .expect("drone prefix not authorized");

    println!("Waiting for drones to connect...");

    loop {
        match drone_announcements.announced().await {
            Some((path, Some(broadcast))) => {
                let drone_id = path.to_string();
                let unit_id = UnitId::from(drone_id.clone());
                println!("[+] Drone discovered: {drone_id}");

                if unit_map.get_unit(&unit_id).is_ok() {
                    if let Err(e) = unit_map.remove_unit(&unit_id) {
                        println!("[!] Failed to remove unit {drone_id}: {e}");
                    }
                    continue;
                }

                // Create the control broadcast and command track for this drone
                let control_path = control_broadcast_path(&drone_id);
                let mut cmd_broadcast = producer
                    .create_broadcast(&control_path)
                    .expect("failed to create control broadcast");
                let track = cmd_broadcast.create_track(Track::new(COMMAND_TRACK));

                // Create and insert the unit context
                let unit_context = UnitContext::new(cmd_broadcast, track);
                if let Err(e) = unit_map.insert_unit(unit_id, unit_context) {
                    println!("[!] Failed to insert unit {drone_id}: {e}");
                    continue;
                }
                println!("[*] Created command channel for drone {drone_id}");

                // Spawn telemetry reader for this drone.
                let reader_id = drone_id.clone();
                tokio::spawn(async move {
                    let mut track = broadcast.subscribe_track(&Track::new(POSITION_TRACK));
                    loop {
                        match track.next_group().await {
                            Ok(Some(mut group)) => {
                                while let Ok(Some(frame)) = group.read_frame().await {
                                    match DronePosition::decode(frame.as_ref()) {
                                        Ok(pos) => {
                                            println!(
                                                "[RX {reader_id}] lat={:.6} lon={:.6} alt={:.1}m",
                                                pos.latitude, pos.longitude, pos.altitude_m,
                                            );
                                        }
                                        Err(e) => {
                                            println!("[RX {reader_id}] decode error: {e}");
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                println!("[-] Drone {reader_id} position track closed");
                                break;
                            }
                            Err(e) => {
                                println!("[!] Drone {reader_id} position error: {e}");
                                break;
                            }
                        }
                    }
                });

                // Spawn random command sender for this drone.
                let sender_unit_map = Arc::clone(&unit_map);
                let sender_unit_id = UnitId::from(drone_id.clone());
                tokio::spawn(async move {
                    let mut rng = StdRng::from_os_rng();
                    loop {
                        tokio::time::sleep(Duration::from_secs(rng.random_range(2..6))).await;

                        let cmd_type = COMMANDS[rng.random_range(0..COMMANDS.len())];
                        let cmd = DroneCommand {
                            drone_id: drone_id.clone(),
                            command: cmd_type.into(),
                            target_lat: rng.random_range(37.0..38.0),
                            target_lon: rng.random_range(-123.0..-122.0),
                            target_alt_m: rng.random_range(50.0..500.0),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };

                        if let Err(e) = send_command(&sender_unit_map, &sender_unit_id, cmd) {
                            println!("[!] Failed to send command to {drone_id}: {e}");
                            break;
                        }
                    }
                });
            }
            Some((path, None)) => {
                let drone_id = path.to_string();
                let unit_id = UnitId::from(drone_id.as_str());
                println!("[-] Drone departed: {drone_id}");
                if let Err(e) = unit_map.remove_unit(&unit_id) {
                    println!("[!] Failed to remove departed drone {drone_id}: {e}");
                }
            }
            None => {
                println!("Announcement stream closed");
                break;
            }
        }
    }

    Ok(())
}

fn send_command(
    unit_map: &UnitMap<UnitContext>,
    unit_id: &UnitId,
    cmd: DroneCommand,
) -> Result<()> {
    let unit_ref = unit_map
        .get_unit(unit_id)
        .map_err(|e| anyhow::anyhow!("command track should exist for discovered drone: {e}"))?;

    let cmd_type = drone_proto::CommandType::try_from(cmd.command).unwrap();
    let mut buf = Vec::with_capacity(cmd.encoded_len());
    cmd.encode(&mut buf).expect("failed to encode command");

    unit_ref
        .view(|ctx| {
            ctx.write_command_frame(buf);
            println!("[TX] sent {cmd_type:?} to drone {unit_id}");
        })
        .map_err(|e| anyhow::anyhow!("unit context no longer valid: {e}"))?;

    Ok(())
}
