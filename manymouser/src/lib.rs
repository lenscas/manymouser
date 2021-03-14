mod drivers;

use std::{borrow::Cow, ffi::CStr, marker::PhantomData};

use drivers::{Driver, Event};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DriverOptions {
    LinuxEvDev,
    ManyMouse,
}

fn get_driver(options: &[DriverOptions]) -> Option<Box<dyn Driver>> {
    if options.is_empty() {
        println!("No preferences given, manymouser will NEVER load a driver in that case.");
        return None;
    }
    options.iter().find_map(|v| match v {
        DriverOptions::LinuxEvDev if cfg!(target_os = "linux") => {
            Some(Box::new(drivers::linux_evdev::DriverContainer::new()) as Box<dyn Driver>)
        }
        DriverOptions::ManyMouse => drivers::manymouse_driver::ManyMouseDriver::new()
            .map(Box::new)
            .map(|v| v as Box<dyn Driver>),
        _ => None,
    })
}

pub struct Context {
    driver: Box<dyn Driver>,
    _not_send_or_sync: PhantomData<*mut ()>,
}
impl Context {
    pub fn new(preference: &[DriverOptions]) -> Option<Self> {
        Some(Self {
            driver: get_driver(preference)?,
            _not_send_or_sync: PhantomData,
        })
    }
    pub fn poll(&mut self) -> Option<Event> {
        self.driver.poll()
    }
    pub fn driver_name(&self) -> Cow<'static, str> {
        self.driver.driver_name()
    }
    pub fn device_name(&self, id: usize) -> Option<&CStr> {
        self.driver.device_name(id)
    }
    pub fn get_all_mouse_ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.driver.get_all_ids()
    }
    pub fn get_all_mouse_names(&self) -> impl Iterator<Item = &CStr> + '_ {
        self.driver.get_all_mouse_names()
    }
}
