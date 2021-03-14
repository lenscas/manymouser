use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    ffi::{CStr, OsStr},
    fs::File,
    io::Read,
    mem::{self, MaybeUninit},
    os::{
        raw::c_ushort,
        unix::prelude::{AsRawFd, FromRawFd, OsStrExt},
    },
};

use evdev::raw::{eviocgabs, eviocgbit, eviocgname, input_absinfo};
use input_linux_sys::{
    input_event, ABS_MAX, ABS_X, ABS_Y, BTN_BACK, BTN_LEFT, BTN_MISC, BTN_MOUSE, BTN_STYLUS,
    BTN_STYLUS2, BTN_TOUCH, EV_ABS, EV_KEY, EV_REL, KEY_MAX, REL_DIAL, REL_HWHEEL, REL_MAX,
    REL_WHEEL, REL_X, REL_Y,
};
use mem::size_of;
use nix::{
    libc::{c_int, closedir, open, DIR, O_NONBLOCK, O_RDONLY, S_IFCHR},
    sys::stat::stat,
};

use super::Driver;
const E_AGAIN: i32 = 11;

const MAX_MICE: usize = 32;
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ManyMouseEventType {
    Absmotion = 0,
    Relmotion = 1,
    Button = 2,
    Scroll = 3,
    _Disconnect = 4,
    _Max = 5,
}

#[derive(Debug)]
pub struct ManyMouseEvent {
    pub event_type: ManyMouseEventType,
    pub device: usize,
    pub item: u16,
    pub value: i32,
    pub minval: Option<i32>,
    pub maxval: Option<i32>,
}

pub struct MouseStruct {
    pub fd: File,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub name: [u8; 64],
}

pub struct DriverContainer {
    pub mice: [Option<MouseStruct>; MAX_MICE],
    pub available_mice: usize,
}

fn test_bit(array: &[u8], bit: c_int) -> u8 {
    array[bit as usize / 8] & (1 << (bit % 8))
}

impl MouseStruct {
    pub fn poll_mouse(&mut self, device: usize) -> std::io::Result<Option<ManyMouseEvent>> {
        loop {
            //struct input_event event;
            let mut buf: [u8; size_of::<input_event>()] = [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ];
            let res = self.fd.read_exact(&mut buf);
            if let Err(x) = res {
                if matches!(x.raw_os_error(), Some(E_AGAIN)) {
                    return Ok(None);
                }
                return Err(x);
            }
            let event: input_event = unsafe { std::mem::transmute(buf) };
            //let event = buf[0].try_into().unwrap();

            let mut event_type;
            let item;
            let mut min_val = None;
            let mut max_val = None;
            if event.type_ == EV_REL.try_into().unwrap() {
                event_type = ManyMouseEventType::Relmotion;
                if (event.code == REL_X.try_into().unwrap())
                    || (event.code == REL_DIAL.try_into().unwrap())
                {
                    item = 0;
                } else if event.code == REL_Y.try_into().unwrap() {
                    item = 1;
                } else if event.code == REL_WHEEL.try_into().unwrap() {
                    event_type = ManyMouseEventType::Scroll;
                    item = 0;
                } else if event.code == REL_HWHEEL.try_into().unwrap() {
                    event_type = ManyMouseEventType::Scroll;
                    item = 1;
                } else {
                    continue;
                } /* else */
            }
            /* if */
            else if event.type_ == EV_ABS.try_into().unwrap() {
                event_type = ManyMouseEventType::Absmotion;
                if event.code == ABS_X.try_into().unwrap() {
                    item = 0;
                    min_val = Some(self.min_x);
                    max_val = Some(self.max_x);
                }
                /* if */
                else if event.code == ABS_Y.try_into().unwrap() {
                    item = 1;
                    min_val = Some(self.min_y);
                    max_val = Some(self.max_y);
                } else {
                    continue;
                } /* else */
            } else if event.type_ == EV_KEY.try_into().unwrap() {
                event_type = ManyMouseEventType::Button;
                if (event.code >= BTN_LEFT.try_into().unwrap())
                    && (event.code <= BTN_BACK.try_into().unwrap())
                {
                    item = event.code - c_ushort::try_from(BTN_MOUSE).unwrap();
                }
                /* just in case some device uses this block of events instead... */
                else if (event.code >= BTN_MISC.try_into().unwrap())
                    && (event.code <= BTN_LEFT.try_into().unwrap())
                {
                    item = event.code - c_ushort::try_from(BTN_MISC).unwrap();
                } else if event.code == BTN_TOUCH.try_into().unwrap() {
                    /* tablet... */
                    item = 0;
                } else if event.code == BTN_STYLUS.try_into().unwrap() {
                    /* tablet... */
                    item = 1;
                } else if event.code == BTN_STYLUS2.try_into().unwrap() {
                    /* tablet... */
                    item = 2;
                } else {
                    /*printf("unhandled mouse button: 0x%X\n", event.code);*/
                    continue;
                } /* else */
            } else {
                continue;
            }
            return Ok(Some(ManyMouseEvent {
                device,
                event_type,
                item,
                value: event.value,
                minval: min_val,
                maxval: max_val,
            }));
        }
    }
}

