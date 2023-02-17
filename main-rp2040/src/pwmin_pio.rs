use {defmt_rtt as _, panic_probe as _};
use embassy_rp::gpio::{AnyPin, Pin};
use embassy_rp::pio::{Pio0, Pio1, PioStateMachine, PioStateMachineInstance, 
                      ShiftDirection, Sm0, Sm1, Sm2, Sm3,FifoJoin};
use embassy_rp::pio_instr_util;
use embassy_rp::relocate::RelocatedProgram;
use {defmt_rtt as _, panic_probe as _};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pubsub::WaitResult;
use embassy_sync::pubsub::PubSubChannel;
use embassy_time::{Duration, Timer};

const SM_CLK:u32 = 125_000_000; //125MHz
static PWM_PUBSUB_CHANNEL:PubSubChannel::<ThreadModeRawMutex, PwmInfo, 200, 1, 5> = PubSubChannel::new();

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

// macro_rules! impl_pwmin_pio {
//     ($pio:ident, $sm:ident, $fn:ident) => {
//         #[embassy_executor::task]
//         pub async fn $fn(mut sm: PioStateMachineInstance<$pio, $sm>, pin:AnyPin) {
//             //setup msg
//             let mut msg:PwmInfo = PwmInfo::default();
//             msg.pin = pin.pin() as u32;
//             let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();

//             // setup sm
//             let _wait_irq = sm.sm_no();
//             let prg = pio_proc::pio_file!("./src/PwmIn.pio");
//             let relocated = RelocatedProgram::new(&prg.program);

//             let pin = sm.make_pio_pin(pin);
//             sm.set_jmp_pin(pin.pin());
//             sm.set_in_base_pin(&pin);

//             sm.write_instr(relocated.origin() as usize, relocated.code());
//             pio_instr_util::exec_jmp(&mut sm, relocated.origin());
//             let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
//             sm.set_clkdiv(clkdiv << 8);
//             let pio::Wrap { source, target} = relocated.wrap();
//             sm.set_wrap(source, target);

//             sm.set_autopull(false);
//             sm.set_fifo_join(FifoJoin::RxOnly);
//             sm.set_in_shift_dir(ShiftDirection::Left);
//             // sm.set_pull_threshold(2);
//             sm.clear_fifos();
//             sm.set_enable(true);

//             let mut high_period:u32 = 0; 
//             let mut low_period:u32 = 0; 
//             let mut tmp_period_1:u32 = 0;
//             let mut tmp_period_2:u32 = 0;
//             loop {
//                 // sm.wait_irq(wait_irq).await;
//                 tmp_period_1 = sm.wait_pull().await;
//                 tmp_period_2 = sm.wait_pull().await;
//                 // sm.clear_fifos();
//                 if tmp_period_1 & 0xF0000000 != 0 {
//                     //tmp_period_1 is low_period
//                     high_period = tmp_period_2 * 2;
//                     low_period = tmp_period_1 * 2;
//                 } else {
//                     //tmp_period_1 is high_period
//                     low_period = tmp_period_2 * 2;
//                     high_period = tmp_period_1 * 2;
//                 }
//                 if ((high_period / 10) != (msg.high_period / 10)) || ((low_period / 10) != (msg.low_period / 10)) {
//                     msg.high_period = high_period;
//                     msg.low_period = low_period;
//                     publisher.publish_immediate(msg);
//                 }
//             }
//         }
//     };
// }

// impl_pwmin_pio!(Pio0, Sm0, pio0_sm0_pwmin_task);
// impl_pwmin_pio!(Pio0, Sm1, pio0_sm1_pwmin_task);
// impl_pwmin_pio!(Pio0, Sm2, pio0_sm2_pwmin_task);
// impl_pwmin_pio!(Pio0, Sm3, pio0_sm3_pwmin_task);
// impl_pwmin_pio!(Pio1, Sm0, pio1_sm0_pwmin_task);

// #[embassy_executor::task]
// pub async fn pio0_task_sm0(mut sm: PioStateMachineInstance<Pio0, Sm0>, pin:AnyPin) {
//     //setup msg
//     let mut msg:PwmInfo = PwmInfo::default();
//     msg.pin = pin.pin() as u32;
//     let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();

//     // setup sm
//     let wait_irq = sm.sm_no();
//     let prg = pio_proc::pio_file!("./src/PwmIn.pio");
//     let relocated = RelocatedProgram::new(&prg.program);

