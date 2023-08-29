<div align="center">
  <img src="./crates/coinstr/static/img/smartvaults.svg" width=200/>
  <h2>Bitcoin [taproot] multi-custody</h2>
  <p>
    <a href="https://github.com/coinstr/coinstr/blob/master/LICENSE"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"/></a>
    <a href="https://github.com/coinstr/coinstr/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/coinstr/coinstr/workflows/CI/badge.svg"></a>
  </p>
  <h4>
    <a href="https://coinstr.io">Website</a>
    <span> | </span>
    <a href="https://coinstr.app">Policy builder</a>
  </h4>
</div>

## About

₿ Coinstr is a bitcoin multi-custody protocol for spending policies and proposal execution
<br/>
🖆 Coinstr uses `nostr` for discovering signers, saving policies & PSBTs, and orchestrating signatures with workflow.
<br/>
👨‍👩‍👧‍👦 Coinstr eliminates friction for groups managing Bitcoin together. 

## Getting started

* [Download from releases](https://github.com/coinstr/coinstr/releases) (remember to run `chmod a+x coinstr*`)
  * [Verify the Release](doc/verify-release-binary.md)
* [Build from source](doc/build.md)  
* [Usage](doc/usage/README.md) 

## Project structure

The project is split up into several crates in the `crates/` directory:

### Executables

* [**coinstr**](./crates/coinstr/): Desktop application.
* [**coinstr-cli**](./crates/coinstr-cli): CLI appication.

### Libraries

* [**coinstr-core**](./crates/coinstr-core): Protocol primitives and bitcoin TX building/singning logic.
* [**coinstr-protocol**](./crates/coinstr-protocol): Implementation of the `Coinstr` protocol.
* [**coinstr-sdk**](./crates/coinstr-sdk): High level client library.

### Bindings

**coinstr-sdk** crate can be embedded inside other environments, like Swift, Kotlin, Python and JavaScript. 
Please, explore the [`bindings/`](./bindings/) directory to learn more.

#### Available packages

* **coinstr-sdk**:
    * Kotlin: [`io.coinstr:coinstr-sdk`](https://central.sonatype.com/artifact/io.coinstr/coinstr-sdk)
    * Swift: https://github.com/coinstr/coinstr-sdk-swift

## Architecture
![coinstr-arch](http://www.plantuml.com/plantuml/proxy?cache=no&src=https://raw.githubusercontent.com/coinstr/coinstr/master/doc/arch.iuml)

## State

⚠️ **This project is in an ALPHA state, use at YOUR OWN RISK and, possibly, with only testnet coins until release.** ⚠️

## License

This project is distributed under the MIT software license - see the [LICENSE](LICENSE) file for details
