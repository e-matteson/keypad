//! An example of how to use the macros in the `keypad` driver crate.
//!
//! This uses mock types that implement the `embeddded-hal` traits without using
//! any real hardware. It will compile and run on your host computer, but it
//! won't do anything interesting because there are no real buttons to press.

use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;
use keypad::mock_hal::{self, GpioExt, Input, OpenDrain, Output, PullUp, GPIOA};
use keypad::{keypad_new, keypad_struct};

// Define the struct that represents your keypad matrix. Give the specific pins
// that will be used for the rows and columns of your matrix - each pin number
// has a unique type. Rows must be input pins, and columns must be output
// pins. You can select the modes (PullUp/Floating/OpenDrain/PushPull) to suit
// your circuit.
keypad_struct! {
    pub struct ExampleKeypad<Error = Infallible> {
        rows: (
            mock_hal::gpioa::PA0<Input<PullUp>>,
            mock_hal::gpioa::PA1<Input<PullUp>>,
            mock_hal::gpioa::PA2<Input<PullUp>>,
            mock_hal::gpioa::PA3<Input<PullUp>>,
        ),
        columns: (
            mock_hal::gpioa::PA4<Output<OpenDrain>>,
            mock_hal::gpioa::PA5<Output<OpenDrain>>,
            mock_hal::gpioa::PA6<Output<OpenDrain>>,
            mock_hal::gpioa::PA7<Output<OpenDrain>>,
            mock_hal::gpioa::PA8<Output<OpenDrain>>,
        ),
    }
}

fn main() {
    // Get access to (mock) general-purpose input/output pins.
    let pins = GPIOA::split();

    // Create an instance of the keypad struct you defined above.
    let keypad = keypad_new!(ExampleKeypad {
        rows: (
            pins.pa0.into_pull_up_input(),
            pins.pa1.into_pull_up_input(),
            pins.pa2.into_pull_up_input(),
            pins.pa3.into_pull_up_input(),
        ),
        columns: (
            pins.pa4.into_open_drain_output(),
            pins.pa5.into_open_drain_output(),
            pins.pa6.into_open_drain_output(),
            pins.pa7.into_open_drain_output(),
            pins.pa8.into_open_drain_output(),
        ),
    });

    // Create a 2d array of virtual `KeyboardInput` pins, each representing 1 key in the
    // matrix. They implement the `InputPin` trait and can (mostly) be used
    // just like any other embedded-hal input pins.
    let keys = keypad.decompose();

    let first_key = &keys[0][0];
    println!("Is first key pressed? {:?}\n", first_key.is_low());

    // Print a table of which keys are pressed. This is a boring example because
    // we have no way to press the mock keys and they'll always stay unpressed.

    for (row_index, row) in keys.iter().enumerate() {
        print!("row {}: ", row_index);
        for key in row.iter() {
            let is_pressed = if key.is_low().unwrap() { 1 } else { 0 };
            print!(" {} ", is_pressed);
        }
        println!();
    }

    // Give up ownership of the row and column pins.
    let ((_r0, _r1, _r2, _r3), (_c0, _c1, _c2, _c3, _c4)) = keypad.release();
}
