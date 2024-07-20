use std::fs;
use std::time::{Duration, Instant};

use async_channel::Sender;

use core_pb::messages::server_status::{OverTheAirStep, OverTheAirStepCompletion, ServerStatus};
use core_pb::messages::{GuiToServerMessage, RobotToServerMessage, ServerToRobotMessage};
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};

use crate::sockets::{Destination, Incoming, Outgoing};

pub const PACKET_SIZE: usize = 4096;

pub struct OverTheAirProgramming {
    robots: [OverTheAirRobot; NUM_ROBOT_NAMES],
    binary: Vec<u8>,

    tx: Sender<(Destination, Outgoing)>,
}

pub struct OverTheAirRobot {
    name: RobotName,
    start: Instant,
    current_step: OverTheAirStep,
    last_update: Option<Instant>,
}

impl OverTheAirRobot {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            start: Instant::now(),
            current_step: Default::default(),
            last_update: None,
        }
    }
}

impl OverTheAirRobot {
    fn update_failed(&mut self, status: &mut ServerStatus) {
        status.robots[self.name as usize]
            .ota
            .push(OverTheAirStepCompletion {
                step: self.current_step,
                since_beginning: self.start.elapsed(),
                success: Some(false),
            });
        self.current_step = OverTheAirStep::Failed;
        self.last_update = None;
    }

    fn update_completed(&mut self, status: &mut ServerStatus) {
        if self.current_step == OverTheAirStep::GuiRequest {
            self.start = Instant::now();
        }
        status.robots[self.name as usize]
            .ota
            .push(OverTheAirStepCompletion {
                step: self.current_step,
                since_beginning: self.start.elapsed(),
                success: Some(true),
            });
        let last_step: usize = self.current_step.into();
        self.current_step = (last_step + 1_usize).into();
        self.last_update = None;
    }

    fn update_overwrite(&mut self, new: OverTheAirStep, status: &mut ServerStatus) {
        self.last_update = None;
        if let Some(last) = status.robots[self.name as usize].ota.last_mut() {
            last.step = new;
            last.since_beginning = self.start.elapsed();
            last.success = None;
        }
    }
}

async fn send(tx: &mut Sender<(Destination, Outgoing)>, to: RobotName, msg: ServerToRobotMessage) {
    tx.send((Destination::Robot(to), Outgoing::ToRobot(msg)))
        .await
        .unwrap();
}

impl OverTheAirProgramming {
    pub fn new(tx: Sender<(Destination, Outgoing)>) -> Self {
        Self {
            robots: RobotName::get_all().map(|name| OverTheAirRobot::new(name)),
            binary: vec![],

            tx,
        }
    }

    async fn send_firmware_part(&mut self, to: RobotName, offset: usize) {
        self.tx
            .send((
                Destination::Robot(to),
                Outgoing::ToRobot(ServerToRobotMessage::FirmwareWritePart {
                    offset,
                    len: PACKET_SIZE,
                }),
            ))
            .await
            .unwrap();
        self.tx
            .send((
                Destination::Robot(to),
                Outgoing::RawBytes(self.binary[offset..offset + PACKET_SIZE].to_vec()),
            ))
            .await
            .unwrap();
    }

    async fn cancel_update(&mut self, name: RobotName) {
        send(
            &mut self.tx,
            name,
            ServerToRobotMessage::CancelFirmwareUpdate,
        )
        .await;
        self.robots[name as usize] = OverTheAirRobot::new(name);
    }

    /// Retry operations if necessary; should be called frequently
    pub async fn tick(&mut self, _status: &mut ServerStatus) {
        for name in RobotName::get_all() {
            let do_update = match self.robots[name as usize].last_update {
                None => true,
                Some(t) => t.elapsed() > Duration::from_millis(500),
            } && self.robots[name as usize].current_step
                != OverTheAirStep::GuiRequest;
            if do_update {
                self.robots[name as usize].last_update = Some(Instant::now());
                let msg = match self.robots[name as usize].current_step {
                    OverTheAirStep::RobotReadyConfirmation => {
                        Some(ServerToRobotMessage::ReadyToStartUpdate)
                    }
                    OverTheAirStep::DataTransfer { received, .. } => {
                        self.send_firmware_part(name, received).await;
                        None
                    }
                    OverTheAirStep::HashConfirmation => {
                        Some(ServerToRobotMessage::CalculateFirmwareHash)
                    }
                    OverTheAirStep::MarkUpdateReady => {
                        Some(ServerToRobotMessage::MarkFirmwareUpdated)
                    }
                    OverTheAirStep::Reboot => Some(ServerToRobotMessage::Reboot),
                    OverTheAirStep::CheckFirmwareSwapped => {
                        Some(ServerToRobotMessage::IsFirmwareSwapped)
                    }
                    OverTheAirStep::MarkUpdateBooted => {
                        Some(ServerToRobotMessage::MarkFirmwareBooted)
                    }
                    OverTheAirStep::GuiRequest
                    | OverTheAirStep::GuiConfirmation
                    | OverTheAirStep::FinalGuiConfirmation
                    | OverTheAirStep::Finished
                    | OverTheAirStep::Failed
                    | OverTheAirStep::FetchBinary => None,
                };
                if let Some(msg) = msg {
                    send(&mut self.tx, name, msg).await;
                }
            }
        }
    }

