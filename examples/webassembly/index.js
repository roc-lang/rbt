import fs from "fs";
import addWasm from "./add.wasm";

WebAssembly.instantiate(addWasm).then(addModule => {
  console.log("adding 1 and 2 in WebAssembly:")
  console.log(addModule.instance.exports.add(1, 2));
})
