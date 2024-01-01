# Smart Vaults Core
	
## Description

JavaScript bindings of [smartvaults-cors](https://github.com/smartvaults/smartvaults) library.

This library **should** work on every JavaScript environment (nodejs, web, react native, ...).

## Getting started

```sh
npm i @smartvaults/core
```
    
```javascript
const { Keys, loadWasmAsync } = require("@smartvaults/core");

async function main() {
    // Load WASM 
    // if you are in a non async context, use loadWasmSync()
    await loadWasmAsync();
}

main();
```

More examples can be found in the [examples](https://github.com/smartvaults/smartvaults/tree/master/bindings/smartvaults-core-js/examples) directory.

## State

**This library is in an ALPHA state**, things that are implemented generally work but the API will change in breaking ways.

## License

This project is distributed under the MIT software license - see the [LICENSE](https://github.com/smartvaults/smartvaults/blob/master/LICENSE) file for details