impl DriverContainer {
    pub fn init_mouse(&mut self, _fname: &CStr, fd: File) -> bool {
        //let mouse = self.mice[self.available_mice].as_mut().unwrap();
        let mut has_absolutes = 0;
        let mut is_mouse = 0;
        let mut relcaps: [u8; (REL_MAX as usize / 8) + 1] = [0, 0];
        let mut abscaps: [u8; (ABS_MAX as usize / 8) + 1] = [0, 0, 0, 0, 0, 0, 0, 0];
        let mut keycaps: [u8; (KEY_MAX as usize / 8) + 1] = [0; 96];

        if unsafe {
            eviocgbit(
                fd.as_raw_fd(),
                EV_KEY.try_into().unwrap(),
                std::mem::size_of_val(&keycaps).try_into().unwrap(),
                (&mut keycaps).as_mut_ptr(),
            )
        }
        .is_err()
        {
            return false;
        };

        match unsafe {
            eviocgbit(
                fd.as_raw_fd(),
                EV_REL.try_into().unwrap(),
                std::mem::size_of_val(&relcaps).try_into().unwrap(),
                (&mut relcaps).as_mut_ptr(),
            )
        } {
            Ok(_) => {
                if (test_bit(&relcaps, REL_X) != 0)
                    && (test_bit(&relcaps, REL_Y) != 0)
                    && test_bit(&keycaps, BTN_MOUSE) != 0
                {
                    is_mouse = 1;
                }
            }
            Err(x) => {
                let _ = dbg!(x);
            }
        }
        match unsafe {
            eviocgbit(
                fd.as_raw_fd(),
                EV_ABS.try_into().unwrap(),
                std::mem::size_of_val(&abscaps).try_into().unwrap(),
                (&mut abscaps).as_mut_ptr(),
            )
        } {
            Ok(_) => {
                if (test_bit(&abscaps, ABS_X) != 0) && (test_bit(&abscaps, ABS_Y) != 0) {
                    /* might be a touch pad... */
                    if test_bit(&keycaps, BTN_TOUCH) != 0 {
                        is_mouse = 1; /* touch pad, touchscreen, or tablet. */
                        has_absolutes = 1;
                    }
                }
            }
            Err(x) => {
                let _ = dbg!(x);
            }
        }

        if is_mouse == 0 {
            return false;
        }

        let mut mouse_min_x = 0;
        let mut mouse_min_y = 0;
        let mut mouse_max_x = 0;
        let mut mouse_max_y = 0;
        if has_absolutes != 0 {
            let mut absinfo = MaybeUninit::<input_absinfo>::zeroed();
            if let Err(x) = unsafe {
                eviocgabs(
                    fd.as_raw_fd(),
                    ABS_X.try_into().unwrap(),
                    absinfo.as_mut_ptr(),
                )
            } {
                dbg!(x);
                return false;
            }
            let mut absinfo = unsafe { absinfo.assume_init() };
            mouse_min_x = absinfo.minimum;
            mouse_max_x = absinfo.maximum;

            if unsafe { eviocgabs(fd.as_raw_fd(), ABS_Y.try_into().unwrap(), &mut absinfo) }
                .is_err()
            {
                return false;
            }
            mouse_min_y = absinfo.minimum;
            mouse_max_y = absinfo.maximum;
        }
        let mut mouse_name: [u8; 64] = [0; 64];
        if let Err(x) = unsafe { eviocgname(fd.as_raw_fd(), &mut mouse_name) } {
            dbg!(x);
            mouse_name = [
                b'u', b'n', b'k', b'n', b'o', b'w', b'n', b'd', b'e', b'v', b'i', b'c', b'e', 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ];
        }

        let mouse = MouseStruct {
            fd,
            min_x: mouse_min_x,
            min_y: mouse_min_y,
            max_x: mouse_max_x,
            max_y: mouse_max_y,
            name: mouse_name,
        };
        self.mice[self.available_mice] = Some(mouse);

        true
    }
    /* Return a file descriptor if this is really a mouse, -1 otherwise. */
    pub fn open_if_mouse(&mut self, fname: &CStr) -> bool {
        let devmajor;
        let devminor;

        let statbuf = match stat(fname) {
            Err(x) => {
                dbg!(fname);
                dbg!(x);
                return false;
            }
            Ok(x) => {
                if S_IFCHR == x.st_mode {
                    return false;
                }
                x
            }
        };
        /* evdev node ids are major 13, minor 64-96. Is this safe to check? */
        devmajor = (statbuf.st_rdev & 0xFF00) >> 8;
        devminor = statbuf.st_rdev & 0x00FF;
        if (devmajor != 13) || (devminor < 64) || (devminor > 96) {
            return false; /* not an evdev. */
        }

        let fd = unsafe { open(fname.as_ptr() as *const i8, O_RDONLY | O_NONBLOCK) };
        if fd == -1 {
            return true;
        }
        let fd = unsafe { File::from_raw_fd(fd) };
        if self.init_mouse(fname, fd) {
            return true;
        }
        false
    }
    fn linux_evdev_init(&mut self) -> i32 {
        let dirp: *mut DIR;

        for i in 0..MAX_MICE {
            self.mice[i] = None;
        }

        dirp = unsafe {
            nix::libc::opendir(CStr::from_bytes_with_nul(b"/dev/input\0").unwrap().as_ptr())
        };
        if dirp.is_null() {
            return -1;
        }

        let files = std::fs::read_dir("/dev/input").unwrap();
        for file in files {
            let path = file.unwrap().path();
            let path: &OsStr = &path.as_os_str();
            let mut bytes = path.as_bytes().to_vec();
            if bytes.last().unwrap() != &0 {
                bytes.push(0)
            }
            let cstr = CStr::from_bytes_with_nul(&bytes).unwrap();
            if self.open_if_mouse(cstr) {
                self.available_mice += 1;
            }
        }

        unsafe { closedir(dirp) };

        self.available_mice as i32
    }

