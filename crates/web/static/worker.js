import init, { worker } from "./pkg/web.js";

await init({});
await worker();