    /// Pass all incoming messages through this function; most will do nothing
    pub async fn update(&mut self, msg: &(Destination, Incoming), status: &mut ServerStatus) {
        match msg {
            // gui requests firmware update
            (_, Incoming::FromGui(GuiToServerMessage::StartOtaFirmwareUpdate(name))) => {
                if self.robots[*name as usize].current_step != OverTheAirStep::GuiRequest {
                    eprintln!(
                        "Firmware update was requested for {name} when one was already in progress"
                    );
                    self.cancel_update(*name).await;
                }
                // start update
                status.robots[*name as usize].ota.clear();
                self.robots[*name as usize].update_completed(status);
                self.tick(status).await;
            }
            // gui cancels firmware update
            (_, Incoming::FromGui(GuiToServerMessage::CancelOtaFirmwareUpdate(name))) => {
                self.cancel_update(*name).await;
            }
            // message from robot
            (Destination::Robot(name), Incoming::FromRobot(msg)) => match msg {
                // robot indicates that it is ready for the update
                RobotToServerMessage::ReadyToStartUpdate => {
                    if self.robots[*name as usize].current_step
                        == OverTheAirStep::RobotReadyConfirmation
                    {
                        self.robots[*name as usize].update_completed(status);
                        // read binary
                        match fs::read("pico_pb/target/thumbv6m-none-eabi/release/mdrc-pacbot-pico")
                        {
                            Ok(bytes) => {
                                self.binary = bytes;
                                self.robots[*name as usize].update_completed(status);
                                if let OverTheAirStep::DataTransfer { total, .. } =
                                    &mut self.robots[*name as usize].current_step
                                {
                                    *total = self.binary.len();
                                }
                                // send first packet
                                self.tick(status).await;
                            }
                            Err(e) => {
                                eprintln!("Error reading binary for robot: {e:?}");
                                self.robots[*name as usize].update_failed(status);
                            }
                        }
                    }
                }
                // robot indicates it has received the firmware part
                RobotToServerMessage::ConfirmFirmwarePart { offset, len } => {
                    if let OverTheAirStep::DataTransfer { received, total } =
                        self.robots[*name as usize].current_step
                    {
                        if *offset != received {
                            self.robots[*name as usize].update_failed(status);
                            eprintln!("Robot received bytes at the wrong offset");
                            self.cancel_update(*name).await;
                        } else {
                            // is there another firmware part?
                            if *offset + *len < total {
                                self.robots[*name as usize].update_overwrite(
                                    OverTheAirStep::DataTransfer {
                                        received: *offset + *len,
                                        total: self.binary.len(),
                                    },
                                    status,
                                );
                                // send next packet
                                self.tick(status).await;
                            } else {
                                // we are finished sending the bytes
                                self.robots[*name as usize].update_completed(status);
                                self.tick(status).await;
                            }
                        }
                    }
                }
                // robot sends hash back
                // todo confirm this
                RobotToServerMessage::FirmwareHash(_) => {
                    if self.robots[*name as usize].current_step == OverTheAirStep::HashConfirmation
                    {
                        self.robots[*name as usize].update_completed(status);
                        // wait for a gui to confirm update
                    }
                }
                // robot has marked the new firmware to be used on boot
                RobotToServerMessage::MarkedFirmwareUpdated => {
                    self.complete_if_currently(name, OverTheAirStep::MarkUpdateReady, status)
                        .await;
                }
                RobotToServerMessage::Rebooting => {
                    self.complete_if_currently(name, OverTheAirStep::Reboot, status)
                        .await;
                }
                RobotToServerMessage::FirmwareIsSwapped(swapped) => {
                    if self.robots[*name as usize].current_step
                        == OverTheAirStep::CheckFirmwareSwapped
                    {
                        if *swapped {
                            self.robots[*name as usize].update_completed(status);
                            self.tick(status).await;
                        } else {
                            eprintln!("Robot {name} seems to have rebooted, but its firmware wasn't swapped. Did it crash?");
                            self.robots[*name as usize].update_failed(status);
                        }
                    }
                }
                RobotToServerMessage::MarkedFirmwareBooted => {
                    self.complete_if_currently(name, OverTheAirStep::MarkUpdateBooted, status)
                        .await;
                }
                _ => {}
            },
            (_, Incoming::FromGui(GuiToServerMessage::ConfirmFirmwareUpdate(name))) => {
                self.complete_if_currently(name, OverTheAirStep::GuiConfirmation, status)
                    .await;
                self.complete_if_currently(name, OverTheAirStep::FinalGuiConfirmation, status)
                    .await;
            }
            _ => {}
        }
    }

    async fn complete_if_currently(
        &mut self,
        name: &RobotName,
        current: OverTheAirStep,
        status: &mut ServerStatus,
    ) {
        if self.robots[*name as usize].current_step == current {
            self.robots[*name as usize].update_completed(status);
            self.tick(status).await;
        }
    }
}