    pub(crate) fn linux_evdev_name(&self, index: usize) -> Option<&CStr> {
        self.mice
            .get(index)
            .and_then(|v| v.as_ref())
            .map(|v| unsafe { CStr::from_bytes_with_nul_unchecked(&v.name) })
    }
    pub(crate) fn linux_evdev_poll(&mut self) -> Option<ManyMouseEvent> {
        /*
         * (i) is static so we iterate through all mice round-robin. This
         *  prevents a chatty mouse from dominating the queue.
         */
        static mut I: usize = 2;

        if unsafe { I >= self.available_mice } {
            unsafe { I = 0 };
        }
        /* handle reset condition. */
        while unsafe { I < self.available_mice } {
            let mouse = unsafe { &mut self.mice[I] };
            if let Some(mouse) = mouse {
                if let Ok(res) = mouse.poll_mouse(unsafe { I }) {
                    if res.is_some() {
                        return res;
                    }
                }
            }
            unsafe { I += 1 };
        }

        None
    }
    pub(crate) fn get_all_ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.get_all_mice().enumerate().map(|(i, _)| i)
    }
    pub(crate) fn get_all_mice<'a>(&'a self) -> impl Iterator<Item = &MouseStruct> + 'a {
        self.mice[0..self.available_mice]
            .iter()
            .filter_map(|v| v.as_ref())
    }
    pub(crate) fn new() -> Self {
        let mut new_self = Self {
            mice: Default::default(), //[None; MAX_MICE],
            available_mice: 0,
        };
        new_self.linux_evdev_init();
        new_self
    }
}

impl Driver for DriverContainer {
    fn poll(&mut self) -> Option<super::Event> {
        self.linux_evdev_poll().map(|v| super::Event {
            device_id: v.device,
            event: match v.event_type {
                ManyMouseEventType::Absmotion => super::ManyMouseEvent::AbsoluteMotion {
                    moved: if v.item == 0 {
                        super::AbsoluteMotionMoved::X(v.value)
                    } else {
                        super::AbsoluteMotionMoved::Y(v.value)
                    },
                    max: v.maxval.unwrap(),
                    min: v.minval.unwrap(),
                },
                ManyMouseEventType::Relmotion => super::ManyMouseEvent::RelativeMotion {
                    x: if v.item == 0 { v.value } else { 0 },
                    y: if v.item == 0 { 0 } else { v.value },
                },
                ManyMouseEventType::Button => {
                    println!("{:?}", v);
                    super::ManyMouseEvent::Button {
                        side: if v.item == 1 {
                            super::Button::Left
                        } else {
                            super::Button::Right
                        },
                        is_pressed: v.value == 1,
                    }
                }
                ManyMouseEventType::Scroll => super::ManyMouseEvent::Scroll {
                    value: v.value,
                    min: v.minval.unwrap(),
                    max: v.maxval.unwrap(),
                },
                ManyMouseEventType::_Disconnect => super::ManyMouseEvent::Disconnect,
                ManyMouseEventType::_Max => super::ManyMouseEvent::Max {},
            },
        })
    }

    fn driver_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Linux evdev rust")
    }

    fn device_name(&self, id: usize) -> Option<&CStr> {
        self.linux_evdev_name(id)
    }

    fn get_all_mouse_names(&self) -> Box<dyn Iterator<Item = &CStr> + '_> {
        Box::new(
            self.get_all_mice()
                .map(|v| unsafe { CStr::from_bytes_with_nul_unchecked(&v.name) }),
        )
    }

    fn get_all_ids(&self) -> Box<dyn Iterator<Item = usize> + '_> {
        Box::new(self.get_all_ids())
    }
}
