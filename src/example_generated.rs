//! An example of a struct generated by the `keypad_struct!()` macro.

use crate::mock_hal::{self, Input, OpenDrain, Output, PullUp};

keypad_struct! {
    #[doc= "Example output of `keypad_struct!()`- for documentation purposes only! \n\nYou shouldn't try to use `ExampleKeypad` outside of this crate.\n\nThis struct is the result of this macro invocation:\n```\nuse mock_hal::{self, Input, OpenDrain, Output, PullUp};\n\nkeypad_struct!{\n    pub struct ExampleKeypad {\n        rows: (\n            mock_hal::gpioa::PA0<Input<PullUp>>,\n            mock_hal::gpioa::PA1<Input<PullUp>>,\n            mock_hal::gpioa::PA2<Input<PullUp>>,\n            mock_hal::gpioa::PA3<Input<PullUp>>,\n        ),\n        columns: (\n            mock_hal::gpioa::PA4<Output<OpenDrain>>,\n            mock_hal::gpioa::PA5<Output<OpenDrain>>,\n            mock_hal::gpioa::PA6<Output<OpenDrain>>,\n            mock_hal::gpioa::PA7<Output<OpenDrain>>,\n            mock_hal::gpioa::PA8<Output<OpenDrain>>,\n        ),\n    }\n}\n```"]
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
