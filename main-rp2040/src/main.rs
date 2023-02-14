#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]


mod pwmin_pio;
mod shell;

use heapless::Vec;
use embassy_executor::_export::StaticCell;
use embassy_rp::gpio::{AnyPin, Pin};
use embassy_executor::Spawner;
use embassy_rp::interrupt;
// use embassy_rp::peripherals::USB;
// use embassy_rp::usb::Driver as USBDriver;
use embassy_time::{Duration, Timer};
use embassy_rp::pio::{PioPeripherial};
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::{BufferedUart, BufferedUartRx, BufferedUartTx, Config};
use {defmt_rtt as _, panic_probe as _};

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}

#[embassy_executor::task]
async fn shell_task(spawner:Spawner, mut rx: BufferedUartRx<'static, UART0>, mut tx: BufferedUartTx<'static, UART0>) {
    // embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
    // static LOGGER: usb_shell::UsbShell<1024> = usb_shell::UsbShell::new();
    static LOGGER:serial_shell::UartShell<1024> = serial_shell::UartShell::new();
    unsafe {
            let _ = ::log::set_logger_racy(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Info));
    }
    let _ = LOGGER.run(spawner, rx, tx).await;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let (tx_pin, rx_pin, uart) = (p.PIN_16, p.PIN_17, p.UART0);
    let irq = interrupt::take!(UART0_IRQ);
    let tx_buf = &mut singleton!([0u8; 128])[..];
    let rx_buf = &mut singleton!([0u8; 128])[..];
    let mut cfg = Config::default();
    cfg.baudrate = 921600;
    let uart = BufferedUart::new(uart, irq, tx_pin, rx_pin, tx_buf, rx_buf, cfg);
    //FIXME: embassy-rp bug, we should set uartimsc.rxim to true
    let regs = embassy_rp::pac::UART0.uartimsc();
    unsafe {
        regs.modify(|w| w.set_rxim(true));
    }
    //end FIXME
    let (mut rx, mut tx) = uart.split();

    let pio0 = p.PIO0;
    let pio1 = p.PIO1;
    let (_, sm0, sm1, sm2, sm3, ..) = pio0.split();
    let (_, pio1_sm0, ..) = pio1.split();

    spawner.spawn(shell_task(spawner, rx, tx)).unwrap();
    spawner.spawn(pwmin_pio::Pio0_Sm0_pwmin_task(sm0, p.PIN_0.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio0_task_sm1(sm1, p.PIN_1.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio0_task_sm2(sm2, p.PIN_2.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio0_task_sm3(sm3, p.PIN_3.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio1_task_sm0(pio1_sm0, p.PIN_4.degrade())).unwrap();
    spawner.spawn(pwmin_pio::pwmin_log_task()).unwrap();
    // let mut counter = 0;
    // loop {
        // counter += 1;
        // log::info!("Tick {}", counter);
        // Timer::after(Duration::from_secs(1)).await;
    // }
}
