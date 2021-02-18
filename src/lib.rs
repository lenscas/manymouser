pub mod linux_evdev;

use std::marker::PhantomData;

use linux_evdev::{DriverContainer, ManyMouseEvent};

pub struct EventContext {
    driver: DriverContainer,
    _not_send_or_sync: PhantomData<*mut ()>,
}
impl Default for EventContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EventContext {
    pub fn new() -> Self {
        Self {
            driver: DriverContainer::new(),
            _not_send_or_sync: PhantomData,
        }
    }
    pub fn poll(&mut self) -> Option<ManyMouseEvent> {
        self.driver.linux_evdev_poll()
    }
    pub fn driver_name(&self) -> &'static str {
        "Linux evdev"
    }
    pub fn device_name(&self, id: usize) -> Option<&[u8; 64]> {
        self.driver.linux_evdev_name(id)
    }
    pub fn get_all_mouse_ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.driver.get_all_ids()
    }
    pub fn get_all_mouse_names(&self) -> impl Iterator<Item = &[u8; 64]> + '_ {
        self.driver.get_all_mice().map(|v| &v.name)
    }
}
