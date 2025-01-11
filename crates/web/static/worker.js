import init, * as wasm from "./pkg/web.js";

let finished = false;
onmessage = async function (event) {
  onmessage = null;
  if (finished) {
    return;
  }
  finished = true;

  let { module, memory, ptr, transferables, entry_point } = event.data;

  await init({ module, memory });
  try {
    let entry = wasm[entry_point];
    if (typeof entry === "function") {
      entry(ptr, transferables);
    } else {
      throw new Error("invalid worker entry point");
    }
  } catch (err) {
    console.error(err);
    postMessage(false);
    throw err;
  }
};
