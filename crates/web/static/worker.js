import init, { gfx_worker, machine_worker, sync_worker } from "./pkg/web.js";

let finished = false;
onmessage = async function (event) {
  onmessage = null;
  if (finished) {
    return;
  }
  finished = true;

  let { module, memory, ptr, transferables, worker_type } = event.data;

  await init({ module, memory });
  try {
    switch (worker_type) {
      case "gfx":
        await gfx_worker(ptr, transferables);
        break;
      case "machine":
        await machine_worker(ptr, transferables);
        break;
      case "sync":
        await sync_worker(ptr, transferables);
        break;
      default:
        throw new Error("invalid worker type");
        break;
    }
  } catch (err) {
    console.error(err);
    postMessage(false);
    throw err;
  }
};
