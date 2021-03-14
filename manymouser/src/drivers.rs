use std::{borrow::Cow, ffi::CStr};

mod bindgen;
pub(crate) mod linux_evdev;
pub(crate) mod manymouse_driver;

#[derive(Clone, Copy, Debug)]
pub enum Button {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Debug, Copy)]
pub enum AbsoluteMotionMoved {
    X(i32),
    Y(i32),
}

#[derive(Clone, Debug)]
pub enum ManyMouseEvent {
    Button {
        side: Button,
        is_pressed: bool,
    },
    RelativeMotion {
        x: i32,
        y: i32,
    },
    AbsoluteMotion {
        moved: AbsoluteMotionMoved,
        min: i32,
        max: i32,
    },
    Scroll {
        value: i32,
        min: i32,
        max: i32,
    },
    Disconnect,
    //TODO: Can't find what this does/what it needs :(
    Max {},
}

#[derive(Clone, Debug)]
pub struct Event {
    pub device_id: usize,
    pub event: ManyMouseEvent,
}

pub(crate) trait Driver {
    fn poll(&mut self) -> Option<Event>;
    fn driver_name(&self) -> Cow<'static, str>;
    fn device_name(&self, id: usize) -> Option<&CStr>;
    fn get_all_mouse_names(&self) -> Box<dyn Iterator<Item = &CStr> + '_>;
    fn get_all_ids(&self) -> Box<dyn Iterator<Item = usize> + '_>;
}
