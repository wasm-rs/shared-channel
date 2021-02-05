//! # Single Publisher Single Consumer Channel
//!
//! A simple single publisher, single consumer channel can be used to communicate from main thread
//! to a worker thread or between worker threads.
//!
// NOTE: Current algorithm used to send and receive messages
// has been modeled after https://github.com/willemt/bipbuffer and has not been extensively
// tested for suitability and/or correctness.
//
// This is an ongoing area of development and the algorithm might change at any moment, so
// one should not base their expectations based on the particularities of the algorithm.
use super::*;
use js_sys::{Array, Atomics, Int32Array, SharedArrayBuffer, Uint8Array};
use std::marker::PhantomData;
use std::time::Duration;
#[cfg(test)]
#[allow(unused_imports)]
use wasm_rs_dbg::dbg;

/// Shared single publisher, single-consumer channel
///
/// A channel can be passed between different threads with their own instances of a WebAssembly
/// module by caling [`wasm_bindgen::JsValue::from`] on this channel and subsequently calling
/// [`SharedChannel::from`] on the value in a different thread.
pub struct SharedChannel<T>
where
    T: Shareable,
{
    _header: SharedArrayBuffer,
    _data: SharedArrayBuffer,
    header: Int32Array,
    data: Uint8Array,
    len: u32,
    phantom_data: PhantomData<T>,
}

impl<T> From<SharedChannel<T>> for JsValue
where
    T: Shareable,
{
    fn from(channel: SharedChannel<T>) -> JsValue {
        let array = Array::new();
        array.push(&channel._header);
        array.push(&channel._data);
        array.into()
    }
}

impl<T> From<JsValue> for SharedChannel<T>
where
    T: Shareable,
{
    fn from(array: JsValue) -> SharedChannel<T> {
        let array: Array = array.into();
        let header = array.shift();
        let data = array.shift();
        channel_(header.into(), data.into())
    }
}

const A_START: u32 = 0;
const A_END: u32 = 1;
const B_END: u32 = 2;
const B_USE: u32 = 3;

impl<T> SharedChannel<T>
where
    T: Shareable,
{
    fn unused(&self) -> Result<u32, JsValue> {
        let b_use = (Atomics::load(&self.header, B_USE)? as u32) == 1;
        if b_use {
            let a_start = Atomics::load(&self.header, A_START)? as u32;
            let b_end = Atomics::load(&self.header, B_END)? as u32;
            Ok(a_start - b_end)
        } else {
            let a_end = Atomics::load(&self.header, A_END)? as u32;
            Ok(self.len - a_end)
        }
    }

    fn maybe_switch(&self) -> Result<(), JsValue> {
        let a_start = Atomics::load(&self.header, A_START)? as u32;
        let a_end = Atomics::load(&self.header, A_END)? as u32;
        let b_end = Atomics::load(&self.header, B_END)? as u32;
        if self.len - a_end < a_start - b_end {
            Atomics::store(&self.header, B_USE, 1i32)?;
        }
        Ok(())
    }

    /// Consumes and splits channel into a [`Sender`] and a [`Receiver`]
    ///
    /// Splitting it into allows us to ensure roles aren't mixed up.
    pub fn split(self) -> (Sender<T>, Receiver<T>) {
        (Sender(self.clone()), Receiver(self))
    }
}

impl<T> Clone for SharedChannel<T>
where
    T: Shareable,
{
    fn clone(&self) -> Self {
        Self {
            _header: self._header.clone(),
            _data: self._data.clone(),
            header: self.header.clone(),
            data: self.data.clone(),
            len: self.len,
            phantom_data: PhantomData,
        }
    }
}

/// Sender part of the channel
#[derive(Clone)]
pub struct Sender<T>(pub SharedChannel<T>)
where
    T: Shareable;

/// Receiver part of the channel
pub struct Receiver<T>(pub SharedChannel<T>)
where
    T: Shareable;

/// Creates a channel of `len` bytes
pub fn channel<T>(len: u32) -> SharedChannel<T>
where
    T: Shareable,
{
    let header = SharedArrayBuffer::new(4 * (std::mem::size_of::<u32>() as u32));
    let data = SharedArrayBuffer::new(len);
    channel_(header, data)
}

fn channel_<T>(header: SharedArrayBuffer, data: SharedArrayBuffer) -> SharedChannel<T>
where
    T: Shareable,
{
    let header_ = Int32Array::new(&header);
    let data_ = Uint8Array::new(&data);
    let len = data_.byte_length();
    SharedChannel {
        _header: header,
        _data: data,
        header: header_,
        data: data_,
        len,
        phantom_data: PhantomData,
    }
}

