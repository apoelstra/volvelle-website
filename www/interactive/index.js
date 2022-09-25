
const {Session} = wasm_bindgen;

/**
* Initialize WASM -- this must be called before any wasm funcitonality is available,
* but after the document is loaded.
*
* We call it from `new_session` which is called from the body onload handler. I'm
* not sure how to call it directly and still have it run before `new_session`,
* because it seems to only work in an async context. But awaiting it from within
* `new_session` works.
*/
async function init_wasm() {
    await wasm_bindgen('../pkg/volvelle_wasm_bg.wasm');
}

/**
* Global session data which is used to access the state of the wasm code.
*
* In general, the way all this code works is by reacting to DOM events, calling
* methods on the global session object, then manipulating the DOM based on the
* return value of those methods.
*/
let g_session;

/**
* Initialize the global session object
*
* This directly copies settings from the DOM and overwrites any existing session
* object. It is called from onload and when the user hits the "Update" button,
* which will provide a warning if the operation would be destructive.
*/
async function new_session() {
    await init_wasm();

    if (g_session !== undefined) {
        for (idx = 0; idx < g_session.n_shares(); idx++) {
            let del = document.getElementById("a_worksheet_" + idx);
            del.parentNode.removeChild(del);
        }
    }

    g_session = new Session(
       document.getElementById("i_hrp").value,
       document.getElementById("i_k").value,
       document.getElementById("i_size").value,
       document.getElementById("i_checksum").value,
    );
}

/**
* Determine whether the user's choice of parameters in the web UI differs from g_session
*
* This is used to enable/disable the "Update"/"Cancel" buttons in the global
* settings dialog.
*/
function globalParamsChanged() {
     return (g_session.hrp != document.getElementById("i_hrp").value)
         || (g_session.checksum != document.getElementById("i_checksum").value)
         || (g_session.size != document.getElementById("i_size").value)
         || (g_session.threshold != document.getElementById("i_k").value);
}

/**
* Update the enabled/disabled state of the "Update" and "Cancel" buttons
*/
async function selectGlobalParams() {
     console.assert(g_session !== undefined);
     const changed = globalParamsChanged();

     document.getElementById("bt_update").disabled = !changed;
     document.getElementById("bt_cancel").disabled = !changed;
}

/**
* Create a new session in response to the user clicking "Update"
*
* Will provide a warning prompt if this would destroy shares.
*/
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
    createChecksumWorksheet(idx, g_session.get_checksum_worksheet_cells(idx));
    showDiv("div_worksheet_" + idx);
    return;
}

// Construct a new checksum worksheet
function createChecksumWorksheet(idx, cells) {
    let table = document.createElement('div');
    table.id = "div_worksheet_" + idx;
    table.style.display = "none";

    let home_link = document.createElement("a");
    home_link.appendChild(document.createTextNode("Home"));
    home_link.href = "#";
    home_link.addEventListener("click", () => { showDiv("div_home"); });
    table.appendChild(home_link);

    let rand_link = document.createElement("a");
    rand_link.appendChild(document.createTextNode("Fill Randomly"));
    rand_link.href = "#";
    rand_link.addEventListener("click", () => { randomizeShare(idx); });
    rand_link.style.marginLeft = "2em";
    table.appendChild(rand_link);

    let max_y = 0;
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
        domInp.style.position = "absolute";
        domInp.style.left = (15 + g_cellparams.width * cell.x + (g_cellparams.spacer * (cell.x / 4 | 0))) + "px";
        domInp.style.top = (45 + g_cellparams.height * cell.y) + "px";
        domInp.maxLength = 1;
        table.appendChild(domInp);

        if (max_y < cell.y) { max_y = cell.y; }
    }
    table.style.height = (g_cellparams.height * max_y + 60) + "px";
    document.getElementById('div_content').appendChild(table);

    let share_link = document.createElement("a");
    share_link.textContent = "Share ______";
    share_link.href = "#";
    share_link.addEventListener("click", () => { showDiv("div_worksheet_" + idx); });
    share_link.id = "a_worksheet_" + idx;
    document.getElementById("div_sharelist").appendChild(share_link);
    document.getElementById("div_sharelist").appendChild(document.createElement("br"));
}

// Process any actions needed to update the worksheet
let g_actions = [];
async function processActions() {
}

async function handleInputChange(ev) {
    console.assert(g_session !== undefined);
    ev.target.style.color = "black"; // first undo any red coloring that may be left

    // Update sheet and get list of consquent actions
    g_actions = [
        ...g_actions,
        ...g_session.handle_input_change(ev.target.id, ev.target.value),
    ];

    // Update link text on home page
    let idx = g_session.get_idx_of(ev.target.id);
    let header_str = g_session.get_checksum_worksheet_header_str(idx);
    document.getElementById("a_worksheet_" + idx).textContent = "Share " + header_str;

    // Execute all the actions
    processActions();
}

async function processActions() {
    let interval;
    interval = setInterval(() => {
        const action = g_actions.shift();
        if (action === undefined) {
            clearInterval(interval);
            return;
        }

        let elem = document.getElementById(action.id);
        switch(action.ty) {
        case "flash_error":
            elem.style.color = "red";
            break;
        case "flash_set":
            elem.value = action.value || '';
            elem.style.color = "green";
            setTimeout(() => { elem.style.color = "black"; }, 500);
            break;
        case "set":
            elem.value = action.value || '';
            elem.style.fontWeight = "bold";
            setTimeout(() => { elem.style.fontWeight = "normal"; }, 500);
            break;
        }
    }, 20);
}

function showDiv(id) {
    document.getElementById("div_home").style.display = "none";
    for (i = 0; i < g_session.n_shares(); i++) {
        document.getElementById("div_worksheet_" + i).style.display = "none";
    }

    let to_show = document.getElementById(id);
    to_show.style.display = "block";
    // Do this in a settimeout to avoid angsty "forced reflow took XXms" warnings
    setTimeout(() => {  document.getElementById('div_content').style.height = (to_show.offsetHeight + 15) + "px"; }, 0);
}

/// Fills in 
function randomizeShare(idx) {
    let div_id = "div_worksheet_" + idx;
    for (child of document.getElementById(div_id)) {
    }
}

