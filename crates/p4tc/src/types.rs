use bitflags::bitflags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Transport {
    Unspec = 0,
    Netlink = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ObjType {
    Unspec = 0,
    Table = 1,
    Extern = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Entity {
    Unspec = 0,
    Kernel = 1,
    Tc = 2,
    Timer = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Phase {
    Unspec = 0,
    Sot = 1,
    Mot = 2,
    Eot = 3,
    Abt = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Policy {
    Unspec = 0,
    Basic = 1,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MsgFlags: u32 {
        const ROOT = 1;
        const ACK  = 2;
        const ECHO = 4;
    }
}
