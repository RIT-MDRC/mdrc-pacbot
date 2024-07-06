use crate::{send, Irqs};
use core::future::{ready, Future};
use core_pb::driving::network::{NetworkScanInfo, RobotNetworkBehavior};
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use cyw43::{Control, NetDriver};
use cyw43_pio::PioSpi;
use defmt::{info, unwrap, Format};
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::Pio;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use heapless::Vec;
use static_cell::StaticCell;

pub static NETWORK_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> = Channel::new();

pub struct Network {
    control: Control<'static>,
    stack: &'static Stack<NetDriver<'static>>,
}

#[derive(Debug, Format)]
pub enum NetworkError {
    ConnectionError(u32),
}

impl RobotTask for Network {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        send(message, to).await.map_err(|_| ())
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        NETWORK_CHANNEL.receive().await
    }
}

impl RobotNetworkBehavior for Network {
    type Error = NetworkError;

    fn wifi_is_connected(&self) -> impl Future<Output = Option<[u8; 4]>> {
        let ip = self.stack.config_v4().map(|x| x.address.address().0);
        ready(ip)
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
    ) -> Result<(), Self::Error> {
        info!("Joining network {}", network);

        if let Some(password) = password {
            self.control.join_wpa2(network, password).await
        } else {
            self.control.join_open(network).await
        }
        .map_err(|e| NetworkError::ConnectionError(e.status))?;

        info!("Joined network {}", network);

        // Wait for DHCP, not necessary when using static IP
        info!("Waiting for DHCP...");
        while !self.stack.is_config_up() {
            Timer::after_millis(100).await;
        }
        info!("DHCP is now up!");

        Ok(())
    }

    async fn disconnect_wifi(&mut self) {
        self.control.leave().await;
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

pub async fn initialize_network(
    spawner: Spawner,
    pwr: PIN_23,
    cs: PIN_25,
    pio: PIO0,
    dio: PIN_24,
    clk: PIN_29,
    dma: DMA_CH0,
) -> Network {
    info!("Wifi task started");

    let pwr = Output::new(pwr, Level::Low);
    let cs = Output::new(cs, Level::High);
    let mut pio = Pio::new(pio, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, dio, clk, dma);

    // let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    // let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10140000
    let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    info!("Wifi startup complete");

    let config = Config::dhcpv4(Default::default());
    // let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
    //     address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 1, 212), 24),
    //     dns_servers: Vec::new(),
    //     gateway: None,
    // });

    // Generate random seed
    let seed = 0xab9a_dd1a_3b2b_715a; // chosen by fair dice roll

    // Init network stack
    static STACK: StaticCell<Stack<NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<2>::new()),
        seed,
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    info!("Network stack initialized");

    Network { control, stack }
}
