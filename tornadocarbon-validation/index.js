let wasmRegex;
async function initWasm() {
    wasmRegex = await import('./pkg');
}
initWasm();

//Gets around an issue with Parcel
window.findMatch = function findMatch() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;

    let match = wasmRegex.test(str, regExp);
    console.log(match);

    document.getElementById('output').innerText = `${str} ${match ? 'matches' : 'does not match'} ${regExp}`;
}