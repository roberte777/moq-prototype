use anyhow::Result;
use moq_lite::{BroadcastConsumer, OriginProducer, Track, TrackProducer};
use moq_prototype::drone::DroneSessionMap;
use moq_prototype::drone_proto::{CommandType, DroneCommand, DronePosition};
use moq_prototype::state_machine::telemetry::Position;
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

const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::var("RELAY_URL").unwrap_or_else(|_| "https://localhost:4443".to_string());

    println!("Controller connecting to relay at {url}");

    let (_session, producer, consumer) = connect_bidirectional(&url).await?;

    let producer = Arc::new(producer);

    let unit_map: Arc<UnitMap<UnitContext>> = Arc::new(UnitMap::new());

    let session_map: Arc<DroneSessionMap> = Arc::new(DroneSessionMap::new());

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

                // ensure unit exists in UnitMap (insert if first time, otherwise reuse)
                if unit_map.get_unit(&unit_id).is_err() {
                    let context = UnitContext::new();
                    if let Err(e) = unit_map.insert_unit(unit_id.clone(), context) {
                        println!("[!] Failed to insert unit {drone_id}: {e}");
                        continue;
                    }
                    println!("[*] Created unit entry for drone {drone_id}");
                } else {
                    println!("[*] Reusing existing unit entry for drone {drone_id}");
                }

                // FIXME: How can I let the drone know there has been an error?
                // create session (error if already active - prevents duplicate handling)
                match session_map.create_session(&unit_id) {
                    Ok(session_id) => {
                        println!("[*] Session created for drone {drone_id}: {session_id}");
                    }
                    Err(e) => {
                        println!("[!] {e}");
                        continue; // Don't spawn tasks for duplicate sessions
                    }
                }

                spawn_telemetry_reader(
                    Arc::clone(&unit_map),
                    Arc::clone(&session_map),
                    unit_id.clone(),
                    broadcast,
                );

                spawn_command_writer(
                    Arc::clone(&unit_map),
                    Arc::clone(&session_map),
                    Arc::clone(&producer),
                    unit_id.clone(),
                );

                spawn_command_generator(
                    Arc::clone(&unit_map),
                    Arc::clone(&session_map),
                    unit_id.clone(),
                );
            }

            // Drone disconnects
            // The second announce is always a disconnect. It will also not have
            // a broadcast consumer, hence the None here.
            Some((path, None)) => {
                let drone_id = path.to_string();
                let unit_id = UnitId::from(drone_id.as_str());
                println!("[-] Drone departed: {drone_id}");

                match session_map.remove_session(&unit_id) {
                    Ok(session) => {
                        println!(
                            "[*] Session ended for drone {drone_id}: {}",
                            session.session_id
                        );
                    }
                    Err(e) => {
                        println!("[!] {e}");
                    }
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

fn spawn_telemetry_reader(
    unit_map: Arc<UnitMap<UnitContext>>,
    session_map: Arc<DroneSessionMap>,
    unit_id: UnitId,
    broadcast: BroadcastConsumer,
) {
    let drone_id = unit_id.as_str().to_string();

    tokio::spawn(async move {
        let mut track = broadcast.subscribe_track(&Track::new(POSITION_TRACK));

        loop {
            // Check if session is still active
            if !session_map.has_active_session(&unit_id) {
                println!("[*] Telemetry reader stopping - session ended for {drone_id}");
                break;
            }

            match track.next_group().await {
                Ok(Some(mut group)) => {
                    while let Ok(Some(frame)) = group.read_frame().await {
                        match DronePosition::decode(frame.as_ref()) {
                            Ok(pos) => {
                                let position = Position {
                                    drone_id: pos.drone_id.clone(),
                                    latitude: pos.latitude,
                                    longitude: pos.longitude,
                                    altitude_m: pos.altitude_m,
                                    heading_deg: pos.heading_deg,
                                    speed_mps: pos.speed_mps,
                                    timestamp: pos.timestamp,
                                };

                                if let Ok(unit_ref) = unit_map.get_unit(&unit_id) {
                                    let _ = unit_ref.view(|ctx| {
                                        ctx.update_telemetry(position);
                                    });
                                }

                                println!(
                                    "[RX {drone_id}] lat={:.6} lon={:.6} alt={:.1}m",
                                    pos.latitude, pos.longitude, pos.altitude_m,
                                );
                            }
                            Err(e) => {
                                println!("[RX {drone_id}] decode error: {e}");
                            }
                        }
                    }
                }
                Ok(None) => {
                    println!("[-] Drone {drone_id} position track closed");
                    break;
                }
                Err(e) => {
                    println!("[!] Drone {drone_id} position error: {e}");
                    break;
                }
            }
        }
    });
}

fn spawn_command_writer(
    unit_map: Arc<UnitMap<UnitContext>>,
    session_map: Arc<DroneSessionMap>,
    producer: Arc<OriginProducer>,
    unit_id: UnitId,
) {
    let drone_id = unit_id.as_str().to_string();

    tokio::spawn(async move {
        let control_path = control_broadcast_path(&drone_id);
        let mut broadcast = match producer.create_broadcast(&control_path) {
            Some(bc) => bc,
            None => {
                println!("[!] Failed to create control broadcast for {drone_id}");
                return;
            }
        };
        let mut track: TrackProducer = broadcast.create_track(Track::new(COMMAND_TRACK));

        println!("[*] Command writer started for {drone_id}");

        loop {
            if !session_map.has_active_session(&unit_id) {
                println!("[*] Command writer stopping - session ended for {drone_id}");
                break;
            }

            let cmd = unit_map
                .get_unit(&unit_id)
                .ok()
                .and_then(|unit_ref| unit_ref.view(|ctx| ctx.poll_command()).ok().flatten());

            if let Some(cmd_bytes) = cmd {
                track.write_frame(cmd_bytes);
            }

            tokio::time::sleep(COMMAND_POLL_INTERVAL).await;
        }
    });
}

// put some random commands in der to send to da drone
fn spawn_command_generator(
    unit_map: Arc<UnitMap<UnitContext>>,
    session_map: Arc<DroneSessionMap>,
    unit_id: UnitId,
) {
    let drone_id = unit_id.as_str().to_string();

    tokio::spawn(async move {
        let mut rng = StdRng::from_os_rng();

        loop {
            tokio::time::sleep(Duration::from_secs(rng.random_range(2..6))).await;

            // Check if session is still active
            if !session_map.has_active_session(&unit_id) {
                println!("[*] Command generator stopping - session ended for {drone_id}");
                break;
            }

            let unit_ref = match unit_map.get_unit(&unit_id) {
                Ok(r) => r,
                Err(_) => {
                    println!("[*] Command generator stopping - unit {drone_id} not found");
                    break;
                }
            };

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

            let mut buf = Vec::with_capacity(cmd.encoded_len());
            if cmd.encode(&mut buf).is_err() {
                println!("[!] Failed to encode command for {drone_id}");
                continue;
            }

            let result = unit_ref.view(|ctx| {
                ctx.enqueue_command(buf);
            });

            if result.is_ok() {
                println!("[TX] queued {cmd_type:?} for drone {drone_id}");
            } else {
                println!("[!] Failed to queue command - unit {drone_id} context invalid");
                break;
            }
        }
    });
}
