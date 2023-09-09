from coinstr_sdk import Coinstr, Network, init_desktop_logger
import time

init_desktop_logger("/home/user/.coinstr", Network.TESTNET)

coinstr = Coinstr.open("/home/user/.coinstr", "test", "test", Network.TESTNET)

time.sleep(10.0)

# Drop client
coinstr = None

time.sleep(30.0)