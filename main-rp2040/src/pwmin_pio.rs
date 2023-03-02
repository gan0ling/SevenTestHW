use {defmt_rtt as _, panic_probe as _};
use core::sync::atomic::AtomicBool;

use ashell::ShellResult;
use embassy_rp::{gpio::{AnyPin, Pin}, Peripheral, Peripherals, peripherals::PIO1, peripherals::PIO0, PeripheralRef, pio::PioCommon};
use embassy_rp::pio::{PioStateMachine, PioStateMachineInstance, Pio0, Pio1, Sm0, Sm1, Sm2, Sm3, PioPeripheral,
                      ShiftDirection,FifoJoin};
use embassy_rp::pio_instr_util;
use embassy_rp::relocate::RelocatedProgram;
use {defmt_rtt as _, panic_probe as _};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pubsub::WaitResult;
use embassy_sync::pubsub::PubSubChannel;
use heapless::Vec;
use embassy_executor::Spawner;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use crate::shell::register_shell_cmd;

pub type PwmInCommandSignal = Signal<ThreadModeRawMutex, PwmInCommand>;

const SM_CLK:u32 = 125_000_000; //125MHz
static PWM_PUBSUB_CHANNEL:PubSubChannel::<ThreadModeRawMutex, PwmInfo, 200, 1, 5> = PubSubChannel::new();
static mut PWMIN: PwmInShellEnv = PwmInShellEnv::new();
#[derive(Clone, Copy, defmt::Format)]
pub struct PwmInfo {
    pin:u32,
    clk:u32,
    high_period: u32, //high period in tick count
    low_period: u32 //low period in tick count
}

