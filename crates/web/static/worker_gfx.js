import init, { worker_gfx } from "./pkg/web.js";

let finished = false;
onmessage = async function (event) {
  onmessage = null;
  if (finished) {
    return;
  }
  finished = true;

  let { module, memory, offscreen_canvas, channel } = event.data;
  await init({ module, memory });
  await worker_gfx(offscreen_canvas, channel);
};