impl<T> Sender<T>
where
    T: Shareable,
{
    /// Sends a value into the channel
    ///
    /// If there isn't enough space currently in the channel to accommodate
    /// the value, it'll throw a JavaScript exception (`"not enough space"`)
    pub fn send(&self, value: &T) -> Result<(), JsValue> {
        let bytes = value
            .to_bytes()
            .map_err(|e| JsValue::from(format!("serialization error: {}", e)))?;
        let len = bytes.byte_length();
        if self.0.unused()? < len {
            return Err("not enough space".to_string().into());
        }
        let b_use = (Atomics::load(&self.0.header, B_USE)? as u32) == 1;
        let end_header = if b_use { B_END } else { A_END };

        let end = Atomics::load(&self.0.header, end_header)? as u32;
        for i in 0..len {
            self.0.data.set_index(end + i, bytes.get_index(i));
        }
        Atomics::store(&self.0.header, end_header, (end + len) as i32)?;
        Atomics::notify(&self.0.header, end_header)?;
        Atomics::notify(&self.0.header, A_START)?;

        self.0.maybe_switch()?;

        Ok(())
    }
}

impl<T> Receiver<T>
where
    T: Shareable,
{
    /// Receives a value from the channel
    ///
    /// If `timeout` is `None`, if there is no message, it'll immediately return
    /// `Ok(None)`.
    ///
    /// If `timeout` is `Some(duration)` it will return `Ok(Some(value))` if there was a value,
    /// otherwise, it'll return `Ok(None)` when timed out.
    ///
    /// There's no way to specify an infinite timeout. Instead, a sufficiently large
    /// [`std::time::Duration`] should be used.
    pub fn recv(&self, timeout: Option<Duration>) -> Result<Option<T>, JsValue> {
        let mut array = Uint8Array::new_with_length(0);
        loop {
            match T::from(&array)
                .map_err(|e| JsValue::from(format!("deserialization error: {}", e)))?
            {
                Ok(value) => {
                    return Ok(Some(value));
                }
                Err(Expects(sz)) => {
                    array = Uint8Array::new_with_length(sz);
                    let mut a_start = Atomics::load(&self.0.header, A_START)? as u32;
                    let mut a_end = Atomics::load(&self.0.header, A_END)? as u32;
                    if a_start == a_end || self.0.len < a_start + sz {
                        match timeout {
                            None => return Ok(None),
                            Some(duration) => {
                                let result = Atomics::wait_with_timeout(
                                    &self.0.header,
                                    A_START,
                                    a_start as i32,
                                    duration.as_millis() as f64,
                                )?;
                                if result == "timed-out" {
                                    return Ok(None);
                                }
                                continue;
                            }
                        }
                    }
                    for i in 0..sz {
                        array.set_index(i, self.0.data.get_index(a_start + i));
                    }
                    a_start += sz;
                    let mut b_end = Atomics::load(&self.0.header, B_END)? as u32;
                    let mut b_use = (Atomics::load(&self.0.header, B_USE)? as u32) == 1;
                    if a_start == a_end {
                        if b_use {
                            a_start = 0;
                            a_end = b_end;
                            b_end = 0;
                            b_use = false;
                        } else {
                            a_start = 0;
                            a_end = 0;
                        }
                    }
                    if T::from(&array)
                        .map_err(|e| JsValue::from(format!("deserialization error: {}", e)))?
                        .is_ok()
                    {
                        Atomics::store(&self.0.header, B_USE, if b_use { 1i32 } else { 0i32 })?;
                        Atomics::store(&self.0.header, A_START, a_start as i32)?;
                        Atomics::store(&self.0.header, A_END, a_end as i32)?;
                        Atomics::store(&self.0.header, B_END, b_end as i32)?;
                        self.0.maybe_switch()?;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test() {
        let sz = 0u8.to_bytes().unwrap().byte_length();
        let (sender, receiver) = channel::<u8>(2 * sz).split();
        sender.send(&1).unwrap();
        sender.send(&2).unwrap();
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 1);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 2);
    }

    #[wasm_bindgen_test]
    fn not_enough_space() {
        let sz = 0u8.to_bytes().unwrap().byte_length();
        let (sender, _receiver) = channel::<u8>(1 * sz).split();
        sender.send(&1).unwrap();
        assert!(sender.send(&2).is_err());
    }

    #[wasm_bindgen_test]
    fn circular() {
        let sz = 0u8.to_bytes().unwrap().byte_length();
        let (sender, receiver) = channel::<u8>(8 * sz).split();
        sender.send(&1).unwrap();
        sender.send(&2).unwrap();
        sender.send(&3).unwrap();
        sender.send(&4).unwrap();
        sender.send(&5).unwrap();
        sender.send(&6).unwrap();
        sender.send(&7).unwrap();
        sender.send(&8).unwrap();
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 1);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 2);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 3);
        sender.send(&9).unwrap();
        sender.send(&10).unwrap();
        sender.send(&11).unwrap();
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 4);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 5);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 6);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 7);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 8);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 9);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 10);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 11);
    }

    #[wasm_bindgen_test]
    fn jsvalue() {
        let sz = 0u8.to_bytes().unwrap().byte_length();
        let ch = channel::<u8>(2 * sz);
        let js_value: JsValue = ch.into();
        let ch: SharedChannel<u8> = js_value.into();
        let (sender, receiver) = ch.split();
        sender.send(&1).unwrap();
        sender.send(&2).unwrap();
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 1);
        assert_eq!(receiver.recv(None).unwrap().unwrap(), 2);
    }
}
