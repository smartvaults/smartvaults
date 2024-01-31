const { Policy, Network, loadWasmAsync } = require("../");

async function main() {
    await loadWasmAsync();
    
    // Policy from miniscript
    let miniscript = "thresh(2,pk([87131a00/86'/1'/784923']tpubDDEaK5JwGiGDTRkML9YKh8AF4rHPhkpnXzVjVMDBtzayJpnsWKeiFPxtiyYeGHQj8pnjsei7N98winwZ3ivGoVVKArZVMsEYGig73XVqbSX/0/*),pk([e157a520/86'/1'/784923']tpubDCCYFYCyDkxo1xAzDpoFNdtGcjD5BPLZbEJswjJmwqp67Weqd2C7fg6Jy1SBjgn3wYnKyUtoYKXG4VdQczjqb6FJnqHe3NmFdgy8vNBSty4/0/*))";
    let policy = Policy.fromMiniscript(miniscript, Network.Testnet);
    console.log(policy.descriptor().asString())

    // Check if match any template
    let templateType = policy.templateMatch();
    console.log(templateType);
}

main();