//     let pin = sm.make_pio_pin(pin);
//     sm.set_jmp_pin(pin.pin());
//     sm.set_in_base_pin(&pin);

//     sm.write_instr(relocated.origin() as usize, relocated.code());
//     pio_instr_util::exec_jmp(&mut sm, relocated.origin());
//     let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
//     sm.set_clkdiv(clkdiv << 8);
//     let pio::Wrap { source, target} = relocated.wrap();
//     sm.set_wrap(source, target);

//     sm.set_autopull(false);
//     sm.set_fifo_join(FifoJoin::RxOnly);
//     sm.set_in_shift_dir(ShiftDirection::Left);
//     // sm.set_pull_threshold(2);
//     sm.clear_fifos();
//     sm.set_enable(true);

//     let mut high_period= 0; 
//     let mut low_period = 0; 

//     loop {
//         // sm.wait_irq(wait_irq).await;
//         high_period = sm.wait_pull().await * 2;
//         low_period = sm.wait_pull().await * 2;
//         sm.clear_fifos();
//         if ((high_period / 10) != (msg.high_period / 10)) || ((low_period / 10) != (msg.low_period / 10)) {
//             msg.high_period = high_period;
//             msg.low_period = low_period;
//             publisher.publish_immediate(msg);
//         }
//     }
// }

// #[embassy_executor::task]
// pub async fn pio0_task_sm1(mut sm: PioStateMachineInstance<Pio0, Sm1>, pin:AnyPin) {
//     //setup msg
//     let mut msg:PwmInfo = PwmInfo::default();
//     msg.pin = pin.pin() as u32;
//     let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();

//     // setup sm
//     let wait_irq = sm.sm_no();
//     let prg = pio_proc::pio_file!("./src/PwmIn.pio");
//     let relocated = RelocatedProgram::new(&prg.program);

//     let pin = sm.make_pio_pin(pin);
//     sm.set_jmp_pin(pin.pin());
//     sm.set_in_base_pin(&pin);

//     sm.write_instr(relocated.origin() as usize, relocated.code());
//     pio_instr_util::exec_jmp(&mut sm, relocated.origin());
//     let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
//     sm.set_clkdiv(clkdiv << 8);
//     let pio::Wrap { source, target} = relocated.wrap();
//     sm.set_wrap(source, target);

//     sm.set_autopull(true);
//     sm.set_in_shift_dir(ShiftDirection::Left);
//     sm.set_enable(true);

//     let mut high_period= 0; 
//     let mut low_period = 0; 

//     loop {
//         sm.wait_irq(wait_irq).await;
//         high_period = sm.wait_pull().await * 2;
//         low_period = sm.wait_pull().await * 2;
//         sm.clear_fifos();
//         if ((high_period / 10) != (msg.high_period / 10)) || ((low_period / 10) != (msg.low_period / 10)) {
//             msg.high_period = high_period;
//             msg.low_period = low_period;
//             publisher.publish_immediate(msg);
//         }
//     }
// }

// #[embassy_executor::task]
// pub async fn pio0_task_sm2(mut sm: PioStateMachineInstance<Pio0, Sm2>, pin:AnyPin) {
//     //setup msg
//     let mut msg:PwmInfo = PwmInfo::default();
//     msg.pin = pin.pin() as u32;
//     let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();

//     // setup sm
//     let wait_irq = sm.sm_no();
//     let prg = pio_proc::pio_file!("./src/PwmIn.pio");
//     let relocated = RelocatedProgram::new(&prg.program);

//     let pin = sm.make_pio_pin(pin);
//     sm.set_jmp_pin(pin.pin());
//     sm.set_in_base_pin(&pin);

//     sm.write_instr(relocated.origin() as usize, relocated.code());
//     pio_instr_util::exec_jmp(&mut sm, relocated.origin());
//     let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
//     sm.set_clkdiv(clkdiv << 8);
//     let pio::Wrap { source, target} = relocated.wrap();
//     sm.set_wrap(source, target);

//     sm.set_autopull(true);
//     sm.set_in_shift_dir(ShiftDirection::Left);
//     sm.set_enable(true);

