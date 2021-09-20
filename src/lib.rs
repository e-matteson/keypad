//! **Platform-agnostic driver for keypad matrix circuits**
//!
//! This driver lets you read the state of any key in a keypad matrix as if it
//! was connected to a single input pin. It supports keypads of any size, and any
//! embedded platform that implements the Rust
//! [embedded-hal](https://crates.io/crates/embedded-hal) traits.
//!
//! ## Motivation
//!
//! The simplest way to read keypresses with a microcontroller is to connect
//! each key to one input pin. However, that won't work if you have more keys
//! than available pins. One solution is to use a keypad matrix circuit that
//! lets you read from N*M keys using only N+M pins.
//!
//! ![matrix](https://raw.githubusercontent.com/e-matteson/keypad/58d087473246cdbf232b2831f9fc18c0a7a29fc7/matrix_schem.png)
//!
//! In this circuit, each row is an input pin with a pullup resistor, and each
//! column is an open-drain output pin. You read the state of a particular key by
//! driving its column pin low and reading its row pin.
//!
//! A downside of this approach is that it increases code complexity. Instead of
//! reading a single input pin to check if a key is pressed, you need to
//! actively scan the matrix by driving a column low, reading a row, and setting
//! the column high/floating again.
//!
//! The purpose of this driver is to use the `embedded-hal` traits to hide that
//! complexity. It does this by giving you a set of virtual `KeyInput` pins, each
//! of which represent one key in your keypad matrix. Because they implement the
//! `InputPin` trait, you can treat each one like a normal input pin - even
//! though the driver is secretly scanning the matrix for you whenever you read
//! from it. This approach was inspired by the
//! [shift-register-driver](https://github.com/JoshMcguigan/shift-register-driver)
//! crate, which lets you control a shift register through virtual output pins.
//!
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
//! For an example that runs on an actual microcontroller, see
//! [keypad-bluepill-example](https://github.com/e-matteson/keypad-bluepill-example).
//!
//! ```
//! # #![cfg_attr(docs_rs_workaround, feature(macro_vis_matcher))]
//! #![feature(nll)]
//! #[macro_use]
//! extern crate keypad;
//!
//! use keypad::embedded_hal::digital::InputPin;
//! use keypad::mock_hal::{self, GpioExt, Input, OpenDrain, Output, PullUp, GPIOA};
//!
//! // Define the struct that represents your keypad matrix circuit,
//! // picking the row and column pin numbers.
//! keypad_struct!{
//!     pub struct ExampleKeypad {
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
//!     // Create a 2d array of virtual `KeyboardInput` pins, each
//!     // representing 1 key in the matrix. They implement the
//!     // `InputPin` trait and can be used like other embedded-hal
//!     // input pins.
//!     let keys = keypad.decompose();
//!
//!     let first_key = &keys[0][0];
//!     println!("Is first key pressed? {}\n", first_key.is_low());
//!
//!     // Print a table showing whether each key is pressed.
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
// Workaround needed as long as docs.rs is using rustc <1.30
#![cfg_attr(docs_rs_workaround, feature(macro_vis_matcher))]

/// Re-export, so the macros and the user can import the InputPin and OutputPin
/// traits from here without requiring `extern crate embedded_hal` downstream.
pub extern crate embedded_hal;

// Re-export libcore using an alias so that the macros can work without
// requiring `extern crate core` downstream.
#[doc(hidden)]
pub extern crate core as _core;

pub mod mock_hal;

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
    row: &'a dyn InputPin,
    col: &'a RefCell<dyn OutputPin>,
}

