<div align="center">
  <img src="./crates/smartvaults-desktop/static/img/smartvaults.svg" width=200/>
  <h2>Bitcoin [taproot] multi-custody</h2>
  <p>
    <a href="https://github.com/smartvaults/smartvaults/blob/master/LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"/></a>
    <a href="https://github.com/smartvaults/smartvaults/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/smartvaults/smartvaults/workflows/CI/badge.svg"></a>
  </p>
  <h4>
    <a href="https://smartvaults.app">Web App</a>
    <span> | </span>
    <a href="https://docs.smartvaults.io">Docs</a>
  </h4>
</div>

## About

₿ Smart Vaults is a bitcoin multi-custody protocol for spending policies and proposal execution
<br/>
🖆 Smart Vaults uses `nostr` for discovering signers, saving policies & PSBTs, and orchestrating signatures with workflow.
<br/>
👨‍👩‍👧‍👦 Smart Vaults eliminates friction for groups managing Bitcoin together. 

## Getting started

* [Download from releases](https://github.com/smartvaults/smartvaults/releases) (remember to run `chmod a+x smartvaults-desktop*`)
  * [Verify the Release](doc/verify-release-binary.md)
* [Build from source](doc/build.md)  
* [Usage](doc/usage/README.md) 

## Project structure

The project is split up into several crates in the `crates/` directory:

### Executables

* [**smartvaults-desktop**](./crates/smartvaults-desktop/): Desktop application.
* [**smartvaults-cli**](./crates/smartvaults-cli): CLI appication.

### Libraries

* [**smartvaults-core**](./crates/smartvaults-core): Protocol primitives and bitcoin TX building/singning logic.
* [**smartvaults-protocol**](./crates/smartvaults-protocol): Implementation of the `Smart Vaults` protocol.
* [**smartvaults-sdk**](./crates/smartvaults-sdk): High level client library.

### Bindings

**smartvaults-sdk** crate can be embedded inside other environments, like Swift, Kotlin, Python and JavaScript. 
Please, explore the [`bindings/`](./bindings/) directory to learn more.

#### Available packages

* **smartvaults-sdk**:
    * Kotlin: [`io.smartvaults:smartvaults-sdk`](https://central.sonatype.com/artifact/io.smartvaults/smartvaults-sdk)
    * Swift: https://github.com/smartvaults/smartvaults-sdk-swift

## Architecture
![smartvaults-arch](http://www.plantuml.com/plantuml/proxy?cache=no&src=https://raw.githubusercontent.com/smartvaults/smartvaults/master/doc/arch.iuml)

## State

⚠️ **This project is in an ALPHA state, use at YOUR OWN RISK and, possibly, with only testnet coins until release.** ⚠️

## License

This project is distributed under the MIT software license - see the [LICENSE](LICENSE) file for details
