//! Mock types that implement the `embeddded-hal` traits without using
//! any real hardware.
//!
//! They're used for writing example code that will run on non-embedded targets.
//!
//! Based on the [stm32f103xx_hal](https://github.com/japaric/stm32f103xx-hal)
//! implementation by Jorge Aparicio.

use core::marker::PhantomData;

/// The internal state of a mock input or output pin.
#[derive(Debug)]
enum State {
    High,
    Low,
    Float,
}

/// Input mode marker
#[derive(Debug)]
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Floating input marker
#[derive(Debug)]
pub struct Floating;

/// Pulled up input marker
#[derive(Debug)]
pub struct PullUp;

/// Output mode marker
#[derive(Debug)]
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Push/pull output marker
#[derive(Debug)]
pub struct PushPull;

/// Open drain output marker
#[derive(Debug)]
pub struct OpenDrain;

/// Extension trait to split a mock GPIO peripheral into independent pins
pub trait GpioExt {
    /// The type to split the GPIO into
    type Parts;

    /// Split the GPIO block into independent pins and registers
    fn split() -> Self::Parts;
}

/// Create a whole module around the given mock GPIO port struct. Define structs
/// for its pins and impl useful things.
macro_rules! gpio {
    ($PORT:ident, $port:ident,  [$( ($Pin:ident, $pin:ident, $default_mode:ty) ),+ $(,)* ]) => {
        /// A module containing a mock port of GPIO pins.
        pub mod $port {
            use super::{State, Input,Output, Floating, PushPull, OpenDrain, GpioExt, PullUp, $PORT};
            use core::marker::PhantomData;
            use embedded_hal::digital::v2::{InputPin, OutputPin};

            /// The pins of a mock GPIO port
            #[derive(Debug)]
            pub struct Parts {
                $(
                    #[allow(missing_docs)]
                    pub $pin: $Pin<$default_mode>,
                )+
            }

            impl GpioExt for $PORT {
                type Parts = Parts;

                fn split() -> Parts {
                    Self::Parts {
                        $(
                            $pin: $Pin::default(),
                        )+
                    }
                }
            }

            $(
                /// A mock GPIO pin in a particular mode.
                #[derive(Debug)]
                pub struct $Pin<MODE> {
                    state: State,
                    _mode: PhantomData<MODE>,
                }


                impl Default for $Pin<Input<Floating>> {
                    fn default() -> Self {
                        Self {
                            state: State::Float,
                            _mode: PhantomData,
                        }
                    }
                }

                impl Default for $Pin<Input<PullUp>> {
                    fn default() -> Self {
                        Self {
                            state: State::High,
                            _mode: PhantomData,
                        }
                    }
                }

                impl Default for $Pin<Output<PushPull>> {
                    fn default() -> Self {
                        Self {
                            // TODO is default state actually low?
                            state: State::Low,
                            _mode: PhantomData,
                        }
                    }
                }

                impl Default for $Pin<Output<OpenDrain>> {
                    fn default() -> Self {
                        Self {
                            state: State::Float,
                            _mode: PhantomData,
                        }
                    }
                }

                impl<MODE> $Pin<MODE> {
                    /// Change the mode of this mock pin to an output with low and high states.
                    pub fn into_push_pull_output(self) -> $Pin<Output<PushPull>> {
                        $Pin::default()
                    }

                    /// Change the mode of this mock pin to an output with low and floating states.
                    pub fn into_open_drain_output(self) -> $Pin<Output<OpenDrain>> {
                        $Pin::default()
                    }

                    /// Change the mode of this mock pin to a floating input.
                    pub fn into_floating_input(self) -> $Pin<Input<Floating>> {
                        $Pin::default()
                    }

                    /// Change the mode of this mock pin to an input with a pullup resistor.
                    pub fn into_pull_up_input(self) -> $Pin<Input<PullUp>> {
                        $Pin::default()
                    }
                }

                impl OutputPin for $Pin<Output<PushPull>> {
                    type Error = core::convert::Infallible;
                    /// Drive the mock pin high.
                    fn set_high(&mut self) -> Result<(), Self::Error> {
                        Ok(self.state = State::High)
                    }
                    /// Drive the mock pin low.
                    fn set_low(&mut self) -> Result<(), Self::Error> {
                        Ok(self.state = State::Low)
                    }
                }

                impl OutputPin for $Pin<Output<OpenDrain>> {
                    type Error = core::convert::Infallible;
                    /// Leave the mock pin floating.
                    fn set_high(&mut self) -> Result<(), Self::Error> {
                        Ok(self.state = State::Float)
                    }

                    /// Drive the mock pin low.
                    fn set_low(&mut self) -> Result<(), Self::Error> {
                        Ok(self.state = State::Low)
                    }
                }

                impl<MODE> InputPin for $Pin<Input<MODE>> {
                    type Error = core::convert::Infallible;
                    /// Is the mock input pin high? Panic if it's floating.
                    fn is_high(&self) -> Result<bool,Self::Error> {
                        Ok(!self.is_low()?)
                    }
                    /// Is the mock input pin low? Panic if it's floating.
                    fn is_low(&self) -> Result<bool, Self::Error> {
                        match self.state {
                            State::Low => Ok(true),
                            State::High => Ok(false),
                            State::Float => {
                                panic!("Tried to read a floating input, value is non-deterministic!")
                            }
                        }
                    }
                }
            )+
        }
    };
}

/// A struct representing a mock port of GPIO pins.
#[derive(Debug)]
pub struct GPIOA;

gpio!( GPIOA, gpioa, [
    (PA0, pa0, Input<Floating>),
    (PA1, pa1, Input<Floating>),
    (PA2, pa2, Input<Floating>),
    (PA3, pa3, Input<Floating>),
    (PA4, pa4, Input<Floating>),
    (PA5, pa5, Input<Floating>),
    (PA6, pa6, Input<Floating>),
    (PA7, pa7, Input<Floating>),
    (PA8, pa8, Input<Floating>),
    (PA9, pa9, Input<Floating>),
    (PA10, pa10, Input<Floating>),
    (PA11, pa11, Input<Floating>),
    (PA12, pa12, Input<Floating>),
    (PA13, pa13, Input<Floating>),
    (PA14, pa14, Input<Floating>),
    (PA15, pa15, Input<Floating>),
]);
