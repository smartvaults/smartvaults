from coinstr_sdk import Coinstr, Network, init_logger
import time

init_logger("/home/user/.coinstr", Network.TESTNET)

coinstr = Coinstr.open("/home/user/.coinstr", "test", "test", Network.TESTNET)

time.sleep(10.0)

# Drop client
del coinstr

time.sleep(30.0)