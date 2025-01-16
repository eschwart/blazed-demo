use std::sync::atomic::{
    AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicU16, AtomicU32, AtomicU64, AtomicU8,
};

pub trait AtomicInt {
    type Atomic;
}

impl AtomicInt for i8 {
    type Atomic = AtomicI8;
}
impl AtomicInt for i16 {
    type Atomic = AtomicI16;
}
impl AtomicInt for i32 {
    type Atomic = AtomicI32;
}
impl AtomicInt for i64 {
    type Atomic = AtomicI64;
}
impl AtomicInt for u8 {
    type Atomic = AtomicU8;
}
impl AtomicInt for u16 {
    type Atomic = AtomicU16;
}
impl AtomicInt for u32 {
    type Atomic = AtomicU32;
}
impl AtomicInt for u64 {
    type Atomic = AtomicU64;
}
