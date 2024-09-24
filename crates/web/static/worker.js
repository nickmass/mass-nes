import init, { worker_gfx, worker_machine } from "./pkg/web.js";

let finished = false;
onmessage = async function (event) {
  onmessage = null;
  if (finished) {
    return;
  }
  finished = true;

  await init({ module: event.data.module, memory: event.data.memory });
  switch (event.data.worker_type) {
    case "gfx":
      await worker_gfx(event.data.offscreen_canvas, event.data.channel);
      break;
    case "machine":
      await worker_machine(event.data.channel);
      break;
    default:
      console.log("invalid worker type");
      break;
  }
};
