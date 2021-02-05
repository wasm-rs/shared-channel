import * as pkg from "pkg";

let worker = new Worker("/worker.js");

worker.onmessage = () => {
	let channel = new pkg.Channel();
	worker.postMessage(channel.replica());
	window.sender = channel.sender();
	console.log("Now, you can use methods on `sender`. Try `sender.init()");
}
