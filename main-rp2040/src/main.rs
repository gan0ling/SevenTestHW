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
use {defmt_rtt as _, panic_probe as _};
use pwmin_pio::pwmin_register_cmd;

// macro_rules! singleton {
//     ($val:expr) => {{
//         type T = impl Sized;
//         static STATIC_CELL: StaticCell<T> = StaticCell::new();
//         let (x,) = STATIC_CELL.init(($val,));
//         x
//     }};
// }


#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // init uart
    // let (tx_pin, rx_pin, uart) = (p.PIN_16, p.PIN_17, p.UART0);
    // let irq = interrupt::take!(UART0_IRQ);
    // let tx_buf = &mut singleton!([0u8; 128])[..];
    // let rx_buf = &mut singleton!([0u8; 128])[..];
    // let mut cfg = Config::default();
    // cfg.baudrate = 921600;
    // let uart = BufferedUart::new(uart, irq, tx_pin, rx_pin, tx_buf, rx_buf, cfg);

    // //FIXME: embassy-rp bug, we should set uartimsc.rxim to true
    // let regs = embassy_rp::pac::UART0.uartimsc();
    // unsafe {
    //     regs.modify(|w| w.set_rxim(true));
    // }
    // //end FIXME
    
    // let (mut rx, mut tx) = uart.split();
    //init log
    mylog::init_log();
    // init SevenShell
    // let history = LRUHistory::default();
    // let completer = StaticAutocomplete(CMD_LIST);
    // let mut shell:SevenShell = AShell::new(completer, history, &mylog::LOG_PIPE).await;
    pwmin_register_cmd();
    //init usb shell
    let irq = interrupt::take!(USBCTRL_IRQ);
    let driver = USBDriver::new(p.USB, irq);
    let usb_shell = usb_shell::UsbShell;

    //init pio
    // let pio0 = p.PIO0;
    // let pio1 = p.PIO1;
    // let (_, sm0, _sm1, _sm2, _sm3, ..) = pio0.split();
    // let (_, _pio1_sm0, ..) = pio1.split();
    // spawner.spawn(shell_task(spawner, rx, tx)).unwrap();
    // spawner.spawn(shell_task(uart)).unwrap();
    // spawner.spawn(mylog::log_task(tx));
    // spawner.spawn(pwmin_pio::pio0_sm0_pwmin_task(sm0, p.PIN_0.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio0_task_sm1(sm1, p.PIN_1.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio0_task_sm2(sm2, p.PIN_2.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio0_task_sm3(sm3, p.PIN_3.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pio1_task_sm0(pio1_sm0, p.PIN_4.degrade())).unwrap();
    // spawner.spawn(pwmin_pio::pwmin_log_task()).unwrap();
    // let mut counter = 0;

    // loop {
    //     let mut env = SevenShellEnv::default();
    //     let mut rx_buf:[u8;32] = [0;32];
    //     loop {
    //         let rx_len = rx.read(&mut rx_buf).await.unwrap();
    //         for byte in &rx_buf[..rx_len] {
    //             shell.feed(&mut env, *byte).await;
    //         }
    //     }
    // }

    // join(tx_fut, rx_fut).await;
    usb_shell.run(&mut usb_shell::LoggerState::new(), driver).await;
}
