import("pkg/shared_channel_demo_bg.wasm")
.then(wasm => import("./index.js"))
.catch(e => console.error("Error importing `index.js`:", e));
