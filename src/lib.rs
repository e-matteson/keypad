//! **Platform-agnostic driver for a generic keypad**
//!
//! This driver lets you read the state of any key in a keypad matrix as if it
//! was connected to a single input pin. It supports keypads of any size, and any
//! embedded platform that implements the
//! [embedded-hal](https://crates.io/crates/embedded-hal) traits.
//!
//! ## Motivation
//!
//! The simplest way to read keypresses with a microcontroller is to connect
//! each key to one input pin. However, that won't work if you have more keys
//! than available pins. One solution is to use a keypad matrix circuit that
//! lets you read from N*M keys using only N+M pins.
//!
//! [TODO drawing]
//!
//! A downside of this approach is that it increases code complexity. Instead of
//! reading a single input pin to check if a key is pressed, you need to
//! actively scan the matrix by driving a column low, reading a row, and setting
//! the column high/floating again.
//!
//! The purpose of this driver is to use the `embedded-hal` traits to hide that
//! complexity. It does this by giving you a set of virtual `KeyInput` pins, each
//! of which represent one key in your keypad matrix. Because they implement the
//! `InputPin` trait, you can treat each one like a single input pin, without
//! worrying about the matrix-scanning that happens under the hood.
//!
//! This approach was inspired by the
//! [shift-register-driver](https://github.com/JoshMcguigan/shift-register-driver)
//! crate, which uses virtual output pins to simplify the use of a shift
//! register.
//!
//! ## Limitations
//!
//! - Reading the key state is not reentrant.
//!
//! - This is not optimized for scanning through the entire keypad as quickly as
//! possible. That's a tradeoff that comes from treating each key
//! as an independent input.
//!
//!
//! ## Example
//!
//! This example uses mock types that implement the `embeddded-hal` traits
//! without using any real hardware. It will compile and run on your host
//! computer, but it won't do anything interesting because there are no real
//! buttons to press.
//!
//! See the `example` crate for documentation of the `ExampleKeypad` struct
//! generated here.
//!
//! ```
//! #![feature(nll)]
//! #[macro_use]
//! extern crate keypad;
//! extern crate core;
//!
//! use keypad::embedded_hal::digital::InputPin;
//! use keypad::mock_hal::{self, GpioExt, Input, OpenDrain, Output, PullUp, GPIOA};
//!
//! // Define the struct that represents your keypad matrix circuit, picking the
//! // row and column pin numbers.
//! keypad_struct!{
//!     struct ExampleKeypad {
//!         rows: (
//!             mock_hal::gpioa::PA0<Input<PullUp>>,
//!             mock_hal::gpioa::PA1<Input<PullUp>>,
//!             mock_hal::gpioa::PA2<Input<PullUp>>,
//!             mock_hal::gpioa::PA3<Input<PullUp>>,
//!         ),
//!         columns: (
//!             mock_hal::gpioa::PA4<Output<OpenDrain>>,
//!             mock_hal::gpioa::PA5<Output<OpenDrain>>,
//!             mock_hal::gpioa::PA6<Output<OpenDrain>>,
//!             mock_hal::gpioa::PA7<Output<OpenDrain>>,
//!             mock_hal::gpioa::PA8<Output<OpenDrain>>,
//!         ),
//!     }
//! }
//!
//! fn main() {
//!     let pins = GPIOA::split();
//!
//!     // Create an instance of the keypad struct you defined above.
//!     let keypad = keypad_new!(ExampleKeypad {
//!         rows: (
//!             pins.pa0.into_pull_up_input(),
//!             pins.pa1.into_pull_up_input(),
//!             pins.pa2.into_pull_up_input(),
//!             pins.pa3.into_pull_up_input(),
//!         ),
//!         columns: (
//!             pins.pa4.into_open_drain_output(),
//!             pins.pa5.into_open_drain_output(),
//!             pins.pa6.into_open_drain_output(),
//!             pins.pa7.into_open_drain_output(),
//!             pins.pa8.into_open_drain_output(),
//!         ),
//!     });
//!
//!     // Create a 2d array of virtual `KeyboardInput` pins, each representing 1 key in the
//!     // matrix. They implement the `InputPin` trait and can (mostly) be used
//!     // just like any other embedded-hal input pins.
//!     let keys = keypad.decompose();
//!
//!     let first_key = &keys[0][0];
//!     println!("Is first key pressed? {}\n", first_key.is_low());
//!
//!     // Print a table of which keys are pressed.
//!
//!     for (row_index, row) in keys.iter().enumerate() {
//!         print!("row {}: ", row_index);
//!         for key in row.iter() {
//!             let is_pressed = if key.is_low() { 1 } else { 0 };
//!             print!(" {} ", is_pressed);
//!         }
//!         println!();
//!     }
//!
//!     // Give up ownership of the row and column pins.
//!     let ((_r0, _r1, _r2, _r3), (_c0, _c1, _c2, _c3, _c4)) = keypad.release();
//! }
//! ```
//!