//     let mut high_period= 0; 
//     let mut low_period = 0; 
//     loop {
//         sm.wait_irq(wait_irq).await;
//         high_period = sm.wait_pull().await * 2;
//         low_period = sm.wait_pull().await * 2;
//         sm.clear_fifos();
//         if ((high_period / 10) != (msg.high_period / 10)) || ((low_period / 10) != (msg.low_period / 10)) {
//             msg.high_period = high_period;
//             msg.low_period = low_period;
//             publisher.publish_immediate(msg);
//         }
//     }
// }

// #[embassy_executor::task]
// pub async fn pio0_task_sm3(mut sm: PioStateMachineInstance<Pio0, Sm3>, pin:AnyPin) {
//     //setup msg
//     let mut msg:PwmInfo = PwmInfo::default();
//     msg.pin = pin.pin() as u32;
//     let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();

//     // setup sm
//     let wait_irq = sm.sm_no();
//     let prg = pio_proc::pio_file!("./src/PwmIn.pio");
//     let relocated = RelocatedProgram::new(&prg.program);

//     let pin = sm.make_pio_pin(pin);
//     sm.set_jmp_pin(pin.pin());
//     sm.set_in_base_pin(&pin);

//     sm.write_instr(relocated.origin() as usize, relocated.code());
//     pio_instr_util::exec_jmp(&mut sm, relocated.origin());
//     let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
//     sm.set_clkdiv(clkdiv << 8);
//     let pio::Wrap { source, target} = relocated.wrap();
//     sm.set_wrap(source, target);

//     sm.set_autopull(true);
//     sm.set_in_shift_dir(ShiftDirection::Left);
//     sm.set_enable(true);

//     let mut high_period= 0; 
//     let mut low_period = 0; 

//     loop {
//         sm.wait_irq(wait_irq).await;
//         high_period = sm.wait_pull().await * 2;
//         low_period = sm.wait_pull().await * 2;
//         sm.clear_fifos();
//         if ((high_period / 10) != (msg.high_period / 10)) || ((low_period / 10) != (msg.low_period / 10)) {
//             msg.high_period = high_period;
//             msg.low_period = low_period;
//             publisher.publish_immediate(msg);
//         }
//     }
// }

// #[embassy_executor::task]
// pub async fn pio1_task_sm0(mut sm: PioStateMachineInstance<Pio1, Sm0>, pin:AnyPin) {
//     //setup msg
//     let mut msg:PwmInfo = PwmInfo::default();
//     msg.pin = pin.pin() as u32;
//     let publisher = PWM_PUBSUB_CHANNEL.publisher().unwrap();

//     // setup sm
//     let wait_irq = sm.sm_no();
//     let prg = pio_proc::pio_file!("./src/PwmIn.pio");
//     let relocated = RelocatedProgram::new(&prg.program);

//     let pin = sm.make_pio_pin(pin);
//     sm.set_jmp_pin(pin.pin());
//     sm.set_in_base_pin(&pin);

//     sm.write_instr(relocated.origin() as usize, relocated.code());
//     pio_instr_util::exec_jmp(&mut sm, relocated.origin());
//     let clkdiv:u32 = (125e6 / (SM_CLK as f32)) as u32;
//     sm.set_clkdiv(clkdiv << 8);
//     let pio::Wrap { source, target} = relocated.wrap();
//     sm.set_wrap(source, target);

//     sm.set_autopull(false);
//     sm.set_in_shift_dir(ShiftDirection::Left);
//     sm.set_fifo_join(FifoJoin::RxOnly);
//     sm.set_enable(true);

//     let mut high_period= 0; 
//     let mut low_period = 0; 

//     loop {
//         sm.wait_irq(wait_irq).await;
//         high_period = sm.wait_pull().await * 2;
//         low_period = sm.wait_pull().await * 2;
//         sm.clear_fifos();
//         if ((high_period / 10) != (msg.high_period / 10)) || ((low_period / 10) != (msg.low_period / 10)) {
//             msg.high_period = high_period;
//             msg.low_period = low_period;
//             publisher.publish_immediate(msg);
//         }
//     }
// }

#[embassy_executor::task]
pub async fn pwmin_log_task() {
    let mut sub = PWM_PUBSUB_CHANNEL.subscriber().unwrap();

    loop {
        if let WaitResult::Message(msg) = sub.next_message().await {
            log::info!("[PwmIn]:{}:{}:{}:{}", msg.pin, msg.clk, msg.high_period, msg.low_period);
        }
    }
}
