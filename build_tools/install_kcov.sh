#!/bin/bash
set -e

# travis has old kcov apparently, just build it

if [[ -z $TRAVIS ]]; then
    echo "You probably don't want to run this script when not on a travis ci box"
    exit 1
fi

wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz
tar xzf master.tar.gz
cd kcov-master
mkdir build
cd build
cmake ..
make
sudo make install
cd ../..
