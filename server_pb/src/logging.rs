use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use defmt_decoder::{DecodeError, Frame, Locations, StreamDecoder, Table};
use log::{info, Level, Record};
use ouroboros::self_referencing;
use std::env;
use std::path::Path;

pub struct RobotLoggers {
    locs: Locations,
    bad_box: RobotLoggersBadBox,
}

#[self_referencing]
struct RobotLoggersBadBox {
    table: Table,
    #[borrows(table)]
    #[covariant]
    decoders: [Box<dyn StreamDecoder + 'this>; NUM_ROBOT_NAMES],
}

impl RobotLoggers {
    pub fn generate() -> Result<Self, ()> {
        return Err(());
        let elf = std::fs::read("pico_pb/target/thumbv6m-none-eabi/release/mdrc-pacbot-pico")
            .map_err(|_| ())?;

        let table = Table::parse(&elf).ok().flatten().ok_or(())?;

        Ok(Self {
            locs: table.get_locations(&elf).map_err(|_| ())?,
            bad_box: RobotLoggersBadBoxBuilder {
                table,
                decoders_builder: |table| RobotName::get_all().map(|_| table.new_stream_decoder()),
            }
            .build(),
        })
    }

    pub fn feed_robot_logs(&mut self, name: RobotName, bytes: &[u8]) {
        return;
        self.bad_box.with_decoders_mut(|d| {
            d[name as usize].received(bytes);
            loop {
                match d[name as usize].decode() {
                    Ok(frame) => {
                        let (file, line, mod_path) =
                            location_info(&self.locs, &frame, &env::current_dir().unwrap());
                        log::logger().log(
                            &Record::builder()
                                .args(format_args!("{}", frame.display_message()))
                                .level(
                                    frame
                                        .level()
                                        .map(|l| match l {
                                            defmt_parser::Level::Trace => Level::Trace,
                                            defmt_parser::Level::Debug => Level::Debug,
                                            defmt_parser::Level::Info => Level::Info,
                                            defmt_parser::Level::Warn => Level::Warn,
                                            defmt_parser::Level::Error => Level::Error,
                                        })
                                        .unwrap_or(Level::Info),
                                )
                                .target(&format!(
                                    "defmt::{name}@{}::{}",
                                    mod_path.as_deref().unwrap_or("unknown"),
                                    line.unwrap_or(0),
                                ))
                                .module_path(mod_path.as_deref())
                                .file(file.as_deref())
                                .line(line)
                                .build(),
                        );
                    }
                    Err(DecodeError::UnexpectedEof) => break,
                    Err(DecodeError::Malformed) => {
                        info!("Malformed message from robot {name}")
                    }
                }
            }
        })
    }
}

type LocationInfo = (Option<String>, Option<u32>, Option<String>);

fn location_info(locs: &Locations, frame: &Frame, current_dir: &Path) -> LocationInfo {
    let (mut file, mut line, mut mod_path) = (None, None, None);

    let loc = locs.get(&frame.index());

    if let Some(loc) = loc {
        // try to get the relative path, else the full one
        let path = loc.file.strip_prefix(current_dir).unwrap_or(&loc.file);

        file = Some(path.display().to_string());
        line = Some(loc.line as u32);
        mod_path = Some(loc.module.clone());
    }

    (file, line, mod_path)
}