#![no_std]
#![warn(missing_docs)]

pub mod mock_hal;

/// Re-export, so macros can import things from here instead of requiring the
/// application to directly use the `embedded_hal` crate too.
pub extern crate embedded_hal;

use core::cell::RefCell;
use embedded_hal::digital::{InputPin, OutputPin};

/// A virtual `embedded-hal` input pin representing one key of the keypad.
///
/// A `KeypadInput` stores references to one row and one column pin. When you
/// read from it with `.is_low()` or `.is_high()`, it secretly sets the column
/// pin low, reads from the row pin, and then sets the column pin high again.
/// The column pin is actually stored inside a `RefCell` in the keypad struct,
/// so that multiple `KeypadInput`s can mutate the column pin's state as needed,
/// even though they only have a shared/immutable reference to it.
///
/// This has several implications.
///
/// 1) Reading from `KeypadInput`s is not reentrant. If we were in the middle
/// of reading a `KeypadInput` and entered an interrupt service routine that
/// read any `KeypadInput` of the same keypad, we might read an incorrect value
/// or cause a `panic`.
///
/// 2) Reading from a `KeypadInput` is slower than reading from a real input
/// pin, because it needs to change the output pin state twice for every read.
pub struct KeypadInput<'a> {
    row: &'a InputPin,
    col: &'a RefCell<OutputPin>,
}

impl<'a> KeypadInput<'a> {
    /// Create a new `KeypadInput`. For use in macros.
    pub fn new(row: &'a InputPin, col: &'a RefCell<OutputPin>) -> Self {
        Self { row, col }
    }
}

impl<'a> InputPin for KeypadInput<'a> {
    /// Read the state of the key at this row and column. Not reentrant.
    fn is_high(&self) -> bool {
        !self.is_low()
    }

    /// Read the state of the key at this row and column. Not reentrant.
    fn is_low(&self) -> bool {
        self.col.borrow_mut().set_low();
        let out = self.row.is_low();
        self.col.borrow_mut().set_high();
        out
    }
}

