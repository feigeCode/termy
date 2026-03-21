#![cfg_attr(not(unix), allow(unused_imports))]

pub(crate) mod channel;
pub(crate) mod coalescer;
pub(crate) mod parser;
pub(crate) mod worker;

pub(crate) use channel::{ControlCommandResult, ControlRequest, try_enqueue_control_request};
#[cfg(unix)]
pub(crate) use channel::{
    FATAL_EXIT_QUEUE_BOUND, NOTIFICATION_QUEUE_BOUND, PENDING_QUEUE_BOUND, REQUEST_QUEUE_BOUND,
};
pub(crate) use coalescer::NotificationCoalescer;
#[cfg(unix)]
pub(crate) use worker::spawn_control_threads;
