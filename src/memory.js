/// Javascript library for managing memory in WebAssembly for the dropbear engine Gleam interface. 

class WasmMemoryManager {
    constructor(memory) {
        this.memory = memory;
        this.nextFreeOffset = 1024;
        this.allocations = new Map();
    }

    /// Fetches the new view of the memory after the memory is refreshed
    getView() {
        return new DataView(this.memory.buffer);
    }
}