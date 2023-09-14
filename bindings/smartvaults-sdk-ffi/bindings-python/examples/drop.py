from smartvaults_sdk import SmartVaults, Network, init_desktop_logger
import time

init_desktop_logger("/home/user/.smartvaults", Network.TESTNET)

client = SmartVaults.open("/home/user/.smartvaults", "test", "test", Network.TESTNET)

time.sleep(10.0)

# Drop client
client = None

time.sleep(30.0)