use crate::logging::LOGS_PIPE;
use crate::Irqs;
use core::cell::RefCell;
use core_pb::driving::network::{NetworkScanInfo, RobotNetworkBehavior};
use cyw43::{Control, JoinOptions};
use cyw43_pio::PioSpi;
use defmt::{info, unwrap, Format};
use embassy_boot_rp::{AlignedBuffer, BlockingFirmwareUpdater, FirmwareUpdaterConfig, State};
use embassy_embedded_hal::flash::partition::BlockingPartition;
use embassy_executor::Spawner;
use embassy_net::tcp::{AcceptError, TcpSocket};
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::flash::{Blocking, Flash};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, FLASH, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::Pio;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use static_cell::StaticCell;

const FLASH_SIZE: usize = 2 * 1024 * 1024;

#[allow(clippy::type_complexity)]
pub struct Network {
    control: Control<'static>,
    stack: Stack<'static>,
    updater: BlockingFirmwareUpdater<
        'static,
        BlockingPartition<'static, NoopRawMutex, Flash<'static, FLASH, Blocking, 2097152>>,
        BlockingPartition<'static, NoopRawMutex, Flash<'static, FLASH, Blocking, 2097152>>,
    >,
}

#[derive(Debug, Format)]
pub enum NetworkError {
    Connection(u32),
    Accept(AcceptError),
    FirmwareUpdater,
}

impl RobotNetworkBehavior for Network {
    type Error = NetworkError;
    type Socket<'a> = TcpSocket<'a>;

    async fn mac_address(&mut self) -> [u8; 6] {
        self.control.address().await
    }

    async fn wifi_is_connected(&self) -> Option<[u8; 4]> {
        self.stack.config_v4().map(|x| x.address.address().octets())
    }

    async fn list_networks<const C: usize>(&mut self) -> Vec<NetworkScanInfo, C> {
        let mut network_info = Vec::new();
        let mut networks = self.control.scan(Default::default()).await;
        for i in 0..C {
            if let Some(network) = networks.next().await {
                // cyw43/CHIP
                let band = (network.chanspec & 0xc000) >> 14;
                network_info[i] = NetworkScanInfo {
                    ssid: network.ssid,
                    is_5g: band == 0xc000,
                }
            } else {
                break;
            }
        }
        network_info
    }

    async fn connect_wifi(
        &mut self,
        network: &str,
        password: Option<&str>,
    ) -> Result<(), <Self as RobotNetworkBehavior>::Error> {
        info!("Joining network {} with password {:?}", network, password);

        if let Some(password) = password {
            // let x = self.control.scan(cyw43::ScanOptions::default()).await.next().await.unwrap();
            // x.
            self.control
                .join(network, JoinOptions::new(password.as_bytes()))
                .await
        } else {
            self.control.join(network, JoinOptions::new_open()).await
        }
        .map_err(|e| NetworkError::Connection(e.status))?;

        info!("Joined network {}", network);

        // Wait for DHCP, not necessary when using static IP
        info!("Waiting for DHCP...");
        while !self.stack.is_config_up() {
            Timer::after_millis(100).await;
        }
        info!("DHCP is now up!");

        blink(&mut self.control, 1, Duration::from_millis(400)).await;

        info!("ip = {}", self.stack.config_v4().unwrap().address);

        Ok(())
    }

    async fn disconnect_wifi(&mut self) {
        self.control.leave().await;
    }

    async fn tcp_accept<'a>(
        &mut self,
        port: u16,
        tx_buffer: &'a mut [u8; 5192],
        rx_buffer: &'a mut [u8; 5192],
    ) -> Result<Self::Socket<'a>, <Self as RobotNetworkBehavior>::Error>
    where
        Self: 'a,
    {
        let mut socket = TcpSocket::new(self.stack, rx_buffer, tx_buffer);
        // socket.set_timeout(Some(Duration::from_secs(10)));

        // self.control.gpio_set(0, false).await;
        info!("Listening for connections on port {}", port);
        socket.accept(port).await.map_err(NetworkError::Accept)?;
        info!("Connection successful");

        blink(&mut self.control, 1, Duration::from_millis(100)).await;

        Ok(socket)
    }

    async fn tcp_close<'a>(&mut self, socket: &mut Self::Socket<'a>) {
        socket.close()
    }

    async fn prepare_firmware_update(&mut self) {
        // let _ = self.updater.prepare_update();
    }

    async fn write_firmware(&mut self, offset: usize, data: &[u8]) -> Result<(), Self::Error> {
        self.updater
            .write_firmware(offset, data)
            .map_err(|_| NetworkError::FirmwareUpdater)
    }

    async fn hash_firmware(&mut self, _update_len: u32, _output: &mut [u8; 32]) {
        // todo
    }

    async fn mark_firmware_updated(&mut self) {
        let _ = self.updater.mark_updated();
    }

    async fn firmware_swapped(&mut self) -> bool {
        matches!(self.updater.get_state(), Ok(State::Swap))
    }

    async fn reboot(&mut self) {
        Timer::after_secs(3).await;
        cortex_m::peripheral::SCB::sys_reset();
    }

    async fn mark_firmware_booted(&mut self) {
        let _ = self.updater.mark_booted();
    }

    // fn read_logging_bytes(buf: &mut [u8]) -> Option<usize> {
    //     LOGS_PIPE.try_read(buf).ok()
    // }
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[allow(clippy::too_many_arguments)]
pub async fn initialize_network(
    spawner: Spawner,
    pwr: PIN_23,
    cs: PIN_25,
    pio: PIO0,
    dio: PIN_24,
    clk: PIN_29,
    dma: DMA_CH0,
    flash: FLASH,
) -> Network {
    info!("Wifi task started");

    let pwr = Output::new(pwr, Level::Low);
    let cs = Output::new(cs, Level::High);
    let mut pio = Pio::new(pio, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, dio, clk, dma);

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10140000
    // let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    // let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    info!("Wifi startup complete");

    Timer::after_secs(1).await;

    let config = Config::dhcpv4(Default::default());
    // let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
    //     address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 1, 212), 24),
    //     dns_servers: Vec::new(),
    //     gateway: None,
    // });

    // Generate random seed
    let seed = 0xab9a_dd1a_3b2b_715a; // chosen by fair dice roll

    // Init network stack
    static RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    info!("Network stack initialized");

    let flash = Flash::<_, _, FLASH_SIZE>::new_blocking(flash);
    // let flash = Mutex::new(RefCell::new(flash));
    static FLASH_CELL: StaticCell<Mutex<NoopRawMutex, RefCell<Flash<FLASH, Blocking, 2097152>>>> =
        StaticCell::new();
    let flash = &*FLASH_CELL.init_with(|| Mutex::new(RefCell::new(flash)));

    let config = FirmwareUpdaterConfig::from_linkerfile_blocking(flash, flash);
    static ALIGNED: StaticCell<AlignedBuffer<1>> = StaticCell::new();
    let aligned = ALIGNED.init_with(|| AlignedBuffer([0; 1]));
    let updater = BlockingFirmwareUpdater::new(config, &mut aligned.0);

    blink(&mut control, 2, Duration::from_millis(200)).await;

    Network {
        control,
        stack,
        updater,
    }
}

pub async fn blink<'a>(control: &mut Control<'a>, count: usize, duration: Duration) {
    for _ in 0..count {
        control.gpio_set(0, true).await;
        Timer::after(duration).await;
        control.gpio_set(0, false).await;
        Timer::after(duration).await;
    }
    Timer::after(Duration::from_secs(1)).await;
}
