worker = self;
import("./pkg/shared_channel_demo_bg.wasm")
.then(wasm => {
	import("./pkg/shared_channel_demo.js").then(module => {
		postMessage("started");
		worker.onmessage = (msg) => {
			let channel = module.Channel.from(msg.data);
			while (true) {
				channel.run();
				console.debug("worker: runner terminated, restarting");
			}
		}
	})
});
