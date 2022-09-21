
const {Session} = wasm_bindgen;
async function init_wasm() {
    await wasm_bindgen('../pkg/volvelle_wasm_bg.wasm');
}

// Global "session" state which is basically passed into Rust and back
// every time something changes.
let g_session;

// Construct a new session from the user's current choice of global parameters
async function new_session() {
    await init_wasm();

    g_session = new Session(
       document.getElementById("i_hrp").value,
       document.getElementById("i_k").value,
       document.getElementById("i_size").value,
       document.getElementById("i_checksum").value,
    );
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

     if (g_session.n_shares() == 0 || confirm("Updating global parameters will erase all shares! Are you sure?")) {
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
async function newInitialShare() {
    console.assert(g_session !== undefined);
    const idx = g_session.new_share();
    g_session.set_active_share(idx);
    createChecksumWorksheet(g_session.get_checksum_worksheet_cells());
    return;
}

// Construct a new checksum worksheet
function createChecksumWorksheet(cells) {
     // FIXME we aren't supposed to actually manipulate the DOM here
     document.getElementById("div_home").style.display = "none";
     let tab = document.getElementById('div_worksheet');
     tab.textContent = "";
console.log(cells);

     for (cell of cells) {
         let domInp = document.createElement("input");
         domInp.id = cell.dom_id;
         domInp.disabled = true; // only a couple cell types are editable
         domInp.value = cell.val || '';
         switch (cell.ty) {
         case "symbol":
             console.assert(cell.val !== undefined);
             domInp.className = "cell cell_symbol";
             break;
         case "fixed_hrp":
             console.assert(cell.val !== undefined);
             domInp.className = "cell cell_hrp";
             break;
         case "share_data":
             domInp.className = "cell cell_data";
             domInp.addEventListener("change", handleInputChange);
             domInp.disabled = false;
             break;
         case "share_data_checksum":
             domInp.className = "cell cell_data cell_pink";
             break;
         case "residue":
             domInp.className = "cell cell_residue";
         case "sum":
             domInp.className = "cell cell_sum";
             break;
         case "sum_checksum":
             domInp.className = "cell cell_sum cell_pink";
             break;
         case "global_residue":
             domInp.className = "cell cell_residue";
         }
         domInp.style.left = (g_cellparams.width * cell.x + (g_cellparams.spacer * (cell.x / 4 | 0))) + "px";
         domInp.style.top = (g_cellparams.height * cell.y) + "px";
         domInp.maxLength = 1;
         tab.appendChild(domInp);
     }
}

// Create a new, empty share, and switch to its checksum worksheet
async function handleInputChange(ev) {
    console.assert(g_session !== undefined);
    ev.target.style.color = "black"; // first undo any red coloring that may be left
    const actions = g_session.handle_input_change(ev.target.id, ev.target.value);
    await doDomActions(actions);
}

async function doDomActions(actions) {
    console.log(actions);
    for (action of actions) {
        let elem = document.getElementById(action.id);
        switch(action.ty) {
        case "flash_error":
            elem.style.color = "red";
            // setTimeout(function() { elem.style.color = "black"; }, 500); // actually don't change it back
            break;
        case "flash_set":
            elem.value = action.value;
            elem.style.color = "green";
            setTimeout(function() { elem.style.color = "black"; }, 500);
            break;
        case "set":
            elem.value = action.value;
            elem.style.color = "white";
            await sleep(50);
            elem.style.color = "black";
            setTimeout(function() { elem.style.color = "black"; }, 500);
            break;
        }
    }
}

