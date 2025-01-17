import init, * as wasm from './pkg/debugger.js';

registerProcessor("WorkletProcessor", class WasmProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        let {module, memory, ptr} = options.processorOptions;
        wasm.initSync({ module, memory });
        this.processor = wasm.WorkletProcessor.unpack(ptr);
    }
    process(inputs, outputs) {
        return this.processor.process(outputs[0][0]);
    }
});
