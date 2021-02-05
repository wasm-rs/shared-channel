//! # Shared Channel for WebAssembly
//!
//! This crate provides a way for WebAssembly threads to receive messages from other threads using
//! a JavaScript primitive called `SharedArrayBuffer` which allows to share memory and use atomics
//! between different threads.
//!
//! This allows us to deploy Rust code as a worker process communicating with the main thread.
use js_sys::Uint8Array;
use thiserror::Error;
use wasm_bindgen::prelude::*;

pub mod spsc;

/// [`Shareable::from`] indicates that it needs at least `n` bytes to proceed
#[derive(Debug, Clone, Error)]
#[error("expects {0} bytes more")]
pub struct Expects(pub u32);

/// Any type that can be sent through a shared channel must implement this
pub trait Shareable: Sized {
    /// A generic error
    type Error: std::error::Error;
    /// Converts value into a [`js_sys::Uint8Array`]
    fn to_bytes(&self) -> Result<Uint8Array, Self::Error>;
    /// Converts [`js_sys::Uint8Array`] into a value
    fn from(bytes: &Uint8Array) -> Result<Result<Self, Expects>, Self::Error>;
}

#[cfg(feature = "serde")]
impl<T> Shareable for T
where
    for<'a> T: serde::Serialize + serde::Deserialize<'a>,
{
    #[cfg(not(any(feature = "serde-bincode")))]
    std::compile_error!("one of these features has to be enabled: serde-bincode");

    #[cfg(feature = "serde-bincode")]
    type Error = bincode::Error;

    fn to_bytes(&self) -> Result<Uint8Array, Self::Error> {
        #[cfg(feature = "serde-bincode")]
        let mut result: Vec<u8> = (bincode::serialized_size(self)? as u32)
            .to_ne_bytes()
            .into();
        #[cfg(feature = "serde-bincode")]
        let mut encoded: Vec<u8> = bincode::serialize(self)?;
        result.append(&mut encoded);
        Ok(Uint8Array::from(&result[..]))
    }
    fn from(bytes: &Uint8Array) -> Result<Result<Self, Expects>, Self::Error> {
        if bytes.byte_length() == 0 {
            return Ok(Err(Expects(4))); // need length
        }
        if bytes.byte_length() >= 4 {
            let mut data = vec![0; bytes.byte_length() as usize];
            bytes.copy_to(&mut data);
            let size = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
            if bytes.byte_length() == 4 {
                return Ok(Err(Expects(4 + size))); // now we know the full length
            }
            #[cfg(feature = "serde-bincode")]
            return Ok(Ok(bincode::deserialize::<Self>(&data[4..])?));
        }

        #[cfg(feature = "serde-bincode")]
        Err(Box::new(bincode::ErrorKind::Custom(
            "unexpected data".to_string(),
        )))
    }
}
