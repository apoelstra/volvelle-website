
const {Session} = wasm_bindgen;

/**
* Initialize WASM -- this must be called before any wasm funcitonality is available,
* but after the document is loaded, and must only be called once. (Curiously, calling
* it more than once actually crashes qutebrowser)
*
* This also calls `new_session` since I want to call this from onload as well. I'd
* like to have these two things be separated but it's not clear how, since it seems
* `wasm_bindgen` will only work in an async context and I need it to complete before
* I can call `new_session`, and the await keywoard doesn't work in onload="".
*/
async function body_onload() {
    // Init wasm
    await wasm_bindgen('../pkg/volvelle_wasm_bg.wasm');

    // Grab data from local storage, if any
    let data = null;
    if (g_has_local_storage) {
        data = localStorage.getItem("session");
    }

    // Create new session (will blank local storage, which is why we fetched any
    // existing data in the line above)
    await new_session();

    // Update session with localStorage data, if there was any.
    if (data !== null) {
        g_session.update_from_local_storage_str(data);
        cancelGlobalParams();
        localStorage.setItem("session", g_session.local_storage_str());
        for (idx = 0; idx < g_session.n_shares(); idx++) {
            createChecksumWorksheet(idx, g_session.get_checksum_worksheet_cells(idx));
        }
    }
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
* Determine whether or not local storage is available.
*
* It would be nice if we could just try to use local storage and soft-fail if it's
* not available, but some browsers actually throw exceptions and will interrupt
* your code, so we need to check that first.
*
* This basically came from https://gist.github.com/paulirish/5558557
*/
const g_has_local_storage = function() {
    try {
        const x = '__storage_test__';
        localStorage.setItem(x, x);
        localStorage.removeItem(x);
        return true;
    }
    catch (e) {
        return false; // we dgaf why, just say it's disabled if it doesn't work
    }
}();

/**
* Initialize the global session object
*
* This directly copies settings from the DOM and overwrites any existing session
* object, deleting all shares. It is called from onload and when the user hits
* the "Update" button, which will provide a warning if the operation would be
* destructive.
*/
async function new_session() {
    if (g_session !== undefined) {
        for (idx = 0; idx < g_session.n_shares(); idx++) {
            const del1 = document.getElementById("a_worksheet_" + idx);
            del1.parentNode.removeChild(del1);
            const del2 = document.getElementById("div_worksheet_" + idx);
            del2.parentNode.removeChild(del2);
        }
    }

    g_session = new Session(
       document.getElementById("i_hrp").value,
       document.getElementById("i_k").value,
       document.getElementById("i_size").value,
       document.getElementById("i_checksum").value,
    );

    if (g_has_local_storage) {
        localStorage.setItem("session", g_session.local_storage_str());
    }
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

     document.getElementById("bt_update").value = changed ? "Update & Clear Shares" : "Clear Shares";
     document.getElementById("bt_cancel").disabled = !changed;
}

/**
* Create a new session in response to the user clicking "Update"
*
* Will provide a warning prompt if this would destroy shares.
*/
async function updateGlobalParams() {
     console.assert(g_session !== undefined);

     if (g_session.n_shares() == 0 || confirm("This will erase all shares! Are you sure?")) {
         document.getElementById("bt_update").value = "Clear Shares";
         document.getElementById("bt_cancel").disabled = true;
         return new_session();
     }
}

/**
* The user clicked "Cancel"; replace the DOM global setting values with those from g_session
*/
async function cancelGlobalParams() {
     console.assert(g_session !== undefined);
     document.getElementById("i_hrp").value = g_session.hrp;
     document.getElementById("i_k").value = g_session.threshold;
     document.getElementById("i_size").value = g_session.size;
     document.getElementById("i_checksum").value = g_session.checksum;
     document.getElementById("bt_update").value = "Clear Shares";
     document.getElementById("bt_cancel").disabled = true;
}

/**
* The user clicked "new share"; create a new empty share and switch to its checksum worksheet
*/
async function newInitialShare() {
    console.assert(g_session !== undefined);
    const idx = g_session.new_share();
    createChecksumWorksheet(idx, g_session.get_checksum_worksheet_cells(idx));
    showDiv("div_worksheet_" + idx);
    if (g_has_local_storage) {
        localStorage.setItem("session", g_session.local_storage_str());
    }
    return;
}

/**
* Construct a new checksum worksheet
*
* This is a purely DOM-manipulating function. It should be called with a valid share
* index `idx` and the output from `g_session.get_checksum_worksheet(idx)` which
* provides a giant list of cells, their types, positions and values.
*/
function createChecksumWorksheet(idx, cells) {
    const table = document.createElement('div');
    table.id = "div_worksheet_" + idx;
    table.style.display = "none";

    const home_link = document.createElement("a");
    home_link.appendChild(document.createTextNode("Home"));
    home_link.href = "#";
    home_link.addEventListener("click", () => { showDiv("div_home"); });
    table.appendChild(home_link);

    const rand_link = document.createElement("a");
    rand_link.appendChild(document.createTextNode("Fill Randomly"));
    rand_link.href = "#";
    rand_link.addEventListener("click", () => { randomizeShare(idx); });
    rand_link.style.marginLeft = "2em";
    table.appendChild(rand_link);

    let max_y = 0;
    for (cell of cells) {
        const domInp = document.createElement("input");
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

    const share_link = document.createElement("a");
    const header_str = g_session.get_checksum_worksheet_header_str(idx);
    share_link.textContent = "Share " + header_str;
    share_link.href = "#";
    share_link.addEventListener("click", () => { showDiv("div_worksheet_" + idx); });
    share_link.id = "a_worksheet_" + idx;
    document.getElementById("div_sharelist").appendChild(share_link);
    document.getElementById("div_sharelist").appendChild(document.createElement("br"));
}

/**
* List of "actions" returned from `g_session` in response to some cell update.
*
* These actions are in the order that a paper user would (probably) fill in the
* cells, and should be replicated in the DOM with a tiny delay between each one
* to create an "animation" of the proper worksheet completion.
*
* We store these in a global array and pop them one by one from a `setInterval`
* function which disables itself when there are no more actions. When adding
* more actions you should call `processActions` to ensure that the loop is running.
*/
let g_worksheet_actions = [];

/**
* Respond to a user's edit of a cell in the checksum worksheet.
*
* We pass the change to wasm via `g_session.handle_input_change`, which returns
* a list of actions which we use to update the rest of the sheet.
*/
async function handleInputChange(ev) {
    console.assert(g_session !== undefined);
    ev.target.style.color = "black"; // first undo any red coloring that may be left

    // Update sheet and get list of consquent actions
    g_worksheet_actions = [
        ...g_worksheet_actions,
        ...g_session.handle_input_change(ev.target.id, ev.target.value),
    ];

    // Update local storage
    if (g_has_local_storage) {
        localStorage.setItem("session", g_session.local_storage_str());
    }

    // Update link text on home page
    const idx = g_session.get_idx_of(ev.target.id);
    const header_str = g_session.get_checksum_worksheet_header_str(idx);
    document.getElementById("a_worksheet_" + idx).textContent = "Share " + header_str;

    // Execute all the actions
    processActions();
}

/**
* Process the queue of actions.
*
* Deal with every queued-up action with a small delay in between.
*/
async function processActions() {
    let interval;
    interval = setInterval(() => {
        const action = g_worksheet_actions.shift();
        if (action === undefined) {
            clearInterval(interval);
            return;
        }

        const elem = document.getElementById(action.id);
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

/**
* Hide all the main <div>s except the given one, which we show.
*/
function showDiv(id) {
    document.getElementById("div_home").style.display = "none";
    for (i = 0; i < g_session.n_shares(); i++) {
        document.getElementById("div_worksheet_" + i).style.display = "none";
    }

    const to_show = document.getElementById(id);
    to_show.style.display = "block";
    // Do this in a settimeout to avoid angsty "forced reflow took XXms" warnings
    setTimeout(() => {  document.getElementById('div_content').style.height = (to_show.offsetHeight + 15) + "px"; }, 0);
}

/**
* Fills in a checksum worksheet with random data.
*/
function randomizeShare(idx) {
    const div_id = "div_worksheet_" + idx;
    const ALPHABET = "023456789ACDEFGHJKLMNPQRSTUVWXYZ";
    const rand = new Uint32Array(g_session.size);
    crypto.getRandomValues(rand);

    let i = 0;
    for (child of document.getElementById(div_id).childNodes) {
        if (child.disabled === false) {
            if (child.value == '') {
                child.value = ALPHABET[rand[i++] & 0x1f];
                g_worksheet_actions.push({
                    "ty": "flash_set",
                    "id": child.id,
                    "value": child.value,
                });
                child.dispatchEvent(new Event('change'));
            }
        }
    }
}

