//
// *** The following is basically boilerplate to set up a webworker ***
//
const worker = new Worker('worker.js');

// When sending messages, assign unique nonces to each one and maintain
// a map of promises which will associate the response to the sent message
let nonce = 0;
let activeQueries = {};
async function postWorkerMessage(msg) {
    // Obtain a unique nonce
    const msgNonce = nonce;
    nonce += 1;

    // Tag the message and send it.
    msg.nonce = msgNonce;
    worker.postMessage(msg);

    // Return a promise so the caller can attach callbacks
    return await new Promise((resolve) => {activeQueries[msgNonce] = resolve;});
}

// Respond to messages by looking at the nonce->promise map
function handleWorkerMessage(ev) {
    const msgNonce = ev.data.nonce;
    activeQueries[msgNonce](ev.data);
    delete activeQueries[msgNonce];
}
worker.onmessage = handleWorkerMessage;
//
// *** End boilerplate ***
//

// Global "session" state which is basically passed into Rust and back
// every time something changes.
let g_session;

// Construct a new session from the user's current choice of global parameters
async function new_session() {
    g_session = await postWorkerMessage({
        "method": "new_session",
        "hrp": document.getElementById("i_hrp").value,
        "threshold": document.getElementById("i_k").value,
        "size": document.getElementById("i_size").value,
        "checksum": document.getElementById("i_checksum").value
    });
}

// Determine whether the user's choice of parameters in the web UI differs from
// the active parameters
function globalParamsChanged() {
     return (g_session.hrp != document.getElementById("i_hrp").value)
         || (g_session.checksum != document.getElementById("i_checksum").value)
         || (g_session.size != document.getElementById("i_size").value)
         || (g_session.threshold != document.getElementById("i_k").value);
}

// If the user has changed something, activate/deactivate the Update/Cancel buttons
async function selectGlobalParams() {
     console.assert(g_session !== undefined);
     const changed = globalParamsChanged();

     document.getElementById("bt_update").disabled = !changed;
     document.getElementById("bt_cancel").disabled = !changed;
}

// The user clicked "Update"; clear all shares and set the global parameters to new values
async function updateGlobalParams() {
     console.assert(g_session !== undefined);
     console.assert(globalParamsChanged()); // button should've been disabled otherwise

     if (g_shares.length == 0 || confirm("Updating global parameters will erase all shares! Are you sure?")) {
         document.getElementById("bt_update").disabled = true;
         document.getElementById("bt_cancel").disabled = true;
         return new_session();
     }
}

// The user clicked "Cancel"; copy the current values into the web UI
async function cancelGlobalParams() {
     console.assert(g_session !== undefined);
     console.assert(globalParamsChanged()); // button should've been disabled otherwise
     document.getElementById("i_hrp").value = g_session.hrp;
     document.getElementById("i_k").value = g_session.threshold;
     document.getElementById("i_size").value = g_session.size;
     document.getElementById("i_checksum").value = g_session.checksum;
     document.getElementById("bt_update").disabled = true;
     document.getElementById("bt_cancel").disabled = true;
}

// Create a new, empty share, and switch to its checksum worksheet
async function newShare() {
    console.assert(g_session !== undefined);
    g_session = await postWorkerMessage({
        "method": "new_share",
        "session": g_session,
    });
    createChecksumWorksheet(g_session.shares[g_session.shares.length - 1]);
    return;
}

// Construct a new checksum worksheet
function createChecksumWorksheet(worksheet) {
     // FIXME we aren't supposed to actually manipulate the DOM here
     document.getElementById("div_home").style.display = "none";
     let tab = document.getElementById('div_worksheet');
     tab.textContent = "";

     console.log(worksheet.rows);
     let y = 0;
     for (row of worksheet.rows) {
         let x = 0;
         for (cell of row.cells) {
             let domInp = document.createElement("input");
             domInp.id = cell.dom_id;
             console.log("Adding cell <input> " + cell.dom_id);
             switch (cell.ty) {
             case "fixed_hrp":
                 console.assert(cell.val !== undefined);
                 domInp.className = "cell cell_hrp";
                 domInp.disabled = true;
                 domInp.value = cell.val;
                 break;
             case "fixed_residue":
                 console.assert(cell.val !== undefined);
                 domInp.className = "cell cell_residue";
                 domInp.disabled = true;
                 domInp.value = cell.val;
                 break;
             case "share_data_non_checksum":
                 domInp.className = "cell cell_data";
                 domInp.value = cell.val || '';
                 break;
             case "share_data_checksum":
                 domInp.className = "cell cell_data";
                 domInp.value = cell.val || '';
                 break;
             case "residue":
                 domInp.className = "cell cell_residue";
                 domInp.value = cell.val || '';
             case "sum":
                 domInp.className = "cell cell_sum";
                 domInp.value = cell.val || '';
                 break;
             case "global_residue":
                 domInp.className = "cell cell_residue";
                 domInp.value = cell.val || '';
             }
             domInp.style.left = (g_cellparams.width * (x + row.start_idx) + (g_cellparams.spacer * ((x + row.start_idx) / 4 | 0))) + "px";
             domInp.style.top = (g_cellparams.height * y) + "px";
             domInp.maxWidth = 1;
             tab.appendChild(domInp);
             x += 1;
         }
         y += 1;
     }
}

