use std::fs;
use std::time::{Duration, Instant};

use async_channel::Sender;

use core_pb::messages::ota::{OverTheAirStep, OverTheAirStepCompletion};
use core_pb::messages::server_status::ServerStatus;
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
    last_update: Option<Instant>,
}

impl OverTheAirRobot {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            start: Instant::now(),
            last_update: None,
        }
    }
}

impl OverTheAirRobot {
    fn update_failed(&mut self, status: &mut ServerStatus) {
        let robot = &mut status.robots[self.name as usize];
        let curr = robot.ota_current;
        robot.ota_completed.push(OverTheAirStepCompletion {
            step: curr,
            since_beginning: self.start.elapsed(),
            success: Some(false),
        });
        robot.ota_current = OverTheAirStep::GuiRequest;
        self.last_update = None;
        for OverTheAirStepCompletion { success, .. } in &mut robot.ota_completed {
            if success.is_none() {
                *success = Some(false)
            }
        }
    }

    fn update_new_in_progress(&mut self, status: &mut ServerStatus) {
        let curr = status.robots[self.name as usize].ota_current;
        status.robots[self.name as usize]
            .ota_completed
            .push(OverTheAirStepCompletion {
                step: curr,
                since_beginning: self.start.elapsed(),
                success: None,
            });
    }

    fn update_completed(&mut self, status: &mut ServerStatus) {
        let curr = status.robots[self.name as usize].ota_current;
        if curr == OverTheAirStep::GuiRequest {
            self.start = Instant::now();
        }
        if status.robots[self.name as usize]
            .ota_completed
            .last()
            .map(|x| x.step != curr)
            .unwrap_or(true)
        {
            status.robots[self.name as usize]
                .ota_completed
                .push(OverTheAirStepCompletion {
                    step: curr,
                    since_beginning: self.start.elapsed(),
                    success: Some(true),
                });
        }
        let last_step: usize = curr.into();
        status.robots[self.name as usize].ota_current = (last_step + 1_usize).into();
        if status.robots[self.name as usize].ota_current == OverTheAirStep::Finished {
            status.robots[self.name as usize]
                .ota_completed
                .push(OverTheAirStepCompletion {
                    step: OverTheAirStep::Finished,
                    since_beginning: self.start.elapsed(),
                    success: Some(true),
                });
            status.robots[self.name as usize].ota_current = OverTheAirStep::GuiRequest;
        }
        self.last_update = None;
    }

    fn update_overwrite(
        &mut self,
        new: OverTheAirStep,
        success: Option<bool>,
        status: &mut ServerStatus,
    ) {
        self.last_update = None;
        if let Some(last) = status.robots[self.name as usize].ota_completed.last_mut() {
            last.step = new;
            last.since_beginning = self.start.elapsed();
            last.success = success;
        }
        status.robots[self.name as usize].ota_current = new;
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
        let next_packet_len = if offset + PACKET_SIZE > self.binary.len() {
            self.binary.len() - offset
        } else {
            PACKET_SIZE
        };
        self.tx
            .send((
                Destination::Robot(to),
                Outgoing::RawBytes(self.binary[offset..offset + next_packet_len].to_vec()),
            ))
            .await
            .unwrap();
    }

    async fn cancel_update(&mut self, name: RobotName, status: &mut ServerStatus) {
        send(
            &mut self.tx,
            name,
            ServerToRobotMessage::CancelFirmwareUpdate,
        )
        .await;
        self.robots[name as usize].update_failed(status);
    }

    /// Retry operations if necessary; should be called frequently
    pub async fn tick(&mut self, status: &mut ServerStatus) {
        for name in RobotName::get_all() {
            let do_update = match self.robots[name as usize].last_update {
                None => true,
                Some(t) => t.elapsed() > Duration::from_millis(500),
            } && status.robots[name as usize].ota_current
                != OverTheAirStep::GuiRequest;
            if do_update {
                self.robots[name as usize].last_update = Some(Instant::now());
                let msg = match status.robots[name as usize].ota_current {
                    OverTheAirStep::RobotReadyConfirmation => {
                        Some(ServerToRobotMessage::ReadyToStartUpdate)
                    }
                    OverTheAirStep::DataTransfer { received, .. } => {
                        self.send_firmware_part(name, received).await;
                        None
                    }
                    OverTheAirStep::HashConfirmation => {
                        Some(ServerToRobotMessage::CalculateFirmwareHash(0))
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
                if status.robots[*name as usize].ota_current != OverTheAirStep::GuiRequest {
                    eprintln!(
                        "Firmware update was requested for {name} when one was already in progress"
                    );
                    self.cancel_update(*name, status).await;
                }
                // start update
                status.robots[*name as usize].ota_completed.clear();
                self.robots[*name as usize].update_completed(status);
                self.tick(status).await;
            }
            // gui cancels firmware update
            (_, Incoming::FromGui(GuiToServerMessage::CancelOtaFirmwareUpdate(name))) => {
                self.cancel_update(*name, status).await;
            }
            (_, Incoming::FromGui(GuiToServerMessage::ClearFirmwareUpdateHistory(name))) => {
                status.robots[*name as usize].ota_completed.clear();
            }
            // message from robot
            (Destination::Robot(name), Incoming::FromRobot(msg)) => match msg {
                // robot indicates that it is ready for the update
                RobotToServerMessage::ReadyToStartUpdate => {
                    println!("[server] {name} ready to start update");
                    if status.robots[*name as usize].ota_current
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
                                    &mut status.robots[*name as usize].ota_current
                                {
                                    *total = self.binary.len();
                                }
                                // send first packet
                                self.tick(status).await;
                                self.robots[*name as usize].update_new_in_progress(status);
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
                        status.robots[*name as usize].ota_current
                    {
                        if *offset != received {
                            self.robots[*name as usize].update_failed(status);
                            eprintln!("Robot received bytes at the wrong offset");
                            self.cancel_update(*name, status).await;
                        } else {
                            // is there another firmware part?
                            if *offset + *len < total {
                                self.robots[*name as usize].update_overwrite(
                                    OverTheAirStep::DataTransfer {
                                        received: *offset + *len,
                                        total: self.binary.len(),
                                    },
                                    None,
                                    status,
                                );
                                // send next packet
                                self.tick(status).await;
                            } else {
                                // we are finished sending the bytes
                                self.robots[*name as usize].update_overwrite(
                                    OverTheAirStep::DataTransfer {
                                        received: self.binary.len(),
                                        total: self.binary.len(),
                                    },
                                    Some(true),
                                    status,
                                );
                                self.robots[*name as usize].update_completed(status);
                                self.tick(status).await;
                            }
                        }
                    }
                }
                // robot sends hash back
                // todo confirm this
                RobotToServerMessage::FirmwareHash(_) => {
                    if status.robots[*name as usize].ota_current == OverTheAirStep::HashConfirmation
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
                    if status.robots[*name as usize].ota_current
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
        if status.robots[*name as usize].ota_current == current {
            self.robots[*name as usize].update_completed(status);
            self.tick(status).await;
        }
    }
}