impl<'a> KeypadInput<'a> {
    /// Create a new `KeypadInput`. For use in macros.
    pub fn new(row: &'a dyn InputPin, col: &'a RefCell<dyn OutputPin>) -> Self {
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
/// You can specify the visibility of the struct (eg. `pub`) as usual, and add
/// doc comments using the `#[doc="..."]` attribute.
///
/// Don't access or modify the struct's fields directly. Instead, use
/// the methods implemented by this macro, documented here:
/// [`example_generated::ExampleKeypad`](./example_generated/struct.ExampleKeypad.html)
///
/// # Example
///
/// ```
/// # #![cfg_attr(docs_rs_workaround, feature(macro_vis_matcher))]
/// #[macro_use]
/// extern crate keypad;
///
/// use keypad::mock_hal::{self, Input, OpenDrain, Output, PullUp};
///
/// keypad_struct!{
///     #[doc="My super-special keypad."]
///     pub struct ExampleKeypad {
///         rows: (
///             mock_hal::gpioa::PA0<Input<PullUp>>,
///             mock_hal::gpioa::PA1<Input<PullUp>>,
///             mock_hal::gpioa::PA2<Input<PullUp>>,
///             mock_hal::gpioa::PA3<Input<PullUp>>,
///         ),
///         columns: (
///             mock_hal::gpioa::PA4<Output<OpenDrain>>,
///             mock_hal::gpioa::PA5<Output<OpenDrain>>,
///             mock_hal::gpioa::PA6<Output<OpenDrain>>,
///             mock_hal::gpioa::PA7<Output<OpenDrain>>,
///             mock_hal::gpioa::PA8<Output<OpenDrain>>,
///         ),
///     }
/// }
///
/// # fn main() {
/// # }
/// ```
///
/// # Safety
///
/// This macro uses `unsafe` to create an array with uninitialized memory, which
/// is then immediately initialized in a loop. This is fine as long as there is
/// not a bug in how the macro calculates the dimensions of the array.

// This macro is complicated because it's stupidly hard to index into a tuple.
// The only way I found is to construct a different pattern for each field, and
// then repeatedly destructure the tuple with each pattern. Luckily we're only
// taking references to the fields, or it would get even harder.
#[macro_export]
macro_rules! keypad_struct {
    (
        $(#[$attributes:meta])* $visibility:vis struct $struct_name:ident {
            rows: ( $($row_type:ty),* $(,)* ),
            columns: ( $($col_type:ty),* $(,)* ),
        }
    ) => {
        $(#[$attributes])* $visibility struct $struct_name {
            /// The input pins used for reading each row.
            rows: ($($row_type),* ,),
            /// The output pins used for scanning through each column. They're
            /// wrapped in RefCells so that we can change their state even if we
            /// only have shared/immutable reference to them. This lets us
            /// actively scan the matrix when reading the state of a virtual
            /// `KeypadInput` pin.
            columns: ($($crate::_core::cell::RefCell<$col_type>),* ,),
        }

        impl $struct_name {
            /// Get a 2d array of embedded-hal input pins, each representing one
            /// key in the keypad matrix.
            #[allow(dead_code)]
            $visibility fn decompose<'a>(&'a self) ->
                keypad_struct!(
                    @array2d_type
                        ($($row_type),*)
                        ($($crate::_core::cell::RefCell<$col_type>),*)
                )
            {

                let rows: [
                    &$crate::embedded_hal::digital::InputPin;
                    keypad_struct!(@count $($row_type)*)
                ]
                    = keypad_struct!(@tuple  self.rows,  ($($row_type),*));

                let columns: [
                    &$crate::_core::cell::RefCell<$crate::embedded_hal::digital::OutputPin>;
                    keypad_struct!(@count $($col_type)*)
                ]
                    = keypad_struct!(@tuple  self.columns,  ($($col_type),*));

                let mut out: keypad_struct!(
                    @array2d_type
                        ($($row_type),*)
                        ($($crate::_core::cell::RefCell<$col_type>),*)
                ) = unsafe {
                    $crate::_core::mem::uninitialized()
                };

                for r in 0..rows.len() {
                    for c in 0..columns.len() {
                        out[r][c] = $crate::KeypadInput::new(rows[r], columns[c]);
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
            $visibility fn release(self) ->(($($row_type),* ,), ($($crate::_core::cell::RefCell<$col_type>),* ,)) {
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
        $crate::KeypadInput<'a>
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

/// Create an instance of the struct you defined with the `keypad_struct!()` macro..
///
/// The pin numbers and modes will need to match the ones you specified with `keypad_struct!()`.
///
/// ```
/// # #![cfg_attr(docs_rs_workaround, feature(macro_vis_matcher))]
/// # #[macro_use]
/// # extern crate keypad;
/// # use keypad::mock_hal::{self, Input, OpenDrain, Output, PullUp};
/// # use keypad::mock_hal::{GpioExt, GPIOA};
/// # keypad_struct!{
/// #     pub struct ExampleKeypad {
/// #         rows: (
/// #             mock_hal::gpioa::PA0<Input<PullUp>>,
/// #             mock_hal::gpioa::PA1<Input<PullUp>>,
/// #             mock_hal::gpioa::PA2<Input<PullUp>>,
/// #             mock_hal::gpioa::PA3<Input<PullUp>>,
/// #         ),
/// #         columns: (
/// #             mock_hal::gpioa::PA4<Output<OpenDrain>>,
/// #             mock_hal::gpioa::PA5<Output<OpenDrain>>,
/// #             mock_hal::gpioa::PA6<Output<OpenDrain>>,
/// #             mock_hal::gpioa::PA7<Output<OpenDrain>>,
/// #             mock_hal::gpioa::PA8<Output<OpenDrain>>,
/// #         ),
/// #     }
/// # }
/// # fn main() {
/// let pins = GPIOA::split();
///
/// let keypad = keypad_new!(ExampleKeypad {
///     rows: (
///         pins.pa0.into_pull_up_input(),
///         pins.pa1.into_pull_up_input(),
///         pins.pa2.into_pull_up_input(),
///         pins.pa3.into_pull_up_input(),
///     ),
///     columns: (
///         pins.pa4.into_open_drain_output(),
///         pins.pa5.into_open_drain_output(),
///         pins.pa6.into_open_drain_output(),
///         pins.pa7.into_open_drain_output(),
///         pins.pa8.into_open_drain_output(),
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
            columns:  ($($crate::_core::cell::RefCell::new($col_val)),* ,),
        }
    };
}

#[cfg(feature = "example_generated")]
pub mod example_generated;

// #[cfg(feature = "example_generated")]
// pub use example_generated::ExampleKeypad;