/// Define a new struct representing your keypad matrix circuit.
///
/// Every pin has a unique type, depending on its pin number and its current
/// mode. This struct is where you specify which pin types will be used in the rows
/// and columns of the keypad matrix. All the row pins must implement the
/// `InputPin` trait, and the column pins must implement the `OutputPin` trait.
///
/// You can specify the visibility of the struct (eg. `pub`) as usual.
///
/// This macro will implement the `decompose()` and `release()` methods for your
/// struct. To view documentation for those methods, see the `example` crate.
///
/// # Example
///
/// ```
/// # extern crate core;
/// #[macro_use]
/// extern crate keypad;
/// use keypad::mock_hal::{self, Input, Output, PullUp, OpenDrain};
///
/// keypad_struct!{
///     struct MyKeypad {
///         rows: (
///             mock_hal::gpioa::PA0<Input<PullUp>>,
///             mock_hal::gpioa::PA1<Input<PullUp>>,
///         ),
///         columns: (
///             mock_hal::gpioa::PA2<Output<OpenDrain>>,
///             mock_hal::gpioa::PA3<Output<OpenDrain>>,
///             mock_hal::gpioa::PA4<Output<OpenDrain>>,
///         ),
///     }
/// }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! keypad_struct {
    (
      $visibility:vis struct $struct_name:ident {
            rows: ( $($row_type:ty),* $(,)* ),
            columns: ( $($col_type:ty),* $(,)* ),
        }
    ) => {
        /// A struct that owns the row and column pins of your keypad matrix
        /// circuit, generated with the `keypad_struct!` macro.
        $visibility struct $struct_name {
            /// The input pins used for reading each row.
            rows: ($($row_type),* ,),
            /// The output pins used for scanning through each column. They're
            /// wrapped in RefCells so that we can change their state even if we
            /// only have shared/immutable reference to them. This lets us
            /// actively scan the matrix when reading the state of a virtual
            /// `KeypadInput` pin.
            columns: ($(::core::cell::RefCell<$col_type>),* ,),
        }

        impl $struct_name {
            /// Get a 2d array of embedded-hal input pins, each representing one
            /// key in the keypad matrix.
            #[allow(dead_code)]
            $visibility fn decompose<'a>(&'a self) ->
                keypad_struct!(
                    @array2d_type
                        ($($row_type),*)
                        ($(::core::cell::RefCell<$col_type>),*)
                )
            {

                let rows: [
                    &::keypad::embedded_hal::digital::InputPin;
                    keypad_struct!(@count $($row_type)*)
                ]
                    = keypad_struct!(@tuple  self.rows,  ($($row_type),*));

                let columns: [
                    &::core::cell::RefCell<::keypad::embedded_hal::digital::OutputPin>;
                    keypad_struct!(@count $($col_type)*)
                ]
                    = keypad_struct!(@tuple  self.columns,  ($($col_type),*));

                let mut out: keypad_struct!(
                    @array2d_type
                        ($($row_type),*)
                        ($(::core::cell::RefCell<$col_type>),*)
                ) = unsafe {
                    ::core::mem::uninitialized()
                };

                for r in 0..rows.len() {
                    for c in 0..columns.len() {
                        out[r][c] = ::keypad::KeypadInput::new(rows[r], columns[c]);
                    }
                }
                out
            }

            /// Give back ownership of the row and column pins.
            ///
            /// This consumes the keypad struct. All references to its virtual
            /// `KeypadInput` pins must have gone out of scope before you try to
            /// call `.release()`, or it will fail to compile. Opting in to the
            /// non-lexical lifetimes feature in your project can make that
            /// simpler.
            ///
            /// The column pins will be returned inside of `RefCell`s (because
            /// macros are hard). You can use `.into_inner()` to extract
            /// each column pin from its `RefCell`.
            #[allow(dead_code)]
            $visibility fn release(self) ->(($($row_type),* ,), ($(::core::cell::RefCell<$col_type>),* ,)) {
                (self.rows, self.columns)
            }
        }
    };
    (@array2d_type ($($row:ty),*) ($($col:ty),*) ) => {
        [keypad_struct!(@array1d_type ($($col),*)) ; keypad_struct!(@count $($row)*)]
    };
    (@array1d_type ($($col:ty),*)) => {
        [keypad_struct!(@element_type) ; keypad_struct!(@count $($col)*)]
    };
    (@element_type ) => {
        ::keypad::KeypadInput<'a>
    };
    (@count $($token_trees:tt)*) => {
        0usize $(+ keypad_struct!(@replace $token_trees 1usize))*
    };
    (@replace $_t:tt $sub:expr) => {
        $sub
    };
    (@underscore $unused:tt) => {
        _
    };
    (@destructure_ref $tuple:expr, ($($repeat_n:ty),*)) => {
        {
            let (
                $(keypad_struct!(@underscore $repeat_n),)*
                    ref nth, ..) = $tuple;
            nth
        }
    };
    (@tuple_helper $tuple:expr, ($head:ty), ($($result:expr),*  $(,)*)) => {
        [
            keypad_struct!(@destructure_ref $tuple, ()),
            $($result),*
        ]
    };
    (@tuple_helper $tuple:expr, ($head:ty $(,$repeats:ty)* $(,)*),  ($($result:expr),*  $(,)*)) => {
        keypad_struct!(
            @tuple_helper $tuple, ($($repeats),*),
            (
                keypad_struct!(@destructure_ref $tuple, ($($repeats),*)),
                $($result),*
            )
        )
    };
    (@tuple $tuple:expr, ($($repeats:ty),*)) => {
        keypad_struct!(@tuple_helper $tuple, ($($repeats),*) , ())
    };
}

/// Create an instance of the struct you defined with `keypad_struct!()`.
///
/// The pins will need to match the types you specified in the `keypad_struct!()` macro.
///
/// ```
/// # extern crate core;
/// # #[macro_use]
/// # extern crate keypad;
/// # use keypad::mock_hal::{GPIOA, GpioExt};
/// # use keypad::mock_hal::{self, Input, Output, PullUp, OpenDrain};
/// # keypad_struct!{
/// #     struct MyKeypad {
/// #         rows: (
/// #             mock_hal::gpioa::PA0<Input<PullUp>>,
/// #             mock_hal::gpioa::PA1<Input<PullUp>>,
/// #         ),
/// #         columns: (
/// #             mock_hal::gpioa::PA2<Output<OpenDrain>>,
/// #             mock_hal::gpioa::PA3<Output<OpenDrain>>,
/// #             mock_hal::gpioa::PA4<Output<OpenDrain>>,
/// #         ),
/// #     }
/// # }
/// # fn main() {
/// let pins = GPIOA::split();
///
/// let keypad = keypad_new!(MyKeypad {
///     rows: (
///         pins.pa0.into_pull_up_input(),
///         pins.pa1.into_pull_up_input(),
///     ),
///     columns: (
///         pins.pa2.into_open_drain_output(),
///         pins.pa3.into_open_drain_output(),
///         pins.pa4.into_open_drain_output(),
///     ),
/// });
/// # }
/// ```

#[macro_export]
macro_rules! keypad_new {
    ( $struct_name:ident {
        rows: ( $($row_val:expr),* $(,)* ),
        columns: ( $($col_val:expr),* $(,)* ),
    }) => {
        $struct_name {
            rows:  ($($row_val),* ,),
            columns:  ($(::core::cell::RefCell::new($col_val)),* ,),
        }
    };
}
