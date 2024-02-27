#!/bin/sh

# BitBox02 udev rules

printf "SUBSYSTEM==\"usb\", TAG+=\"uaccess\", TAG+=\"udev-acl\", SYMLINK+=\"bitbox02_%%n\", ATTRS{idVendor}==\"03eb\", ATTRS{idProduct}==\"2403\"\n" > /etc/udev/rules.d/53-hid-bitbox02.rules
printf "KERNEL==\"hidraw*\", SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"03eb\", ATTRS{idProduct}==\"2403\", TAG+=\"uaccess\", TAG+=\"udev-acl\", SYMLINK+=\"bitbox02-%%n\"\n" > /etc/udev/rules.d/54-hid-bitbox02.rules

udevadm control --reload
udevadm trigger

echo "BitBox02 udev rules installed."