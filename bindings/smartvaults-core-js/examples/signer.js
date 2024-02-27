const { CoreSigner, Network, loadWasmAsync, SignerType, DescriptorPublicKey, ColdcardGenericJson, Purpose } = require("../");

async function main() {
    await loadWasmAsync();
    
    // Compose signer from coldcard export
    let json = JSON.stringify(require('./coldcard-export.json'));
    let coldcardJson = ColdcardGenericJson.fromJson(json);
    let signer = CoreSigner.fromColdcard(coldcardJson, Network.Testnet);
    console.log("Fingerprint: " + signer.fingerprint());
    signer.descriptors().forEach((s) => {
        console.log(s.descriptor.asString());
    })

    // Manualy compose signer
    let desc = "[87131a00/86'/1'/784923']tpubDDEaK5JwGiGDTRkML9YKh8AF4rHPhkpnXzVjVMDBtzayJpnsWKeiFPxtiyYeGHQj8pnjsei7N98winwZ3ivGoVVKArZVMsEYGig73XVqbSX/0/*";
    let customSigner = CoreSigner.empty("87131a00", SignerType.AirGap, Network.Testnet);
    customSigner.addDescriptor(Purpose.BIP86, DescriptorPublicKey.parse(desc));
}

main();