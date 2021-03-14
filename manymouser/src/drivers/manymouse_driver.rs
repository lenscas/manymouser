use std::{borrow::Cow, convert::TryInto, ffi::CStr, mem::MaybeUninit};

use bindgen::{
    ManyMouseEventType_MANYMOUSE_EVENT_ABSMOTION, ManyMouseEventType_MANYMOUSE_EVENT_BUTTON,
    ManyMouseEventType_MANYMOUSE_EVENT_DISCONNECT, ManyMouseEventType_MANYMOUSE_EVENT_MAX,
    ManyMouseEventType_MANYMOUSE_EVENT_RELMOTION, ManyMouseEventType_MANYMOUSE_EVENT_SCROLL,
    ManyMouse_Quit,
};

use super::{
    bindgen::{self, ManyMouse_Init},
    AbsoluteMotionMoved, Button, Driver, ManyMouseEvent,
};
pub(crate) struct ManyMouseDriver {
    max_mice: i32,
}

impl ManyMouseDriver {
    pub(crate) fn new() -> Option<Self> {
        let max_mice = unsafe { ManyMouse_Init() };
        println!("{}", max_mice);
        (max_mice > 0).then(|| Self { max_mice })
    }
}

impl Drop for ManyMouseDriver {
    fn drop(&mut self) {
        unsafe { ManyMouse_Quit() }
    }
}
impl Driver for ManyMouseDriver {
    fn poll(&mut self) -> Option<super::Event> {
        unsafe {
            let mut event = MaybeUninit::zeroed();
            let success = bindgen::ManyMouse_PollEvent(event.as_mut_ptr());
            if success == 1 {
                Some(event.assume_init())
            } else {
                None
            }
        }
        .map(|v| super::Event {
            device_id: v.device as usize,
            event: match v.type_ {
                ManyMouseEventType_MANYMOUSE_EVENT_ABSMOTION => ManyMouseEvent::AbsoluteMotion {
                    max: v.maxval,
                    min: v.minval,
                    moved: if v.item == 0 {
                        AbsoluteMotionMoved::X(v.value)
                    } else {
                        AbsoluteMotionMoved::Y(v.value)
                    },
                },
                ManyMouseEventType_MANYMOUSE_EVENT_RELMOTION => ManyMouseEvent::RelativeMotion {
                    x: if v.item == 0 { v.value } else { 0 },
                    y: if v.item == 0 { 0 } else { v.value },
                },
                ManyMouseEventType_MANYMOUSE_EVENT_BUTTON => ManyMouseEvent::Button {
                    side: if v.item == 1 {
                        Button::Left
                    } else if v.item == 2 {
                        Button::Right
                    } else {
                        Button::Middle
                    },
                    is_pressed: v.value == 1,
                },
                ManyMouseEventType_MANYMOUSE_EVENT_DISCONNECT => ManyMouseEvent::Disconnect,
                ManyMouseEventType_MANYMOUSE_EVENT_MAX => {
                    println!("{:?}", v);
                    ManyMouseEvent::Max {}
                }
                ManyMouseEventType_MANYMOUSE_EVENT_SCROLL => ManyMouseEvent::Scroll {
                    value: v.value,
                    min: v.minval,
                    max: v.maxval,
                },
                x => panic!("Got invalid event type {}", x),
            },
        })
    }

    fn driver_name(&self) -> Cow<'static, str> {
        let name_ptr = unsafe { bindgen::ManyMouse_DriverName() };
        if name_ptr.is_null() {
            Cow::Borrowed("Manymouse")
        } else {
            let name = unsafe { CStr::from_ptr(name_ptr) };
            let name = name.to_string_lossy();
            Cow::Owned(format!("{} Manymouse", name))
        }
    }

    fn device_name(&self, id: usize) -> Option<&CStr> {
        let name_ptr = unsafe { bindgen::ManyMouse_DeviceName(id.try_into().unwrap()) };
        if name_ptr.is_null() {
            None
        } else {
            //TODO: Are these names actually guaranteed to be null terminated?
            //If not, oh well. UB FOR EVERYONE!
            Some(unsafe { CStr::from_ptr(name_ptr) })
        }
    }

    fn get_all_mouse_names(&self) -> Box<dyn Iterator<Item = &CStr> + '_> {
        Box::new(
            self.get_all_ids()
                .map(move |v| self.device_name(v))
                .filter_map(|v| v),
        )
    }

    fn get_all_ids(&self) -> Box<dyn Iterator<Item = usize> + '_> {
        let mut id = 0;
        Box::new(std::iter::from_fn(move || loop {
            let return_id = self.device_name(id).map(|_| id);
            id += 1;
            if return_id.is_some() {
                return return_id;
            } else if (id as i32) > self.max_mice {
                return None;
            }
        }))
    }
}
