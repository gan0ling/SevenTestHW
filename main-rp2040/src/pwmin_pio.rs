use {defmt_rtt as _, panic_probe as _};
use ashell::ShellResult;
use embassy_rp::{gpio::{AnyPin, Pin}, PeripheralRef, pio::{PioPeripheral, PioInstanceBase, SmInstanceBase, PioInstance, SmInstance}, Peripheral, Peripherals};
use embassy_rp::pio::{PioStateMachine, PioStateMachineInstance, Pio0, Pio1, Sm0, Sm1, Sm2, Sm3,
                      ShiftDirection,FifoJoin};
use embassy_rp::pio_instr_util;
use embassy_rp::relocate::RelocatedProgram;
use {defmt_rtt as _, panic_probe as _};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pubsub::WaitResult;
use embassy_sync::pubsub::PubSubChannel;
use heapless::Vec;
use embassy_executor::Spawner;
use crate::shell::register_shell_cmd;


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

// #[derive(Copy)]
enum PioSmInstance {
    Pio0Sm0(PioStateMachineInstance<Pio0, Sm0>),
    Pio0Sm1(PioStateMachineInstance<Pio0, Sm1>),
    Pio0Sm2(PioStateMachineInstance<Pio0, Sm2>),
    Pio0Sm3(PioStateMachineInstance<Pio0, Sm3>),
    Pio1Sm0(PioStateMachineInstance<Pio1, Sm0>),
    Pio1Sm1(PioStateMachineInstance<Pio1, Sm1>),
    Pio1Sm2(PioStateMachineInstance<Pio1, Sm2>),
    Pio1Sm3(PioStateMachineInstance<Pio1, Sm3>),
}
struct PwmIn {
    pin: Option<usize>,
    run: bool,
    sm: PioSmInstance,
    // pio: usize,
}

impl PwmIn {
    // pub const fn new(pin: Option<usize>, sm:usize, pio:usize) -> Self {
    pub const fn new(pin: Option<usize>, sm:PioSmInstance) -> Self {
        Self {
            pin,
            run:false,
            sm,
            // pio
        }
    }
}

enum PwmInError {
    PinError,
    PinInUse,
    PinAllocFail,
}
pub struct PwmInShellEnv {
    pwmin_state: [PwmIn;8]
}

impl PwmInShellEnv {
    pub const fn new() -> Self {
        let p = unsafe {Peripherals::steal()};
        let (_, sm0, sm1, sm2, sm3) = p.PIO0.split();
        let (_, sm4, sm5, sm6, sm7) = p.PIO1.split();
        Self {
            pwmin_state: [
                            PwmIn::new(None, PioSmInstance::Pio0Sm0(sm0)),
                            PwmIn::new(None, PioSmInstance::Pio0Sm1(sm1)),
                            PwmIn::new(None, PioSmInstance::Pio0Sm2(sm2)),
                            PwmIn::new(None, PioSmInstance::Pio0Sm3(sm3)),
                            PwmIn::new(None, PioSmInstance::Pio1Sm0(sm4)),
                            PwmIn::new(None, PioSmInstance::Pio1Sm1(sm5)),
                            PwmIn::new(None, PioSmInstance::Pio1Sm2(sm6)),
                            PwmIn::new(None, PioSmInstance::Pio1Sm3(sm7)),
                            // PwmIn::new(None, 0, 0),
                            // PwmIn::new(None, 1, 0),
                            // PwmIn::new(None, 2, 0),
                            // PwmIn::new(None, 3, 0),
                            // PwmIn::new(None, 0, 1),
                            // PwmIn::new(None, 1, 1),
                            // PwmIn::new(None, 2, 1),
                            // PwmIn::new(None, 3, 1),
                        ]
        }
    }

    fn pin_in_use(&self, pin:usize) -> bool {
        for pwmin in &self.pwmin_state {
            if let Some(p) = pwmin.pin {
                if p == pin  && pwmin.run == true {
                    return true;
                }
            }
        }
        false
    }

    pub fn stop(&mut self, pin:usize) {
        for pwmin in &mut self.pwmin_state {
            if let Some(p) = pwmin.pin {
                if p == pin {
                    pwmin.pin = None;
                    pwmin.run = false;
                    //TODO: stop task
                }
            }
        }
    }
    fn create_pin(&self, pin:usize) -> Option<AnyPin> {
        if pin > 29 {
            return None;
        }

    }

    // fn create_sm(&self, pio_no:usize, sm_no:usize) -> impl PioStateMachine {
    // }

