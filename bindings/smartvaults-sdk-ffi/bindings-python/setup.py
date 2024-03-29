#!/usr/bin/env python

from setuptools import setup

from pathlib import Path
this_directory = Path(__file__).parent
long_description = (this_directory / "README.md").read_text()

setup(
    name='smartvaults-sdk',
    version='0.4.0',
    description="Smart Vaults SDK",
    long_description=long_description,
    long_description_content_type='text/markdown',
    include_package_data = True,
    zip_safe=False,
    packages=['smartvaults_sdk'],
    package_dir={'smartvaults_sdk': './src/smartvaults-sdk'},
    url="https://github.com/smartvaults/smartvaults",
    author="Yuki Kishimoto <yukikishimoto@protonmail.com>",
    license="MIT",
     # This is required to ensure the library name includes the python version, abi, and platform tags
    # See issue #350 for more information
    has_ext_modules=lambda: True,
)