impl Default for PwmInfo
{
    fn default() -> Self {
        Self {
            pin: 0,
            clk: SM_CLK,
            high_period: 0,
            low_period: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PwmInCommand {
    Start(usize),
    Stop,
}
// #[derive(Copy, Clone)]
// enum PioSmInstance {
//     Pio0Sm0(PioStateMachineInstance<Pio0, Sm0>),
//     Pio0Sm1(PioStateMachineInstance<Pio0, Sm1>),
//     Pio0Sm2(PioStateMachineInstance<Pio0, Sm2>),
//     Pio0Sm3(PioStateMachineInstance<Pio0, Sm3>),
//     Pio1Sm0(PioStateMachineInstance<Pio1, Sm0>),
//     Pio1Sm1(PioStateMachineInstance<Pio1, Sm1>),
//     Pio1Sm2(PioStateMachineInstance<Pio1, Sm2>),
//     Pio1Sm3(PioStateMachineInstance<Pio1, Sm3>),
// }
struct PwmIn {
    // pin: AnyPin,
    run: bool,
    // sm_no: usize,
    // pio_no: usize,
    // cmd: Signal<ThreadModeRawMutex, PwmInCommand>,
    cmd: PwmInCommandSignal,
}

impl PwmIn {
    // pub const fn new(pin: Option<usize>, sm:usize, pio:usize) -> Self {
    pub const fn new() -> Self {
        Self {
            // pin,
            run:false,
            cmd:Signal::new()
        }
    }
}

pub enum PwmInError {
    PinError,
    PinInUse,
    PinAllocFail,
}
pub struct PwmInShellEnv {
    pwmin_state: [PwmIn;5],
}

impl PwmInShellEnv {
    pub const fn new() -> Self {
        Self {
            pwmin_state: [
                            PwmIn::new(), 
                            PwmIn::new(), 
                            PwmIn::new(), 
                            PwmIn::new(), 
                            PwmIn::new(), 
                            // PwmIn::new(None,1, 1), 
                            // PwmIn::new(None,1, 2), 
                            // PwmIn::new(None,1, 3), 
                        ],
        }
    }

    fn pin_in_use(&self, idx:usize) -> bool {
        if idx < self.pwmin_state.len() {
            return self.pwmin_state[idx].run;
        }
        false
    }

    pub fn get_stop_signal(&self, no:usize) -> Option<&PwmInCommandSignal> {
        if no < self.pwmin_state.len() {
            Some(&self.pwmin_state[no].cmd)
        } else {
            None
        }
    }

    pub fn stop(&mut self, idx:usize) {
        if idx < self.pwmin_state.len() {
            self.pwmin_state[idx].run = false;
            self.pwmin_state[idx].cmd.signal(PwmInCommand::Stop);
        }
    }

    // fn create_pin(&self, pin:usize) -> Option<AnyPin> {
    //     if pin > 29 {
    //         return None;
    //     }

    // }

    // fn create_sm(&self, pio_no:usize, sm_no:usize) -> impl PioStateMachine {
    // }

    pub fn start(&mut self, idx:usize) -> Result<(), PwmInError> {
        if self.pin_in_use(idx) {
            Err(PwmInError::PinInUse)
        } else {
            if idx < self.pwmin_state.len() {
                self.pwmin_state[idx].cmd.signal(PwmInCommand::Start(idx));
                Ok(())
            } else {
                Err(PwmInError::PinError)
            }
        }
    }

}

fn pwmin_cmd(cmd:&str, args:&str) -> ShellResult {
    let (sub_cmd , sub_args) = args.split_once(" ").unwrap_or((args, &""));
    let stop_all = if sub_args.starts_with("all") {
        true
    } else {
        false
    };
    match sub_cmd {
        "start" => {
            //start pwmin
            sub_args.split_ascii_whitespace().map(|a| {a.parse::<usize>().unwrap()}).for_each(|pin| { 
                unsafe {PWMIN.start(pin)};
            });
            Ok(())
        },
        "stop" => {
            //stop pwmin
            sub_args.split_ascii_whitespace().map(|a| {a.parse::<usize>().unwrap()}).for_each(|pin| { 
                unsafe {PWMIN.stop(pin)};
            });
            Ok(())
        },
        _ => {
            Err(ashell::ShellError::ExecuteError(-1))
        }
    }
}

//register pwmin cmd
pub fn pwmin_register_cmd() {
    register_shell_cmd("pwmin", pwmin_cmd);
}

macro_rules! impl_pwmin_pio {
    ($pio:ident, $sm:ident, $fn:ident) => {
        #[embassy_executor::task]
        pub async fn $fn(mut sm: PioStateMachineInstance<$pio, $sm>, pin:AnyPin, signal_no:usize, wrap_source:u8, wrap_target:u8) {
            //setup msg
            let mut msg:PwmInfo = PwmInfo::default();
            msg.pin = pin.pin() as u32;
            let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();
            let signal = unsafe {PWMIN.get_stop_signal(signal_no).unwrap()};

            // setup sm
            sm.set_enable(false);
            sm.restart();
            sm.clear_fifos();
            let _wait_irq = sm.sm_no();
            pio_instr_util::exec_jmp(&mut sm, 0);
            sm.set_wrap(wrap_source, wrap_target);

            let pin = sm.make_pio_pin(pin);
            sm.set_jmp_pin(pin.pin());
            sm.set_in_base_pin(&pin);

            let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
            sm.set_clkdiv(clkdiv << 8);

            // sm.set_autopull(false);
            sm.set_fifo_join(FifoJoin::RxOnly);
            sm.set_in_shift_dir(ShiftDirection::Left);
            sm.set_pull_threshold(2);

            let mut high_period:u32 = 10; 
            let mut low_period:u32 = 20; 
            let mut tmp_period_1:u32 = 0;
            let mut tmp_period_2:u32 = 0;

            loop {
                let cmd = signal.wait().await;
                // log::info!("cmd:{:?}", cmd);
                if let PwmInCommand::Start(_) = cmd {
                    //have received start cmd
                    sm.set_enable(true);
                    loop {
                        // sm.wait_irq(_wait_irq).await;
                        tmp_period_1 = sm.wait_pull().await;
                        tmp_period_2 = sm.wait_pull().await;
                        sm.clear_fifos();
                        if tmp_period_1 & 0xF0000000 != 0 {
                            //tmp_period_1 is low_period
                            high_period = tmp_period_2 * 2;
                            low_period = tmp_period_1 * 2;
                        } else {
                            //tmp_period_1 is high_period
                            low_period = tmp_period_2 * 2;
                            high_period = tmp_period_1 * 2;
                        }
                        if ((high_period / 10) != (msg.high_period / 10)) || ((low_period / 10) != (msg.low_period / 10)) {
                            msg.high_period = high_period;
                            msg.low_period = low_period;
                            publisher.publish_immediate(msg);
                        }

                        //check whether we need exit
                        if signal.signaled() {
                            log::info!("[pwmin] pin {} exited", pin.pin());
                            sm.set_enable(false);
                            break;
                        }
                    }
                }
                else {
                    signal.reset();
                }
            }
        }
    };
}

impl_pwmin_pio!(Pio0, Sm0, pio0_sm0_pwmin_task);
impl_pwmin_pio!(Pio0, Sm1, pio0_sm1_pwmin_task);
impl_pwmin_pio!(Pio0, Sm2, pio0_sm2_pwmin_task);
impl_pwmin_pio!(Pio0, Sm3, pio0_sm3_pwmin_task);
impl_pwmin_pio!(Pio1, Sm0, pio1_sm0_pwmin_task);
// impl_pwmin_pio!(Pio1, Sm1, pio1_sm1_pwmin_task);
// impl_pwmin_pio!(Pio1, Sm2, pio1_sm2_pwmin_task);
// impl_pwmin_pio!(Pio1, Sm3, pio1_sm3_pwmin_task);

// pub fn pwmin_init(pio0sm0:PioStateMachineInstance<Pio0, Sm0>, pio0sm1:PioStateMachineInstance<Pio0, Sm1>, pio0sm2:PioStateMachineInstance<Pio0, Sm2>,
                //   pio0sm3:PioStateMachineInstance<Pio0, Sm3>, pio1sm0:PioStateMachineInstance<Pio1, Sm0>) {
pub async fn pwmin_init(pio0:PIO0, pio1:PIO1, pin0:AnyPin, pin1:AnyPin, pin2:AnyPin, pin3:AnyPin, pin4:AnyPin) {
    register_shell_cmd("pwmin", pwmin_cmd);

    //spawn task
    let (mut pio0common, sm0, sm1, sm2, sm3) = pio0.split();
    let (mut pio1common, sm4, ..) = pio1.split();

    //setup pwmin_program for PIO0 and PIO1 for share
    let prg = pio_proc::pio_file!("./src/PwmIn.pio");
    let relocated = RelocatedProgram::new(&prg.program);
    let pio::Wrap{ source, target } = relocated.wrap();
    pio0common.write_instr(relocated.origin() as usize, relocated.code());
    pio1common.write_instr(relocated.origin() as usize, relocated.code());

    Spawner::for_current_executor().await.spawn(pio0_sm0_pwmin_task(sm0, pin0, 0, source, target)).unwrap();
    Spawner::for_current_executor().await.spawn(pio0_sm1_pwmin_task(sm1, pin1, 1, source, target)).unwrap();
    Spawner::for_current_executor().await.spawn(pio0_sm2_pwmin_task(sm2, pin2, 2, source, target)).unwrap();
    Spawner::for_current_executor().await.spawn(pio0_sm3_pwmin_task(sm3, pin3, 3, source, target)).unwrap();
    Spawner::for_current_executor().await.spawn(pio1_sm0_pwmin_task(sm4, pin4, 4, source, target)).unwrap();
    
    Spawner::for_current_executor().await.spawn(pwmin_log_task()).unwrap();
}

#[embassy_executor::task]
pub async fn pwmin_log_task() {
    let mut sub = PWM_PUBSUB_CHANNEL.subscriber().unwrap();

    loop {
        if let WaitResult::Message(msg) = sub.next_message().await {
            log::info!("[PwmIn]:{}:{}:{}:{}", msg.pin, msg.clk, msg.high_period, msg.low_period);
        }
    }
}
