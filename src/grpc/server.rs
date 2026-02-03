use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info, warn};

use crate::drone::DroneSessionMap;
use crate::drone_proto::DronePosition;
use crate::drone_proto::echo_service_server::{EchoService, EchoServiceServer};
use crate::state_machine::echo::Position;
use crate::unit::UnitId;
use crate::unit_context::UnitContext;
use crate::unit_map::UnitMap;

pub async fn start_server(
    addr: SocketAddr,
    unit_map: Arc<UnitMap<UnitContext>>,
    session_map: Arc<DroneSessionMap>,
) -> anyhow::Result<()> {
    let service = DroneServiceImpl::new(unit_map, session_map);

    info!(address = %addr, "gRPC server starting");

    tonic::transport::Server::builder()
        .add_service(EchoServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

pub struct DroneServiceImpl {
    unit_map: Arc<UnitMap<UnitContext>>,
    session_map: Arc<DroneSessionMap>,
}

impl DroneServiceImpl {
    pub fn new(unit_map: Arc<UnitMap<UnitContext>>, session_map: Arc<DroneSessionMap>) -> Self {
        Self {
            unit_map,
            session_map,
        }
    }
}

#[tonic::async_trait]
impl EchoService for DroneServiceImpl {
    type EchoStream = Pin<Box<dyn futures::Stream<Item = Result<DronePosition, Status>> + Send>>;

    async fn echo(
        &self,
        request: Request<Streaming<DronePosition>>,
    ) -> Result<Response<Self::EchoStream>, Status> {
        let mut inbound = request.into_inner();

        // I need the first message to come in in order to get the drone ID.
        let first_msg = inbound
            .next()
            .await
            .ok_or_else(|| Status::invalid_argument("Empty stream"))?
            .map_err(|e| Status::internal(e.to_string()))?;

        let drone_id = first_msg.drone_id.clone();

        let unit_id = UnitId::from(drone_id.as_str());

        info!(drone_id = %drone_id, "DroneSession started");

        // Create or reuse unit context
        if self.unit_map.get_unit(&unit_id).is_err() {
            let context = UnitContext::new();
            self.unit_map
                .insert_unit(unit_id.clone(), context)
                .map_err(|e| Status::internal(e.to_string()))?;
        }

        match self.session_map.create_session(&unit_id) {
            Ok(session_id) => {
                info!(drone_id = %drone_id, session_id = %session_id, "Session created");
            }
            Err(e) => {
                return Err(Status::already_exists(e.to_string()));
            }
        }

        // Process that first telemetry message
        self.process_position(&unit_id, first_msg);

        // Spawn task to process telemetry → StateMachine
        let unit_map_for_telemetry = Arc::clone(&self.unit_map);
        let telemetry_session_map = Arc::clone(&self.session_map);
        let unit_id_for_telemetry = unit_id.clone();
        let drone_id_for_task = drone_id.clone();

        tokio::spawn(async move {
            while let Some(msg_result) = inbound.next().await {
                match msg_result {
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

                        if let Ok(unit_ref) =
                            unit_map_for_telemetry.get_unit(&unit_id_for_telemetry)
                        {
                            let _ = unit_ref.view(|ctx| ctx.update_position(position));
                        }
                    }
                    Err(e) => {
                        warn!(drone_id = %drone_id_for_task, error = %e, "Telemetry stream error");
                        break;
                    }
                }
            }

            // Cleanup on disconnect
            info!(drone_id = %drone_id_for_task, "Telemetry stream closed");
            let _ = telemetry_session_map.remove_session(&unit_id_for_telemetry);
        });

        let unit_map_for_echo = Arc::clone(&self.unit_map);
        let session_map_for_stream = Arc::clone(&self.session_map);
        let unit_id_for_stream = unit_id.clone();
        let drone_id_for_stream = drone_id.clone();

        let outbound = async_stream::stream! {
            loop {
                if !session_map_for_stream.has_active_session(&unit_id_for_stream) {
                    debug!(drone_id = %drone_id_for_stream, "Session ended, closing echo stream");
                    break;
                }

                let maybe_pos = unit_map_for_echo
                    .get_unit(&unit_id_for_stream)
                    .ok()
                    .and_then(|unit_ref| {
                        unit_ref.view(|ctx| ctx.poll_position()).ok().flatten()
                    });

                if let Some(pos_bytes) = maybe_pos {
                    let pos = DronePosition {
            drone_id: pos_bytes.drone_id,
             latitude: pos_bytes.latitude,
             longitude: pos_bytes.longitude,
             altitude_m: pos_bytes.altitude_m,
             heading_deg: pos_bytes.heading_deg,
             speed_mps: pos_bytes.speed_mps,
             timestamp: pos_bytes.timestamp,

                    };
                            debug!(drone_id = %drone_id_for_stream, position = ?pos, "Sending position");
                            yield Ok(pos);
                }

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };

        Ok(Response::new(Box::pin(outbound)))
    }
}

impl DroneServiceImpl {
    fn process_position(&self, unit_id: &UnitId, pos: crate::drone_proto::DronePosition) {
        let position = Position {
            drone_id: pos.drone_id,
            latitude: pos.latitude,
            longitude: pos.longitude,
            altitude_m: pos.altitude_m,
            heading_deg: pos.heading_deg,
            speed_mps: pos.speed_mps,
            timestamp: pos.timestamp,
        };

        if let Ok(unit_ref) = self.unit_map.get_unit(unit_id) {
            let _ = unit_ref.view(|ctx| ctx.update_position(position));
        }
    }
}
