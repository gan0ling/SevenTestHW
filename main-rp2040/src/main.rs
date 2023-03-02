#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]


mod mylog;
mod shell;
mod usb_shell;
mod pwmin_pio;

use embassy_executor::Spawner;
use embassy_rp::interrupt;
use embassy_rp::usb::Driver as USBDriver;
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::{BufferedUart, BufferedUartRx, BufferedUartTx, Config};
use embedded_io::asynch::{Read, Write};
use embassy_rp::gpio::{AnyPin, Pin};
use embassy_executor::_export::StaticCell;
use embassy_rp::pio::PioPeripheral;
use {defmt_rtt as _, panic_probe as _};
use pwmin_pio::pwmin_init;
use embassy_time::{Duration, Timer};
use crate::shell::{SHELL_ENV, create_shell, SevenShell};

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // init uart
    let (tx_pin, rx_pin, uart) = (p.PIN_16, p.PIN_17, p.UART0);
    let irq = interrupt::take!(UART0_IRQ);
    let tx_buf = &mut singleton!([0u8; 128])[..];
    let rx_buf = &mut singleton!([0u8; 128])[..];
    let mut cfg = Config::default();
    cfg.baudrate = 921600;
    let uart = BufferedUart::new(uart, irq, tx_pin, rx_pin, tx_buf, rx_buf, cfg);
    let (mut rx, mut tx) = uart.split();

    // //FIXME: embassy-rp bug, we should set uartimsc.rxim to true
    let regs = embassy_rp::pac::UART0.uartimsc();
    unsafe {
        regs.modify(|w| w.set_rxim(true));
    }
    // //end FIXME
    
    //init log
    mylog::init_log();
    spawner.spawn(mylog::log_task(tx));
    log::info!("welcome to SevenTest");
    pwmin_init(p.PIO0, p.PIO1, p.PIN_0.degrade(), p.PIN_1.degrade(), p.PIN_2.degrade(), p.PIN_3.degrade(), p.PIN_4.degrade()).await;

    //init usb shell
    #[cfg(usb_shell)]
    {
        let irq = interrupt::take!(USBCTRL_IRQ);
        let driver = USBDriver::new(p.USB, irq);
        let usb_shell = usb_shell::UsbShell;
        usb_shell.run(&mut usb_shell::LoggerState::new(), driver).await;
    }
    

    let mut shell: SevenShell = create_shell().await;
    let mut rx_buf:[u8;32] = [0;32];
    loop {
        let rx_len = rx.read(&mut rx_buf).await.unwrap();
        for byte in &rx_buf[..rx_len] {
            unsafe {shell.feed(&mut SHELL_ENV, *byte).await;}
        }
    }

    // join(tx_fut, rx_fut).await;
}
