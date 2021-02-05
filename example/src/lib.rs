use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_rs_dbg::dbg;
use wasm_rs_shared_channel::spsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Init,
    Done { count: u32 },
}

#[wasm_bindgen]
pub struct Channel {
    sender: Option<spsc::Sender<Request>>,
    receiver: spsc::Receiver<Request>,
}

#[wasm_bindgen]
impl Channel {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Channel {
        let (sender, receiver) = spsc::channel::<Request>(1024).split();
        Channel {
            sender: Some(sender),
            receiver,
        }
    }
    pub fn from(val: JsValue) -> Self {
        let (sender, receiver) = spsc::SharedChannel::from(val).split();
        Channel {
            sender: Some(sender),
            receiver,
        }
    }

    pub fn replica(&self) -> JsValue {
        self.receiver.0.clone().into()
    }

    pub fn run(&mut self) -> Result<(), JsValue> {
        console_error_panic_hook::set_once();
        loop {
            dbg!("waiting for messages for 10 seconds");
            match self
                .receiver
                .recv(Some(std::time::Duration::from_secs(10)))?
            {
                None => {}
                Some(request) => {
                    dbg!(&request);
                    if let Request::Done { .. } = request {
                        dbg!("received `Done`, terminating the runner");
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn sender(&mut self) -> Result<Sender, JsValue> {
        match self.sender.take() {
            Some(sender) => Ok(Sender(sender)),
            None => Err("sender is already taken".to_string().into()),
        }
    }
}

#[wasm_bindgen]
pub struct Sender(spsc::Sender<Request>);

#[wasm_bindgen]
impl Sender {
    pub fn init(&self) -> Result<(), JsValue> {
        self.0.send(&Request::Init)
    }

    pub fn done(&self, count: u32) -> Result<(), JsValue> {
        self.0.send(&Request::Done { count })
    }
}
