importScripts("web/shared_channel_demo.js");
wasm_bindgen("web/shared_channel_demo_bg.wasm").then(() => {
	self.module = wasm_bindgen;
	postMessage("started")
});

onmessage = (msg) => {
	let channel = self.module.Channel.from(msg.data);
	while (true) {
	  channel.run();
	  console.debug("worker: runner terminated, restarting");
	}
}