    pub async fn start(&mut self, pin:usize) -> Result<(), PwmInError> {
        if self.pin_in_use(pin) {
            Err(PwmInError::PinInUse)
        } else {
            for pwmin in &mut self.pwmin_state {
                if pwmin.pin.is_none() {
                    //found empty place
                    pwmin.pin = Some(pin);
                    pwmin.run = true;
                    let p = unsafe {Peripherals::steal()};
                    let pin = match pin {
                        0 =>  p.PIN_0.degrade(),
                        1 =>  p.PIN_1.degrade(),
                        2 =>  p.PIN_2.degrade(),
                        3 =>  p.PIN_3.degrade(),
                        4 =>  p.PIN_4.degrade(),
                        5 =>  p.PIN_5.degrade(),
                        6 =>  p.PIN_6.degrade(),
                        7 =>  p.PIN_7.degrade(),
                        8 =>  p.PIN_8.degrade(),
                        9 =>  p.PIN_9.degrade(),
                        10 => p.PIN_10.degrade(),
                        11 => p.PIN_11.degrade(),
                        12 => p.PIN_12.degrade(),
                        13 => p.PIN_13.degrade(),
                        14 => p.PIN_14.degrade(),
                        15 => p.PIN_15.degrade(),
                        16 => p.PIN_16.degrade(),
                        17 => p.PIN_17.degrade(),
                        18 => p.PIN_18.degrade(),
                        19 => p.PIN_19.degrade(),
                        20 => p.PIN_20.degrade(),
                        21 => p.PIN_21.degrade(),
                        22 => p.PIN_22.degrade(),
                        23 => p.PIN_23.degrade(),
                        24 => p.PIN_24.degrade(),
                        25 => p.PIN_25.degrade(),
                        26 => p.PIN_26.degrade(),
                        27 => p.PIN_27.degrade(),
                        28 => p.PIN_28.degrade(),
                        29 => p.PIN_29.degrade(),
                        _ =>  p.PIN_0.degrade(),
                    };
                    //TODO: create sm instance
                    // let sm = self.create_sm(pwmin.pio, pwmin.sm);
                    let sminstance = pwmin.sm;
                    match sminstance {
                        PioSmInstance::Pio0Sm0(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        PioSmInstance::Pio0Sm1(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        PioSmInstance::Pio0Sm2(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        PioSmInstance::Pio0Sm3(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        PioSmInstance::Pio1Sm0(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        PioSmInstance::Pio1Sm1(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        PioSmInstance::Pio1Sm2(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        PioSmInstance::Pio1Sm3(sm) => {Spawner::for_current_executor().await.spawn(pwmin_task(sm, pin));},
                        }
                    return Ok(());
                } else {
                    return Err(PwmInError::PinError);
                }
                    return Ok(());
            }
            Err(PwmInError::PinAllocFail)
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
                //check pin is already started?
            });
            Ok(())
        },
        "stop" => {
            //stop pwmin
            log::info!("stop pin:{}", sub_args);
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

#[embassy_executor::task]
pub async fn pwmin_task(mut sm: impl PioStateMachine, pin:AnyPin) {
    //setup msg
    let mut msg:PwmInfo = PwmInfo::default();
    msg.pin = pin.pin() as u32;
    let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();

    // setup sm
    let _wait_irq = sm.sm_no();
    let prg = pio_proc::pio_file!("./src/PwmIn.pio");
    let relocated = RelocatedProgram::new(&prg.program);

    let pin = sm.make_pio_pin(pin);
    sm.set_jmp_pin(pin.pin());
    sm.set_in_base_pin(&pin);

    sm.write_instr(relocated.origin() as usize, relocated.code());
    pio_instr_util::exec_jmp(&mut sm, relocated.origin());
    let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
    sm.set_clkdiv(clkdiv << 8);
    let pio::Wrap { source, target} = relocated.wrap();
    sm.set_wrap(source, target);

    sm.set_autopull(false);
    sm.set_fifo_join(FifoJoin::RxOnly);
    sm.set_in_shift_dir(ShiftDirection::Left);
    // sm.set_pull_threshold(2);
    sm.clear_fifos();
    sm.set_enable(true);

    let mut high_period:u32 = 0; 
    let mut low_period:u32 = 0; 
    let mut tmp_period_1:u32 = 0;
    let mut tmp_period_2:u32 = 0;
    loop {
        // sm.wait_irq(wait_irq).await;
        tmp_period_1 = sm.wait_pull().await;
        tmp_period_2 = sm.wait_pull().await;
        // sm.clear_fifos();
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
    }
}

// #[embassy_executor::task]
pub async fn pwmin_log_task() {
    let mut sub = PWM_PUBSUB_CHANNEL.subscriber().unwrap();

    loop {
        if let WaitResult::Message(msg) = sub.next_message().await {
            log::info!("[PwmIn]:{}:{}:{}:{}", msg.pin, msg.clk, msg.high_period, msg.low_period);
        }
    }
}
