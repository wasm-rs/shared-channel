import * as pkg from "pkg";
import Worker from "./worker.js";
let channel = new pkg.Channel();
let worker = new Worker();
worker.onmessage = () => {
	worker.postMessage(channel.replica());
	window.sender = channel.sender();
	console.log("Now, you can use methods on `sender`. Try `sender.init()");
}
