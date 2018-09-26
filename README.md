[![Latest version](https://img.shields.io/crates/v/keypad.svg)](https://crates.io/crates/keypad)
[![Documentation](https://docs.rs/keypad/badge.svg)](https://docs.rs/keypad)

# keypad

**Platform-agnostic driver for keypad matrix circuits**

This driver lets you read the state of any key in a keypad matrix as if it
was connected to a single input pin. It supports keypads of any size, and any
embedded platform that implements the Rust
[embedded-hal](https://crates.io/crates/embedded-hal) traits.

### Motivation

The simplest way to read keypresses with a microcontroller is to connect
each key to one input pin. However, that won't work if you have more keys
than available pins. One solution is to use a keypad matrix circuit that
lets you read from N*M keys using only N+M pins.

![matrix](https://raw.githubusercontent.com/e-matteson/keypad/58d087473246cdbf232b2831f9fc18c0a7a29fc7/matrix_schem.png)

In this circuit, each row is an input pin with a pullup resistor, and each
column is an open-drain output pin. You read the state of a particular key by
driving its column pin low and reading its row pin.

A downside of this approach is that it increases code complexity. Instead of
reading a single input pin to check if a key is pressed, you need to
actively scan the matrix by driving a column low, reading a row, and setting
the column high/floating again.

The purpose of this driver is to use the `embedded-hal` traits to hide that
complexity. It does this by giving you a set of virtual `KeyInput` pins, each
of which represent one key in your keypad matrix. Because they implement the
`InputPin` trait, you can treat each one like a single input pin, without
worrying about the matrix-scanning that happens under the hood.

This approach was inspired by the
[shift-register-driver](https://github.com/JoshMcguigan/shift-register-driver)
crate, which uses virtual output pins to control a shift register.

### Limitations

- Reading the key state is not reentrant.

- This is not optimized for scanning through the entire keypad as quickly as
possible. That's a tradeoff that comes from treating each key
as an independent input.


### Example

This example uses mock types that implement the `embeddded-hal` traits
without using any real hardware. It will compile and run on your host
computer, but it won't do anything interesting because there are no real
buttons to press.

For an example that runs on an actual microcontroller, see
[keypad-bluepill-example](https://github.com/e-matteson/keypad-bluepill-example).

```rust
#![feature(nll)]
#[macro_use]
extern crate keypad;

use keypad::embedded_hal::digital::InputPin;
use keypad::mock_hal::{self, GpioExt, Input, OpenDrain, Output, PullUp, GPIOA};

// Define the struct that represents your keypad matrix circuit,
// picking the row and column pin numbers.
keypad_struct!{
    pub struct ExampleKeypad {
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

    // Create a 2d array of virtual `KeyboardInput` pins, each
    // representing 1 key in the matrix. They implement the
    // `InputPin` trait and can be used like other embedded-hal
    // input pins.
    let keys = keypad.decompose();

    let first_key = &keys[0][0];
    println!("Is first key pressed? {}\n", first_key.is_low());

    // Print a table showing whether each key is pressed.

    for (row_index, row) in keys.iter().enumerate() {
        print!("row {}: ", row_index);
        for key in row.iter() {
            let is_pressed = if key.is_low() { 1 } else { 0 };
            print!(" {} ", is_pressed);
        }
        println!();
    }

    // Give up ownership of the row and column pins.
    let ((_r0, _r1, _r2, _r3), (_c0, _c1, _c2, _c3, _c4)) = keypad.release();
}
```



### License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
