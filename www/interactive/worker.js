
importScripts('../pkg/volvelle_wasm.js')
const {new_session, new_share} = wasm_bindgen;

// Response to a message from the main thread, which will contain
// at least two fields:
//     `method` tells us what to do
//      `nonce` is a unique ID we have to echo back so the main
//              thread can associate messages to their responses
onmessage = async function(e) {
    await wasm_bindgen('../pkg/volvelle_wasm_bg.wasm');

    if (e.data.method === "new_session") {
        let result = new_session(
            e.data.hrp,
            e.data.threshold,
            e.data.size,
            e.data.checksum
        );
        result.nonce = e.data.nonce;
        postMessage(result);
    }

    if (e.data.method === "new_share") {
        let result = new_share(e.data.session);
        result.nonce = e.data.nonce;
        postMessage(result);
        return;
    }
}
